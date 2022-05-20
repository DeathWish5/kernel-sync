use core::{
    cell::UnsafeCell,
    default::Default,
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::NestStrategy as IN;

pub struct SpinMutex<T: ?Sized, N: IN> {
    phantom: PhantomData<N>,
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

/// An RAII implementation of a “scoped lock” of a mutex.
/// When this structure is dropped (falls out of scope),
/// the lock will be unlocked.
///
pub struct SpinMutexGuard<'a, T: ?Sized + 'a, N: IN> {
    phantom: PhantomData<N>,
    lock: &'a AtomicBool,
    data: &'a mut T,
}

unsafe impl<N: IN, T: ?Sized + Send> Sync for SpinMutex<T, N> {}
unsafe impl<N: IN, T: ?Sized + Send> Send for SpinMutex<T, N> {}

impl<T, N: IN> SpinMutex<T, N> {
    #[inline(always)]
    pub const fn new(data: T) -> Self {
        SpinMutex {
            phantom: PhantomData,
            locked: AtomicBool::new(false),
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

impl<T: ?Sized, N: IN> SpinMutex<T, N> {
    #[inline(always)]
    pub fn lock(&self) -> SpinMutexGuard<T, N> {
        N::push_off();
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Wait until the lock looks unlocked before retrying
            while self.is_locked() {
                core::hint::spin_loop();
            }
        }
        SpinMutexGuard {
            phantom: PhantomData,
            lock: &self.locked,
            data: unsafe { &mut *self.data.get() },
        }
    }

    #[inline(always)]
    pub fn try_lock(&self) -> Option<SpinMutexGuard<T, N>> {
        N::push_off();
        if self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(SpinMutexGuard {
                phantom: PhantomData,
                lock: &self.locked,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            N::pop_off();
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
        self.locked.load(Ordering::Relaxed)
    }
}

impl<T: ?Sized + fmt::Debug, N: IN> fmt::Debug for SpinMutex<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_lock() {
            Some(guard) => write!(f, "Mutex {{ data: ")
                .and_then(|()| (&*guard).fmt(f))
                .and_then(|()| write!(f, "}}")),
            None => write!(f, "Mutex {{ <locked> }}"),
        }
    }
}

impl<T: ?Sized + Default> Default for SpinMutex<T, N> {
    fn default() -> Self {
        SpinMutex::new(T::default())
    }
}

impl<T, N: IN> From<T> for SpinMutex<T, N> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

impl<'a, T: ?Sized, N: IN> Drop for SpinMutexGuard<'a, T, N> {
    /// The dropping of the SpinMutexGuard will release the lock it was created from.
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
        N::pop_off();
    }
}

impl<'a, T: ?Sized, N: IN> Deref for SpinMutexGuard<'a, T, N> {
    type Target = T;
    fn deref(&self) -> &T {
        self.data
    }
}

impl<'a, T: ?Sized, N: IN> DerefMut for SpinMutexGuard<'a, T, N> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for SpinMutexGuard<'a, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized + fmt::Display> fmt::Display for SpinMutexGuard<'a, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}
