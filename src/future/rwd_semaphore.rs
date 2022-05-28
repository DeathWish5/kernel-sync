use crate::spinlock::{Mutex, MutexGuard};

use alloc::{collections::VecDeque, sync::Arc};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    result::Result,
    task::{Context, Poll, Waker},
};

use crate::NestStrategy as IN;

pub const READER: usize = 1 << 2;
pub const WRITER: usize = 1 << 1;
pub const DISK: usize = 1;

type AcquireResult = Result<(), usize>;

pub struct RwdSemaphore<N: IN> {
    phantom: PhantomData<N>,
    permit: AtomicUsize,
    waiters: Mutex<VecDeque<Arc<Waiter>>, N>,
    _closed: bool,
}

impl<N: IN> RwdSemaphore<N> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
            permit: AtomicUsize::new(0),
            waiters: Mutex::<VecDeque<Arc<Waiter>>, N>::new(VecDeque::new()),
            _closed: false,
        }
    }

    pub fn acquire_read(&self) -> AcquireFuture<'_, N> {
        AcquireFuture {
            semaphore: self,
            node: Arc::new(Waiter::new(AcquireType::Read)),
        }
    }

    pub fn acquire_write(&self) -> AcquireFuture<'_, N> {
        AcquireFuture {
            semaphore: self,
            node: Arc::new(Waiter::new(AcquireType::Write)),
        }
    }

    pub fn acquire_disk(&self) -> AcquireFuture<'_, N> {
        AcquireFuture {
            semaphore: self,
            node: Arc::new(Waiter::new(AcquireType::Disk)),
        }
    }

    pub fn try_acquire_read(&self) -> AcquireResult {
        N::push_off();
        let value = self.permit.fetch_add(READER, Ordering::Acquire);
        if (value & (DISK | WRITER)) != 0 {
            self.permit.fetch_sub(READER, Ordering::Release);
            N::pop_off();
            Err(value)
        } else {
            Ok(())
        }
    }

    pub fn try_acquire_write(&self) -> AcquireResult {
        N::push_off();
        let value = self
            .permit
            .compare_exchange(0, WRITER, Ordering::Acquire, Ordering::Relaxed);
        match value {
            Ok(_) => Ok(()),
            Err(err) => {
                N::pop_off();
                Err(err)
            }
        }
    }

    pub fn try_acquire_disk(&self) -> AcquireResult {
        N::push_off();
        let value = self
            .permit
            .compare_exchange(0, DISK, Ordering::Acquire, Ordering::Relaxed);
        match value {
            Ok(_) => Ok(()),
            Err(err) => {
                N::pop_off();
                Err(err)
            }
        }
    }

    pub fn try_downgrade_read(&self, old: usize) -> AcquireResult {
        debug_assert!(old == WRITER || old == DISK);
        let value = self
            .permit
            .compare_exchange(old, READER, Ordering::Acquire, Ordering::Relaxed);
        match value {
            Ok(_) => {
                let mut waiters = self.waiters.lock();
                Self::wake_reader(&mut waiters);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    pub fn try_downgrade_write(&self, old: usize) -> AcquireResult {
        debug_assert!(old == DISK);
        let value = self
            .permit
            .compare_exchange(old, WRITER, Ordering::Acquire, Ordering::Relaxed);
        value.map(|_| ())
    }

    pub fn try_upgrade_disk(&self, old: usize) -> AcquireResult {
        debug_assert!(old == WRITER || old == READER);
        let value = self
            .permit
            .compare_exchange(old, DISK, Ordering::Acquire, Ordering::Relaxed);
        value.map(|_| ())
    }

    pub fn try_upgrade_write(&self, old: usize) -> AcquireResult {
        debug_assert!(old == READER);
        let value = self
            .permit
            .compare_exchange(old, WRITER, Ordering::Acquire, Ordering::Relaxed);
        value.map(|_| ())
    }

    fn read_upgrade(&self, new: usize) -> AcquireResult {
        let value = self
            .permit
            .fetch_update(Ordering::Acquire, Ordering::Relaxed, |value| {
                if value & (WRITER | DISK) == 0 {
                    Some(value | new)
                } else {
                    None
                }
            });
        match value {
            Ok(_) => loop {
                let value = self.permit.compare_exchange(
                    READER | new,
                    new,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                );
                match value {
                    Ok(_) => break Ok(()),
                    Err(_) => core::hint::spin_loop(),
                }
            },
            Err(err) => Err(err),
        }
    }

    pub fn read_upgrade_write(&self, old: usize) -> AcquireResult {
        debug_assert!(old == READER);
        self.read_upgrade(WRITER)
    }

    pub fn read_upgrade_disk(&self, old: usize) -> AcquireResult {
        debug_assert!(old == READER);
        self.read_upgrade(DISK)
    }

    fn poll_acquire(&self, node: &Arc<Waiter>) -> AcquireResult {
        let mut waiters = self.waiters.lock();
        let req = node.req;
        let res = loop {
            let res = match req {
                AcquireType::Read => self.try_acquire_read(),
                AcquireType::Write => self.try_acquire_write(),
                _ => self.try_acquire_disk(),
            };
            if res.is_ok() || Err(DISK) == res {
                break res;
            }
            core::hint::spin_loop();
        };
        if res.is_err()
            && node
                .queued
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
        {
            waiters.push_back(node.clone());
        }
        res
    }

    pub fn release_read(&self) {
        let mut waiters = self.waiters.lock();
        let old = self.permit.fetch_sub(READER, Ordering::Release);
        if old == READER {
            Self::wake_next(&mut waiters);
        }
        N::pop_off();
    }

    pub fn release_write(&self) {
        let mut waiters = self.waiters.lock();
        self.permit.fetch_and(!WRITER, Ordering::Release);
        Self::wake_next(&mut waiters);
        N::pop_off();
    }

    pub fn release_disk(&self) {
        let mut waiters = self.waiters.lock();
        self.permit.fetch_and(!DISK, Ordering::Release);
        Self::wake_next(&mut waiters);
        N::pop_off();
    }

    fn wake_next(waiters: &mut MutexGuard<VecDeque<Arc<Waiter>>, N>) {
        if !waiters.is_empty() {
            let waiter = waiters.pop_front().unwrap();
            waiter.wake();
            if waiter.req == AcquireType::Read {
                waiters.retain(|waiter| {
                    if waiter.req == AcquireType::Read {
                        waiter.wake();
                        false
                    } else {
                        true
                    }
                });
            }
        }
    }

    fn wake_reader(waiters: &mut MutexGuard<VecDeque<Arc<Waiter>>, N>) {
        waiters.retain(|waiter| {
            if waiter.req == AcquireType::Read {
                waiter.wake();
                false
            } else {
                true
            }
        });
    }

    pub fn reader_count(&self) -> usize {
        let state = self.permit.load(Ordering::Relaxed);
        state / READER
    }

    pub fn writer_count(&self) -> usize {
        (self.permit.load(Ordering::Relaxed) & WRITER) / WRITER
    }

    pub fn disk_count(&self) -> usize {
        (self.permit.load(Ordering::Relaxed) & DISK) / DISK
    }

    pub fn get_permit(&self) -> usize {
        self.permit.load(Ordering::Relaxed)
    }
}

impl<N: IN> Default for RwdSemaphore<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AcquireType {
    Read = 0,
    Write = 1,
    Disk = 2,
}

pub struct AcquireFuture<'a, N: IN> {
    semaphore: &'a RwdSemaphore<N>,
    node: Arc<Waiter>,
}

impl<N: IN> Future for AcquireFuture<'_, N> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.node.waker.is_none() {
            let waiter = unsafe { Arc::<Waiter>::get_mut_unchecked(&mut self.node) };
            waiter.waker = Some(cx.waker().clone());
        }
        assert!(cx.waker().will_wake(self.node.waker.as_ref().unwrap()));
        match self.semaphore.poll_acquire(&self.node) {
            Ok(_) => Poll::Ready(()),
            Err(_) => Poll::Pending,
        }
    }
}

pub struct Waiter {
    req: AcquireType,
    waker: Option<Waker>,
    queued: AtomicBool,
}

impl Waiter {
    const fn new(req: AcquireType) -> Self {
        Self {
            req,
            waker: None,
            queued: AtomicBool::new(false),
        }
    }

    pub fn wake(&self) {
        if let Some(waker) = &self.waker {
            waker.wake_by_ref();
        } else {
            panic!("waiter with None `waker` was enqueued");
        }
    }
}
