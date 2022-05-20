use crate::rw_semaphore::RwSemaphore as Semaphore;

use core::{
    cell::UnsafeCell,
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::NestStrategy as IN;

pub struct FutureRwLock<T: ?Sized, N: IN> {
    phantom: PhantomData<N>,
    lock: Semaphore<N>,
    data: UnsafeCell<T>,
}

/// A guard that provides immutable data access.
///
/// When the guard falls out of scope it will decrement the read count,
/// potentially releasing the lock.
pub struct FutureRwLockReadGuard<'a, T: 'a + ?Sized, N: IN> {
    phantom: PhantomData<N>,
    inner: &'a FutureRwLock<T, N>,
}

/// A guard that provides mutable data access.
///
/// When the guard falls out of scope it will release the lock.
pub struct FutureRwLockWriteGuard<'a, T: 'a + ?Sized, N: IN> {
    phantom: PhantomData<N>,
    inner: &'a FutureRwLock<T, N>,
    data: &'a mut T,
}

// Same unsafe impls as `std::sync::FutureRwLock`
unsafe impl<N: IN, T: ?Sized + Send> Send for FutureRwLock<T, N> {}
unsafe impl<N: IN, T: ?Sized + Send + Sync> Sync for FutureRwLock<T, N> {}

impl<T, N: IN> FutureRwLock<T, N> {
    #[inline]
    pub fn new(data: T) -> Self {
        FutureRwLock::<T, N> {
            phantom: PhantomData,
            lock: Semaphore::<N>::new(),
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

impl<T: ?Sized, N: IN> FutureRwLock<T, N> {
    pub async fn read(&self) -> FutureRwLockReadGuard<'_, T, N> {
        self.lock.acquire_read().await;
        FutureRwLockReadGuard {
            phantom: PhantomData,
            inner: self,
        }
    }

    pub async fn write(&self) -> FutureRwLockWriteGuard<'_, T, N> {
        self.lock.acquire_write().await;
        FutureRwLockWriteGuard {
            phantom: PhantomData,
            inner: self,
            data: unsafe { &mut *self.data.get() },
        }
    }

    #[inline]
    pub fn try_read(&self) -> Option<FutureRwLockReadGuard<T, N>> {
        if self.lock.try_acquire_read().is_ok() {
            Some(FutureRwLockReadGuard {
                phantom: PhantomData,
                inner: self,
            })
        } else {
            None
        }
    }

    pub fn spin_read(&self) -> FutureRwLockReadGuard<T, N> {
        loop {
            match self.try_read() {
                Some(guard) => return guard,
                None => core::hint::spin_loop(),
            }
        }
    }

    #[inline(always)]
    fn try_write(&self) -> Option<FutureRwLockWriteGuard<T, N>> {
        if self.lock.try_acquire_write().is_ok() {
            Some(FutureRwLockWriteGuard {
                phantom: PhantomData,
                inner: self,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            None
        }
    }

    pub fn spin_write(&self) -> FutureRwLockWriteGuard<T, N> {
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

impl<T: ?Sized + fmt::Debug, N: IN> fmt::Debug for FutureRwLock<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_read() {
            Some(guard) => write!(f, "FutureRwLock {{ data: ")
                .and_then(|()| (&*guard).fmt(f))
                .and_then(|()| write!(f, "}}")),
            None => write!(f, "FutureRwLock {{ <locked> }}"),
        }
    }
}

impl<T: ?Sized + Default, N: IN> Default for FutureRwLock<T, N> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T, N: IN> From<T> for FutureRwLock<T, N> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

impl<'rwlock, T: ?Sized, N: IN> FutureRwLockReadGuard<'rwlock, T, N> {
    #[inline]
    pub fn leak(this: Self) -> &'rwlock T {
        N::pop_off();
        let Self { phantom, inner } = this;
        unsafe { &*inner.data.get() }
    }
}

impl<'rwlock, T: ?Sized + fmt::Debug, N: IN> fmt::Debug for FutureRwLockReadGuard<'rwlock, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'rwlock, T: ?Sized + fmt::Display, N: IN> fmt::Display
    for FutureRwLockReadGuard<'rwlock, T, N>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<'rwlock, T: ?Sized, N: IN> FutureRwLockWriteGuard<'rwlock, T, N> {
    #[inline]
    pub fn leak(this: Self) -> &'rwlock mut T {
        N::pop_off();
        let data = this.data as *mut _; // Keep it in pointer form temporarily to avoid double-aliasing
        core::mem::forget(this);
        unsafe { &mut *data }
    }
}

impl<'rwlock, T: ?Sized + fmt::Debug, N: IN> fmt::Debug for FutureRwLockWriteGuard<'rwlock, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'rwlock, T: ?Sized + fmt::Display, N: IN> fmt::Display
    for FutureRwLockWriteGuard<'rwlock, T, N>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<'rwlock, T: ?Sized, N: IN> Deref for FutureRwLockReadGuard<'rwlock, T, N> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.inner.data.get() }
    }
}

impl<'rwlock, T: ?Sized, N: IN> Deref for FutureRwLockWriteGuard<'rwlock, T, N> {
    type Target = T;

    fn deref(&self) -> &T {
        self.data
    }
}

impl<'rwlock, T: ?Sized, N: IN> DerefMut for FutureRwLockWriteGuard<'rwlock, T, N> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<'rwlock, T: ?Sized, N: IN> Drop for FutureRwLockReadGuard<'rwlock, T, N> {
    fn drop(&mut self) {
        self.inner.lock.release_read();
    }
}

impl<'rwlock, T: ?Sized, N: IN> Drop for FutureRwLockWriteGuard<'rwlock, T, N> {
    fn drop(&mut self) {
        self.inner.lock.release_write();
    }
}
