use crate::rw_semaphore::RwSemaphore as Semaphore;

use core::{
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut},
};

use crate::mcslock::LockChannel;

pub struct FutureMCSLock<T: ?Sized> {
    pub(crate) lock: [Semaphore; 2],
    data: UnsafeCell<T>,
}

pub struct FutureMCSLockGuard<'a, T: ?Sized + 'a> {
    inner: &'a FutureMCSLock<T>,
    channel: LockChannel,
}

unsafe impl<T: ?Sized + Send> Sync for FutureMCSLock<T> {}
unsafe impl<T: ?Sized + Send> Send for FutureMCSLock<T> {}

impl<T> FutureMCSLock<T> {
    #[inline(always)]
    pub fn new(data: T) -> Self {
        FutureMCSLock {
            lock: [Semaphore::new(), Semaphore::new()], // TODO: remove hardcode
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

impl<T: ?Sized> FutureMCSLock<T> {
    pub async fn lock(&self, channel: LockChannel) -> FutureMCSLockGuard<'_, T> {
        self.lock[channel as usize].acquire_write().await;
        FutureMCSLockGuard {
            inner: self,
            channel,
        }
    }

    #[inline(always)]
    pub fn try_lock(&self, channel: LockChannel) -> Option<FutureMCSLockGuard<T>> {
        if self.lock[channel as usize].try_acquire_write().is_ok()
        {
            Some(FutureMCSLockGuard {
                inner: self,
                channel,
            })
        } else {
            None
        }
    }

    pub fn spin_lock(&self, channel: LockChannel) -> FutureMCSLockGuard<T> {
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

impl<T: ?Sized + Default> Default for FutureMCSLock<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> From<T> for FutureMCSLock<T> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}


impl<'a, T: ?Sized + fmt::Display> fmt::Display for FutureMCSLockGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized> Deref for FutureMCSLockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.inner.data.get() }
    }
}

impl<'a, T: ?Sized> DerefMut for FutureMCSLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.inner.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for FutureMCSLockGuard<'a, T> {
    /// The dropping of the MutexGuard will release the lock it was created from.
    fn drop(&mut self) {
        self.inner.lock[self.channel as usize].release_write();
    }
}

impl<T: ?Sized> fmt::Display for FutureMCSLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FutureMCSLock{{lock=[N = {}, I = {}]}}",
            self.lock[LockChannel::Normal as usize].try_acquire_write().is_err(),
            self.lock[LockChannel::Interrupt as usize].try_acquire_write().is_err(),
        )
    }
}
