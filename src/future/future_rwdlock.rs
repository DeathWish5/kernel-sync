use crate::rwd_semaphore::{RwdSemaphore as Semaphore, DISK, READER, WRITER};

use core::{
    cell::UnsafeCell,
    fmt,
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
};

use crate::NestStrategy as IN;

pub struct FutureRwdLock<T: ?Sized, N: IN> {
    phantom: PhantomData<N>,
    lock: Semaphore<N>,
    data: UnsafeCell<T>,
}

/// A guard that provides immutable data access.
///
/// When the guard falls out of scope it will decrement the read count,
/// potentially releasing the lock.
pub struct FutureRwdLockReadGuard<'a, T: 'a + ?Sized, N: IN> {
    phantom: PhantomData<N>,
    inner: &'a FutureRwdLock<T, N>,
}

/// A guard that provides mutable data access.
///
/// When the guard falls out of scope it will release the lock.
pub struct FutureRwdLockWriteGuard<'a, T: 'a + ?Sized, N: IN> {
    phantom: PhantomData<N>,
    inner: &'a FutureRwdLock<T, N>,
    data: &'a mut T,
}

/// A guard that provides mutable data access.
///
/// When the guard falls out of scope it will release the lock.
pub struct FutureRwdLockDiskGuard<'a, T: 'a + ?Sized, N: IN> {
    phantom: PhantomData<N>,
    inner: &'a FutureRwdLock<T, N>,
    data: &'a mut T,
}

// Same unsafe impls as `std::sync::FutureRwdLock`
unsafe impl<N: IN, T: ?Sized + Send> Send for FutureRwdLock<T, N> {}
unsafe impl<N: IN, T: ?Sized + Send + Sync> Sync for FutureRwdLock<T, N> {}

impl<T, N: IN> FutureRwdLock<T, N> {
    #[inline]
    pub fn new(data: T) -> Self {
        FutureRwdLock::<T, N> {
            phantom: PhantomData,
            lock: Semaphore::<N>::new(),
            data: UnsafeCell::new(data),
        }
    }

    /// Consumes this `FutureRwdLock`eturning the underlying data.
    #[inline]
    pub fn into_inner(self) -> T {
        // We know statically that there are no outstanding references to
        // `self` so there's no need to lock.
        let FutureRwdLock { data, .. } = self;
        data.into_inner()
    }

    #[inline(always)]
    pub fn as_mut_ptr(&self) -> *mut T {
        self.data.get()
    }
}

