use crate::rw_semaphore::RwSemaphore as Semaphore;

use core::{
    cell::UnsafeCell,
    default::Default,
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::NestStrategy as IN;

pub struct FutureMutex<T: ?Sized, N: IN> {
    phantom: PhantomData<N>,
    locked: Semaphore<N>,
    data: UnsafeCell<T>,
}

/// An RAII implementation of a “scoped lock” of a mutex.
/// When this structure is dropped (falls out of scope),
/// the lock will be unlocked.
///
pub struct FutureMutexGuard<'a, T: ?Sized, N: IN> {
    phantom: PhantomData<N>,
    lock: &'a FutureMutex<T, N>,
}

unsafe impl<N: IN, T: ?Sized + Send> Sync for FutureMutex<T, N> {}
unsafe impl<N: IN, T: ?Sized + Send> Send for FutureMutex<T, N> {}

impl<T, N: IN> FutureMutex<T, N> {
    #[inline(always)]
    pub fn new(data: T) -> Self {
        FutureMutex::<T, N> {
            phantom: PhantomData,
            locked: Semaphore::<N>::new(),
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

impl<T: ?Sized, N: IN> FutureMutex<T, N> {
    pub async fn lock(&self) -> FutureMutexGuard<'_, T, N> {
        self.locked.acquire_write().await;
        FutureMutexGuard {
            phantom: PhantomData,
            lock: self,
        }
    }

    pub fn try_lock(&self) -> Option<FutureMutexGuard<T, N>> {
        if self.locked.try_acquire_write().is_ok() {
            Some(FutureMutexGuard {
                phantom: PhantomData,
                lock: self,
            })
        } else {
            None
        }
    }

    pub fn spin_lock(&self) -> FutureMutexGuard<T, N> {
        loop {
            match self.try_lock() {
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

    #[inline(always)]
    pub fn is_locked(&self) -> bool {
        self.locked.get_permit() != 0
    }
}

impl<T: ?Sized + fmt::Debug, N: IN> fmt::Debug for FutureMutex<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_lock() {
            Some(guard) => write!(f, "Mutex {{ data: ")
                .and_then(|()| (&*guard).fmt(f))
                .and_then(|()| write!(f, "}}")),
            None => write!(f, "Mutex {{ <locked> }}"),
        }
    }
}

impl<T: ?Sized + Default, N: IN> Default for FutureMutex<T, N> {
    fn default() -> Self {
        FutureMutex::new(T::default())
    }
}

impl<T, N: IN> From<T> for FutureMutex<T, N> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

impl<'a, T: ?Sized, N: IN> Drop for FutureMutexGuard<'a, T, N> {
    /// The dropping of the FutureMutexGuard will release the lock it was created from.
    fn drop(&mut self) {
        self.lock.locked.release_write();
    }
}

impl<'a, T: ?Sized, N: IN> Deref for FutureMutexGuard<'a, T, N> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T: ?Sized, N: IN> DerefMut for FutureMutexGuard<'a, T, N> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T: ?Sized + fmt::Debug, N: IN> fmt::Debug for FutureMutexGuard<'a, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized + fmt::Display, N: IN> fmt::Display for FutureMutexGuard<'a, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}
