use crate::rw_semaphore::RwSemaphore as Semaphore;

use core::{
    cell::UnsafeCell,
    default::Default,
    fmt,
    ops::{Deref, DerefMut},
};

pub struct FutureMutex<T: ?Sized> {
    locked: Semaphore,
    data: UnsafeCell<T>,
}

/// An RAII implementation of a “scoped lock” of a mutex.
/// When this structure is dropped (falls out of scope),
/// the lock will be unlocked.
///
pub struct FutureMutexGuard<'a, T: ?Sized + 'a> {
    lock: &'a FutureMutex<T>,
}

unsafe impl<T: ?Sized + Send> Sync for FutureMutex<T> {}
unsafe impl<T: ?Sized + Send> Send for FutureMutex<T> {}

impl<T> FutureMutex<T> {
    #[inline(always)]
    pub fn new(data: T) -> Self {
        FutureMutex {
            locked: Semaphore::new(),
            data: UnsafeCell::new(data),
        }
    }

    #[inline(always)]
    pub fn into_inner(self) -> T {
        // We know statically that there are no outstanding references to
        // `self` so there's no need to lock.
        self.data.into_inner()
    }

    #[inline(always)]
    pub fn as_mut_ptr(&self) -> *mut T {
        self.data.get()
    }
}

impl<T: ?Sized> FutureMutex<T> {
    pub async fn lock(&self) -> FutureMutexGuard<'_, T> {
        self.locked.acquire_write().await;
        FutureMutexGuard { lock: self }
    }

    pub fn try_lock(&self) -> Option<FutureMutexGuard<T>> {
        if self.locked.try_acquire_write().is_ok() {
            Some(FutureMutexGuard { lock: self })
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
    pub fn is_locked(&self) -> bool {
        self.locked.get_permit() != 0
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for FutureMutex<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_lock() {
            Some(guard) => write!(f, "Mutex {{ data: ")
                .and_then(|()| (&*guard).fmt(f))
                .and_then(|()| write!(f, "}}")),
            None => write!(f, "Mutex {{ <locked> }}"),
        }
    }
}

impl<T: ?Sized + Default> Default for FutureMutex<T> {
    fn default() -> Self {
        FutureMutex::new(T::default())
    }
}

impl<T> From<T> for FutureMutex<T> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

impl<'a, T: ?Sized> Drop for FutureMutexGuard<'a, T> {
    /// The dropping of the FutureMutexGuard will release the lock it was created from.
    fn drop(&mut self) {
        self.lock.locked.release_write();
    }
}

impl<'a, T: ?Sized> Deref for FutureMutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> DerefMut for FutureMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for FutureMutexGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized + fmt::Display> fmt::Display for FutureMutexGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}