impl<T: ?Sized, N: IN> FutureRwdLock<T, N> {
    pub async fn read(&self) -> FutureRwdLockReadGuard<'_, T, N> {
        self.lock.acquire_read().await;
        FutureRwdLockReadGuard {
            phantom: PhantomData,
            inner: self,
        }
    }

    pub async fn write(&self) -> FutureRwdLockWriteGuard<'_, T, N> {
        self.lock.acquire_write().await;
        FutureRwdLockWriteGuard {
            phantom: PhantomData,
            inner: self,
            data: unsafe { &mut *self.data.get() },
        }
    }

    pub async fn disk(&self) -> FutureRwdLockDiskGuard<'_, T, N> {
        self.lock.acquire_disk().await;
        FutureRwdLockDiskGuard {
            phantom: PhantomData,
            inner: self,
            data: unsafe { &mut *self.data.get() },
        }
    }

    #[inline]
    pub fn try_read(&self) -> Option<FutureRwdLockReadGuard<T, N>> {
        if self.lock.try_acquire_read().is_ok() {
            Some(FutureRwdLockReadGuard {
                phantom: PhantomData,
                inner: self,
            })
        } else {
            None
        }
    }

    pub fn spin_read(&self) -> FutureRwdLockReadGuard<T, N> {
        loop {
            match self.try_read() {
                Some(guard) => return guard,
                None => core::hint::spin_loop(),
            }
        }
    }

    #[inline(always)]
    pub fn try_write(&self) -> Option<FutureRwdLockWriteGuard<T, N>> {
        if self.lock.try_acquire_write().is_ok() {
            Some(FutureRwdLockWriteGuard {
                phantom: PhantomData,
                inner: self,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            None
        }
    }

    pub fn spin_write(&self) -> FutureRwdLockWriteGuard<T, N> {
        loop {
            match self.try_write() {
                Some(guard) => return guard,
                None => core::hint::spin_loop(),
            }
        }
    }

    #[inline(always)]
    pub fn try_disk(&self) -> Option<FutureRwdLockDiskGuard<T, N>> {
        if self.lock.try_acquire_disk().is_ok() {
            Some(FutureRwdLockDiskGuard {
                phantom: PhantomData,
                inner: self,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            None
        }
    }

    pub fn spin_disk(&self) -> FutureRwdLockDiskGuard<T, N> {
        loop {
            match self.try_disk() {
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

    pub fn disk_count(&self) -> usize {
        self.lock.disk_count()
    }

    pub fn get_mut(&mut self) -> &mut T {
        // We know statically that there are no other references to `self`, so
        // there's no need to lock the inner lock.
        unsafe { &mut *self.data.get() }
    }
}

impl<'rwlock, T: ?Sized, N: IN> FutureRwdLockDiskGuard<'rwlock, T, N> {
    pub fn downgrade_read(self) -> FutureRwdLockReadGuard<'rwlock, T, N> {
        let inner = self.inner;
        mem::forget(self);
        inner.lock.try_downgrade_read(DISK).unwrap(); // It will never fail
        FutureRwdLockReadGuard {
            phantom: PhantomData,
            inner,
        }
    }

    pub fn downgrade_write(self) -> FutureRwdLockWriteGuard<'rwlock, T, N> {
        let inner = self.inner;
        mem::forget(self);
        inner.lock.try_downgrade_write(DISK).unwrap(); // It will never fail
        FutureRwdLockWriteGuard {
            phantom: PhantomData,
            inner,
            data: unsafe { &mut *inner.data.get() },
        }
    }
}

impl<'rwlock, T: ?Sized, N: IN> FutureRwdLockWriteGuard<'rwlock, T, N> {
    pub fn downgrade_read(self) -> FutureRwdLockReadGuard<'rwlock, T, N> {
        let inner = self.inner;
        mem::forget(self);
        inner.lock.try_downgrade_read(WRITER).unwrap(); // It will never fail
        FutureRwdLockReadGuard {
            phantom: PhantomData,
            inner,
        }
    }

    pub fn upgrade_disk(self) -> FutureRwdLockDiskGuard<'rwlock, T, N> {
        let inner = self.inner;
        mem::forget(self);
        inner.lock.try_upgrade_disk(WRITER).unwrap(); // It will never fail
        FutureRwdLockDiskGuard {
            phantom: PhantomData,
            inner,
            data: unsafe { &mut *inner.data.get() },
        }
    }
}

impl<'rwlock, T: ?Sized, N: IN> FutureRwdLockReadGuard<'rwlock, T, N> {
    pub fn try_upgrade_disk(self) -> Result<FutureRwdLockDiskGuard<'rwlock, T, N>, Self> {
        if self.inner.lock.read_upgrade_disk(READER).is_ok() {
            let inner = self.inner;
            mem::forget(self);
            Ok(FutureRwdLockDiskGuard {
                phantom: PhantomData,
                inner,
                data: unsafe { &mut *inner.data.get() },
            })
        } else {
            Err(self)
        }
    }

    pub fn try_upgrade_write(self) -> Result<FutureRwdLockWriteGuard<'rwlock, T, N>, Self> {
        if self.inner.lock.read_upgrade_write(READER).is_ok() {
            let inner = self.inner;
            mem::forget(self);
            Ok(FutureRwdLockWriteGuard {
                phantom: PhantomData,
                inner,
                data: unsafe { &mut *inner.data.get() },
            })
        } else {
            Err(self)
        }
    }
}

