use core::{
    cell::UnsafeCell,
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::NestStrategy as IN;

#[repr(usize)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum LockChannel {
    Normal = 0,
    Interrupt = 1,
}

pub struct MCSLock<T: ?Sized, N: IN> {
    phantom: PhantomData<N>,
    pub(crate) locked: [AtomicBool; 2],
    data: UnsafeCell<T>,
}

pub struct MCSLockGuard<'a, T: ?Sized, N: IN> {
    phantom: PhantomData<N>,
    mcslock: &'a MCSLock<T, N>,
    data: &'a mut T,
    channel: LockChannel,
}

unsafe impl<N: IN, T: ?Sized + Send> Sync for MCSLock<T, N> {}
unsafe impl<N: IN, T: ?Sized + Send> Send for MCSLock<T, N> {}

impl<T, N: IN> MCSLock<T, N> {
    #[inline(always)]
    pub const fn new(data: T) -> Self {
        MCSLock {
            phantom: PhantomData,
            locked: [AtomicBool::new(false), AtomicBool::new(false)], // TODO: remove hardcode
            data: UnsafeCell::new(data),
        }
    }

    #[inline(always)]
    pub fn into_inner(self) -> T {
        // We know statically that there are no outstanding references to
        // `self` so there's no need to lock.
        let MCSLock { data, .. } = self;
        data.into_inner()
    }

    #[inline(always)]
    pub fn as_mut_ptr(&self) -> *mut T {
        self.data.get()
    }
}

impl<T: ?Sized, N: IN> MCSLock<T, N> {
    #[inline(always)]
    pub fn lock(&self, channel: LockChannel) -> MCSLockGuard<T, N> {
        while self.locked[channel as usize]
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Wait until the lock looks unlocked before retrying
            while self.is_locked(channel) {
                core::hint::spin_loop();
            }
        }

        MCSLockGuard {
            phantom: PhantomData,
            mcslock: self,
            data: unsafe { &mut *self.data.get() },
            channel,
        }
    }

    #[inline(always)]
    pub fn try_lock(&self, channel: LockChannel) -> Option<MCSLockGuard<T, N>> {
        if self.locked[channel as usize]
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(MCSLockGuard {
                phantom: PhantomData,
                mcslock: self,
                data: unsafe { &mut *self.data.get() },
                channel,
            })
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut T {
        // We know statically that there are no other references to `self`, so
        // there's no need to lock the inner mutex.
        unsafe { &mut *self.data.get() }
    }

    #[inline(always)]
    pub fn is_locked(&self, channel: LockChannel) -> bool {
        self.locked[channel as usize].load(Ordering::Relaxed)
    }
}

impl<'a, T: ?Sized + fmt::Display, N: IN> fmt::Display for MCSLockGuard<'a, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized, N: IN> Deref for MCSLockGuard<'a, T, N> {
    type Target = T;
    fn deref(&self) -> &T {
        self.data
    }
}

impl<'a, T: ?Sized, N: IN> DerefMut for MCSLockGuard<'a, T, N> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<'a, T: ?Sized, N: IN> Drop for MCSLockGuard<'a, T, N> {
    /// The dropping of the MutexGuard will release the lock it was created from.
    fn drop(&mut self) {
        self.mcslock.locked[self.channel as usize].store(false, Ordering::Release);
    }
}

impl<T: ?Sized, N: IN> fmt::Display for MCSLock<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MCSLock{{locked=[N = {}, I = {}]}}",
            self.locked[LockChannel::Normal as usize].load(Ordering::Relaxed),
            self.locked[LockChannel::Interrupt as usize].load(Ordering::Relaxed),
        )
    }
}
