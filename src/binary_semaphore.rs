use super::Mutex;

use alloc::{collections::VecDeque, sync::Arc};
use core::sync::atomic::{AtomicBool, Ordering};
use core::{
    future::Future,
    pin::Pin,
    result::Result,
    task::{Context, Poll, Waker},
};
type AcquireResult = Result<(), ()>;

pub(crate) struct Semaphore {
    permit: AtomicBool,
    waiters: Mutex<VecDeque<Arc<Waiter>>>,
    _closed: bool,
}

impl Semaphore {
    pub fn new() -> Self {
        Self {
            permit: AtomicBool::new(true),
            waiters: Mutex::new(VecDeque::new()),
            _closed: false,
        }
    }

    pub fn acquire(&self) -> AcquireFuture<'_> {
        AcquireFuture {
            semaphore: self,
            node: Arc::new(Waiter::new()),
        }
    }

    pub fn try_acquire(&self) -> AcquireResult {
        if self
            .permit
            .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Ok(())
        } else {
            Err(())
        }
    }

    fn poll_acquire(&self, node: &Arc<Waiter>) -> AcquireResult {
        let mut waiters = self.waiters.lock();
        if self
            .permit
            .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Ok(())
        } else {
            if node
                .queued
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                waiters.push_back(node.clone());
            }
            Err(())
        }
    }

    pub fn release(&self) {
        let mut waiters = self.waiters.lock();
        self.permit.store(true, Ordering::Release);
        while !waiters.is_empty() {
            let waiter = waiters.pop_front().unwrap();
            if let Some(waker) = &waiter.waker {
                waker.wake_by_ref();
                break;
            }
        }
    }

    pub fn get_permit(&self) -> bool {
        self.permit.load(Ordering::Relaxed)
    }
}

pub(crate) struct AcquireFuture<'a> {
    semaphore: &'a Semaphore,
    node: Arc<Waiter>,
}

impl Future for AcquireFuture<'_> {
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
    waker: Option<Waker>,
    queued: AtomicBool,
}

impl Waiter {
    const fn new() -> Self {
        Self {
            waker: None,
            queued: AtomicBool::new(false),
        }
    }
}