impl<T: ?Sized + fmt::Debug, N: IN> fmt::Debug for FutureRwdLock<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_read() {
            Some(guard) => write!(f, "FutureRwdLock {{ data: ")
                .and_then(|()| (&*guard).fmt(f))
                .and_then(|()| write!(f, "}}")),
            None => write!(f, "FutureRwdLock {{ <locked> }}"),
        }
    }
}

impl<T: ?Sized + Default, N: IN> Default for FutureRwdLock<T, N> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T, N: IN> From<T> for FutureRwdLock<T, N> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

impl<'rwlock, T: ?Sized, N: IN> FutureRwdLockReadGuard<'rwlock, T, N> {
    #[inline]
    pub fn leak(this: Self) -> &'rwlock T {
        N::pop_off();
        let Self { phantom: _, inner } = this;
        unsafe { &*inner.data.get() }
    }
}

impl<'rwlock, T: ?Sized + fmt::Debug, N: IN> fmt::Debug for FutureRwdLockReadGuard<'rwlock, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'rwlock, T: ?Sized + fmt::Display, N: IN> fmt::Display
    for FutureRwdLockReadGuard<'rwlock, T, N>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<'rwlock, T: ?Sized, N: IN> FutureRwdLockWriteGuard<'rwlock, T, N> {
    #[inline]
    pub fn leak(this: Self) -> &'rwlock mut T {
        N::pop_off();
        let data = this.data as *mut _; // Keep it in pointer form temporarily to avoid double-aliasing
        core::mem::forget(this);
        unsafe { &mut *data }
    }
}

impl<'rwlock, T: ?Sized + fmt::Debug, N: IN> fmt::Debug for FutureRwdLockWriteGuard<'rwlock, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'rwlock, T: ?Sized + fmt::Display, N: IN> fmt::Display
    for FutureRwdLockWriteGuard<'rwlock, T, N>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<'rwlock, T: ?Sized, N: IN> FutureRwdLockDiskGuard<'rwlock, T, N> {
    #[inline]
    pub fn leak(this: Self) -> &'rwlock mut T {
        N::pop_off();
        let data = this.data as *mut _; // Keep it in pointer form temporarily to avoid double-aliasing
        core::mem::forget(this);
        unsafe { &mut *data }
    }
}

impl<'rwlock, T: ?Sized + fmt::Debug, N: IN> fmt::Debug for FutureRwdLockDiskGuard<'rwlock, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'rwlock, T: ?Sized + fmt::Display, N: IN> fmt::Display
    for FutureRwdLockDiskGuard<'rwlock, T, N>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<'rwlock, T: ?Sized, N: IN> Deref for FutureRwdLockReadGuard<'rwlock, T, N> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.inner.data.get() }
    }
}

impl<'rwlock, T: ?Sized, N: IN> Deref for FutureRwdLockWriteGuard<'rwlock, T, N> {
    type Target = T;

    fn deref(&self) -> &T {
        self.data
    }
}

impl<'rwlock, T: ?Sized, N: IN> DerefMut for FutureRwdLockWriteGuard<'rwlock, T, N> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<'rwlock, T: ?Sized, N: IN> Deref for FutureRwdLockDiskGuard<'rwlock, T, N> {
    type Target = T;

    fn deref(&self) -> &T {
        self.data
    }
}

impl<'rwlock, T: ?Sized, N: IN> DerefMut for FutureRwdLockDiskGuard<'rwlock, T, N> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<'rwlock, T: ?Sized, N: IN> Drop for FutureRwdLockReadGuard<'rwlock, T, N> {
    fn drop(&mut self) {
        self.inner.lock.release_read();
    }
}

impl<'rwlock, T: ?Sized, N: IN> Drop for FutureRwdLockWriteGuard<'rwlock, T, N> {
    fn drop(&mut self) {
        self.inner.lock.release_write();
    }
}

impl<'rwlock, T: ?Sized, N: IN> Drop for FutureRwdLockDiskGuard<'rwlock, T, N> {
    fn drop(&mut self) {
        self.inner.lock.release_disk();
    }
}
