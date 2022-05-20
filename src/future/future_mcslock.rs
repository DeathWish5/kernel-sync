use crate::rw_semaphore::RwSemaphore as Semaphore;

use core::{
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut},
};

use crate::mcslock::LockChannel;
use crate::NestStrategy as IN;

pub struct FutureMCSLock<T: ?Sized, N: IN> {
    pub(crate) lock: [Semaphore<N>; 2],
    data: UnsafeCell<T>,
}

pub struct FutureMCSLockGuard<'a, T: ?Sized, N: IN> {
    inner: &'a FutureMCSLock<T, N>,
    channel: LockChannel,
}

unsafe impl<N: IN, T: ?Sized + Send> Sync for FutureMCSLock<T, N> {}
unsafe impl<N: IN, T: ?Sized + Send> Send for FutureMCSLock<T, N> {}

impl<T, N: IN> FutureMCSLock<T, N> {
    #[inline(always)]
    pub fn new(data: T) -> Self {
        FutureMCSLock::<T, N> {
            lock: [Semaphore::<N>::new(), Semaphore::<N>::new()], // TODO: remove hardcode
            data: UnsafeCell::new(data),
        }
    }

    #[inline(always)]
    pub fn into_inner(self) -> T {
        // We know statically that there are no outstanding references to
        // `self` so there's no need to lock.
        let FutureMCSLock { data, .. } = self;
        data.into_inner()
    }

    #[inline(always)]
    pub fn as_mut_ptr(&self) -> *mut T {
        self.data.get()
    }
}

impl<T: ?Sized, N: IN> FutureMCSLock<T, N> {
    pub async fn lock(&self, channel: LockChannel) -> FutureMCSLockGuard<'_, T, N> {
        self.lock[channel as usize].acquire_write().await;
        FutureMCSLockGuard {
            inner: self,
            channel,
        }
    }

    #[inline(always)]
    pub fn try_lock(&self, channel: LockChannel) -> Option<FutureMCSLockGuard<T, N>> {
        if self.lock[channel as usize].try_acquire_write().is_ok() {
            Some(FutureMCSLockGuard {
                inner: self,
                channel,
            })
        } else {
            None
        }
    }

    pub fn spin_lock(&self, channel: LockChannel) -> FutureMCSLockGuard<T, N> {
        loop {
            match self.try_lock(channel) {
                Some(guard) => return guard,
                None => core::hint::spin_loop(),
            }
        }
    }

    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut T {
        // We know statically that there are no other references to `self`, so
        // there's no need to lock the inner mutex.
        unsafe { &mut *self.data.get() }
    }
}

impl<T: ?Sized + Default, N: IN> Default for FutureMCSLock<T, N> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T, N: IN> From<T> for FutureMCSLock<T, N> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

impl<'a, T: ?Sized + fmt::Display, N: IN> fmt::Display for FutureMCSLockGuard<'a, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized, N: IN> Deref for FutureMCSLockGuard<'a, T, N> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.inner.data.get() }
    }
}

impl<'a, T: ?Sized, N: IN> DerefMut for FutureMCSLockGuard<'a, T, N> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.inner.data.get() }
    }
}

impl<'a, T: ?Sized, N: IN> Drop for FutureMCSLockGuard<'a, T, N> {
    /// The dropping of the MutexGuard will release the lock it was created from.
    fn drop(&mut self) {
        self.inner.lock[self.channel as usize].release_write();
    }
}

impl<T: ?Sized, N: IN> fmt::Display for FutureMCSLock<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FutureMCSLock{{lock=[N = {}, I = {}]}}",
            self.lock[LockChannel::Normal as usize]
                .try_acquire_write()
                .is_err(),
            self.lock[LockChannel::Interrupt as usize]
                .try_acquire_write()
                .is_err(),
        )
    }
}
