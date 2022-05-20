use crate::future_mutex::*;
use crate::rwlock::*;

use core::{
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::NestStrategy as IN;

pub struct LsLock<T: ?Sized, N: IN> {
    phantom: PhantomData<N>,
    llock: FutureMutex<(), N>,
    slock: RwLock<T, N>,
}

/// A guard that provides mutable data access.
///
/// When the guard falls out of scope it will release the lock.
pub struct LsLockLongGuard<'a, T: 'a + ?Sized, N: IN> {
    lguard: FutureMutexGuard<'a, (), N>,
    sguard: RwLockWriteGuard<'a, T, N>,
}

/// A guard that provides immutable data access.
///
/// When the guard falls out of scope it will decrement the read count,
/// potentially releasing the lock.
pub struct LsLockReadGuard<'a, T: 'a + ?Sized, N: IN> {
    sguard: RwLockReadGuard<'a, T, N>,
}

/// A guard that provides mutable data access.
///
/// When the guard falls out of scope it will release the lock.
pub struct LsLockWriteGuard<'a, T: 'a + ?Sized, N: IN> {
    sguard: RwLockWriteGuard<'a, T, N>,
}

// Same unsafe impls as `std::sync::LsLock`
unsafe impl<N: IN, T: ?Sized + Send> Send for LsLock<T, N> {}
unsafe impl<N: IN, T: ?Sized + Send + Sync> Sync for LsLock<T, N> {}

impl<T, N: IN> LsLock<T, N> {
    #[inline]
    pub fn new(data: T) -> Self {
        Self {
            phantom: PhantomData,
            llock: FutureMutex::<(), N>::new(()),
            slock: RwLock::<T, N>::new(data),
        }
    }

    /// Consumes this `LsLock` returning the underlying data.
    #[inline]
    pub fn into_inner(self) -> T {
        // We know statically that there are no outstanding references to
        // `self` so there's no need to lock.
        self.slock.into_inner()
    }

    #[inline(always)]
    pub fn as_mut_ptr(&self) -> *mut T {
        self.slock.as_mut_ptr()
    }
}

impl<T: ?Sized, N: IN> LsLock<T, N> {
    pub async fn disk(&self) -> LsLockLongGuard<'_, T, N> {
        let lguard = self.llock.lock().await;
        let sguard = self.slock.write();
        LsLockLongGuard { lguard, sguard }
    }

    pub async fn read(&self) -> LsLockReadGuard<'_, T, N> {
        let lguard = self.llock.lock().await;
        let sguard = self.slock.read();
        drop(lguard);
        LsLockReadGuard { sguard }
    }

    pub async fn write(&self) -> LsLockWriteGuard<'_, T, N> {
        let lguard = self.llock.lock().await;
        let sguard = self.slock.write();
        drop(lguard);
        LsLockWriteGuard { sguard }
    }

    #[inline]
    pub fn try_read(&self) -> Option<LsLockReadGuard<T, N>> {
        if let Some(lguard) = self.llock.try_lock() {
            if let Some(sguard) = self.slock.try_read() {
                drop(lguard);
                return Some(LsLockReadGuard { sguard });
            }
        }
        None
    }

    pub fn spin_read(&self) -> LsLockReadGuard<T, N> {
        let lguard = self.llock.spin_lock();
        let sguard = self.slock.read();
        drop(lguard);
        return LsLockReadGuard { sguard };
    }

    #[inline(always)]
    fn try_write(&self) -> Option<LsLockWriteGuard<T, N>> {
        if let Some(lguard) = self.llock.try_lock() {
            if let Some(sguard) = self.slock.try_write() {
                drop(lguard);
                return Some(LsLockWriteGuard { sguard });
            }
        }
        None
    }

    pub fn spin_write(&self) -> LsLockWriteGuard<T, N> {
        let lguard = self.llock.spin_lock();
        let sguard = self.slock.write();
        drop(lguard);
        return LsLockWriteGuard { sguard };
    }

    pub fn reader_count(&self) -> usize {
        self.slock.reader_count()
    }

    pub fn writer_count(&self) -> usize {
        self.slock.writer_count()
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.slock.get_mut()
    }
}

impl<T: ?Sized + fmt::Debug, N: IN> fmt::Debug for LsLock<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_read() {
            Some(guard) => write!(f, "LsLock {{ data: ")
                .and_then(|()| (&*guard).fmt(f))
                .and_then(|()| write!(f, "}}")),
            None => write!(f, "LsLock {{ <locked> }}"),
        }
    }
}

impl<T: ?Sized + Default, N: IN> Default for LsLock<T, N> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T, N: IN> From<T> for LsLock<T, N> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

// impl<'rwlock, T: ?Sized, N: IN> LsLockReadGuard<'rwlock, T, N> {
//     #[inline]
//     pub fn leak(this: Self) -> &'rwlock T {
//         crate::RwLockReadGuard::leak(this.sguard)
//     }
// }

impl<'rwlock, T: ?Sized + fmt::Debug, N: IN> fmt::Debug for LsLockReadGuard<'rwlock, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'rwlock, T: ?Sized + fmt::Display, N: IN> fmt::Display for LsLockReadGuard<'rwlock, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

// impl<'rwlock, T: ?Sized, N: IN> LsLockWriteGuard<'rwlock, T, N> {
//     #[inline]
//     pub fn leak(this: Self) -> &'rwlock mut T {
//         crate::RwLockWriteGuard::leak(this.sguard)
//     }
// }

impl<'rwlock, T: ?Sized + fmt::Debug, N: IN> fmt::Debug for LsLockWriteGuard<'rwlock, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'rwlock, T: ?Sized + fmt::Display, N: IN> fmt::Display for LsLockWriteGuard<'rwlock, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

// impl<'rwlock, T: ?Sized, N: IN> LsLockLongGuard<'rwlock, T, N> {
//     #[inline]
//     pub fn leak(this: Self) -> &'rwlock mut T {
//         crate::RwLockWriteGuard::leak(this.sguard)
//     }
// }

impl<'rwlock, T: ?Sized + fmt::Debug, N: IN> fmt::Debug for LsLockLongGuard<'rwlock, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'rwlock, T: ?Sized + fmt::Display, N: IN> fmt::Display for LsLockLongGuard<'rwlock, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<'rwlock, T: ?Sized, N: IN> Deref for LsLockReadGuard<'rwlock, T, N> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.sguard
    }
}

impl<'rwlock, T: ?Sized, N: IN> Deref for LsLockWriteGuard<'rwlock, T, N> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.sguard
    }
}

impl<'rwlock, T: ?Sized, N: IN> Deref for LsLockLongGuard<'rwlock, T, N> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.sguard
    }
}

impl<'rwlock, T: ?Sized, N: IN> DerefMut for LsLockWriteGuard<'rwlock, T, N> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.sguard
    }
}

impl<'rwlock, T: ?Sized, N: IN> DerefMut for LsLockLongGuard<'rwlock, T, N> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.sguard
    }
}
