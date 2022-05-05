use crate::rw_semaphore::RwSemaphore as Semaphore;

use core::{
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut},
};

use super::{pop_off};

pub struct FutureRwLock<T: ?Sized> {
    lock: Semaphore,
    data: UnsafeCell<T>,
}

/// A guard that provides immutable data access.
///
/// When the guard falls out of scope it will decrement the read count,
/// potentially releasing the lock.
pub struct FutureRwLockReadGuard<'a, T: 'a + ?Sized> {
    inner: &'a FutureRwLock<T>,
}

/// A guard that provides mutable data access.
///
/// When the guard falls out of scope it will release the lock.
pub struct FutureRwLockWriteGuard<'a, T: 'a + ?Sized> {
    inner: &'a FutureRwLock<T>,
    data: &'a mut T,
}

// Same unsafe impls as `std::sync::FutureRwLock`
unsafe impl<T: ?Sized + Send> Send for FutureRwLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for FutureRwLock<T> {}

impl<T> FutureRwLock<T> {
    #[inline]
    pub fn new(data: T) -> Self {
        FutureRwLock {
            lock: Semaphore::new(),
            data: UnsafeCell::new(data),
        }
    }

    /// Consumes this `FutureRwLock`eturning the underlying data.
    #[inline]
    pub fn into_inner(self) -> T {
        // We know statically that there are no outstanding references to
        // `self` so there's no need to lock.
        let FutureRwLock { data, .. } = self;
        data.into_inner()
    }

    #[inline(always)]
    pub fn as_mut_ptr(&self) -> *mut T {
        self.data.get()
    }
}

impl<T: ?Sized> FutureRwLock<T> {
    pub async fn read(&self) -> FutureRwLockReadGuard<'_, T> {
        self.lock.acquire_read().await;
        FutureRwLockReadGuard { 
            inner: self,
        }
    }

    pub async fn write(&self) -> FutureRwLockWriteGuard<'_, T> {
        self.lock.acquire_write().await;
        FutureRwLockWriteGuard { 
            inner: self,
            data: unsafe { &mut *self.data.get() },
        }
    }

    #[inline]
    pub fn try_read(&self) -> Option<FutureRwLockReadGuard<T>> {
        if self.lock.try_acquire_read().is_ok() {
            Some(FutureRwLockReadGuard {
                inner: self,
            })
        } else {
            None
        }
    }

    pub fn spin_read(&self) -> FutureRwLockReadGuard<T> {
        loop {
            match self.try_read() {
                Some(guard) => return guard,
                None => core::hint::spin_loop(),
            }
        }
    }

    #[inline(always)]
    fn try_write(&self) -> Option<FutureRwLockWriteGuard<T>> {
        if self.lock.try_acquire_write().is_ok() {
            Some(FutureRwLockWriteGuard {
                inner: self,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            None
        }
    }

    pub fn spin_write(&self) -> FutureRwLockWriteGuard<T> {
        loop {
            match self.try_write() {
                Some(guard) => return guard,
                None => core::hint::spin_loop(),
            }
        }
    }

    pub fn reader_count(&self) -> usize {
        self.lock.reader_count()
    }

    pub fn writer_count(&self) -> usize {
        self.lock.writer_count()
    }

    pub fn get_mut(&mut self) -> &mut T {
        // We know statically that there are no other references to `self`, so
        // there's no need to lock the inner lock.
        unsafe { &mut *self.data.get() }
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for FutureRwLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_read() {
            Some(guard) => write!(f, "FutureRwLock {{ data: ")
                .and_then(|()| (&*guard).fmt(f))
                .and_then(|()| write!(f, "}}")),
            None => write!(f, "FutureRwLock {{ <locked> }}"),
        }
    }
}

impl<T: ?Sized + Default> Default for FutureRwLock<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T> From<T> for FutureRwLock<T> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

impl<'rwlock, T: ?Sized> FutureRwLockReadGuard<'rwlock, T> {
    #[inline]
    pub fn leak(this: Self) -> &'rwlock T {
        pop_off();
        let Self { inner } = this;
        unsafe { &*inner.data.get() }
    }
}

impl<'rwlock, T: ?Sized + fmt::Debug> fmt::Debug for FutureRwLockReadGuard<'rwlock, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'rwlock, T: ?Sized + fmt::Display> fmt::Display for FutureRwLockReadGuard<'rwlock, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<'rwlock, T: ?Sized> FutureRwLockWriteGuard<'rwlock, T> {
    #[inline]
    pub fn leak(this: Self) -> &'rwlock mut T {
        pop_off();
        let data = this.data as *mut _; // Keep it in pointer form temporarily to avoid double-aliasing
        core::mem::forget(this);
        unsafe { &mut *data }
    }
}

impl<'rwlock, T: ?Sized + fmt::Debug> fmt::Debug for FutureRwLockWriteGuard<'rwlock, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'rwlock, T: ?Sized + fmt::Display> fmt::Display for FutureRwLockWriteGuard<'rwlock, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<'rwlock, T: ?Sized> Deref for FutureRwLockReadGuard<'rwlock, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.inner.data.get() }
    }
}

impl<'rwlock, T: ?Sized> Deref for FutureRwLockWriteGuard<'rwlock, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.data
    }
}

impl<'rwlock, T: ?Sized> DerefMut for FutureRwLockWriteGuard<'rwlock, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<'rwlock, T: ?Sized> Drop for FutureRwLockReadGuard<'rwlock, T> {
    fn drop(&mut self) {
        self.inner.lock.release_read();
    }
}

impl<'rwlock, T: ?Sized> Drop for FutureRwLockWriteGuard<'rwlock, T> {
    fn drop(&mut self) {
        self.inner.lock.release_write();
    }
}