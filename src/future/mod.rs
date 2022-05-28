pub mod rw_semaphore;
pub type RwSemaphore<N> = rw_semaphore::RwSemaphore<N>;

pub mod rwd_semaphore;
pub type RwdSemaphore<N> = rwd_semaphore::RwdSemaphore<N>;

pub mod future_mcslock;
pub type FutureMCSLock<T, N> = future_mcslock::FutureMCSLock<T, N>;
pub type FutureMCSLockGuard<'a, T, N> = future_mcslock::FutureMCSLockGuard<'a, T, N>;

pub mod future_mutex;
pub type FutureMutex<T, N> = future_mutex::FutureMutex<T, N>;
pub type FutureMutexGuard<'a, T, N> = future_mutex::FutureMutexGuard<'a, T, N>;

pub mod future_rwlock;
pub type FutureRwLock<T, N> = future_rwlock::FutureRwLock<T, N>;
pub type FutureRwLockReadGuard<'a, T, N> = future_rwlock::FutureRwLockReadGuard<'a, T, N>;
pub type FutureRwLockWriteGuard<'a, T, N> = future_rwlock::FutureRwLockWriteGuard<'a, T, N>;

pub mod future_rwdlock;
pub type FutureRwdLock<T, N> = future_rwdlock::FutureRwdLock<T, N>;
pub type FutureRwdLockReadGuard<'a, T, N> = future_rwdlock::FutureRwdLockReadGuard<'a, T, N>;
pub type FutureRwdLockWriteGuard<'a, T, N> = future_rwdlock::FutureRwdLockWriteGuard<'a, T, N>;
pub type FutureRwdLockDiskGuard<'a, T, N> = future_rwdlock::FutureRwdLockDiskGuard<'a, T, N>;

pub mod no_irq {
    use super::rw_semaphore;
    use crate::nest::NoIrqNest;
    pub type RwSemaphore = rw_semaphore::RwSemaphore<NoIrqNest>;
    use super::rwd_semaphore;
    pub type RwdSemaphore = rwd_semaphore::RwdSemaphore<NoIrqNest>;
    use super::future_mcslock;
    pub type FutureMCSLock<T> = future_mcslock::FutureMCSLock<T, NoIrqNest>;
    pub type FutureMCSLockGuard<'a, T> = future_mcslock::FutureMCSLockGuard<'a, T, NoIrqNest>;
    use super::future_mutex;
    pub type FutureMutex<T> = future_mutex::FutureMutex<T, NoIrqNest>;
    pub type FutureMutexGuard<'a, T> = future_mutex::FutureMutexGuard<'a, T, NoIrqNest>;
    use super::future_rwlock;
    pub type FutureRwLock<T> = future_rwlock::FutureRwLock<T, NoIrqNest>;
    pub type FutureRwLockReadGuard<'a, T> = future_rwlock::FutureRwLockReadGuard<'a, T, NoIrqNest>;
    pub type FutureRwLockWriteGuard<'a, T> =
        future_rwlock::FutureRwLockWriteGuard<'a, T, NoIrqNest>;
    use super::future_rwdlock;
    pub type FutureRwdLock<T> = future_rwdlock::FutureRwdLock<T, NoIrqNest>;
    pub type FutureRwdLockReadGuard<'a, T> =
        future_rwdlock::FutureRwdLockReadGuard<'a, T, NoIrqNest>;
    pub type FutureRwdLockWriteGuard<'a, T> =
        future_rwdlock::FutureRwdLockWriteGuard<'a, T, NoIrqNest>;
    pub type FutureRwdLockDiskGuard<'a, T> =
        future_rwdlock::FutureRwdLockDiskGuard<'a, T, NoIrqNest>;
}

pub mod mock {
    use super::{
        future_mcslock, future_mutex, future_rwdlock, future_rwlock, rw_semaphore, rwd_semaphore,
    };
    use crate::nest::MockNest;
    pub type RwSemaphore = rw_semaphore::RwSemaphore<MockNest>;
    pub type RwdSemaphore = rwd_semaphore::RwdSemaphore<MockNest>;
    pub type FutureMCSLock<T> = future_mcslock::FutureMCSLock<T, MockNest>;
    pub type FutureMCSLockGuard<'a, T> = future_mcslock::FutureMCSLockGuard<'a, T, MockNest>;
    pub type FutureMutex<T> = future_mutex::FutureMutex<T, MockNest>;
    pub type FutureMutexGuard<'a, T> = future_mutex::FutureMutexGuard<'a, T, MockNest>;
    pub type FutureRwLock<T> = future_rwlock::FutureRwLock<T, MockNest>;
    pub type FutureRwLockReadGuard<'a, T> = future_rwlock::FutureRwLockReadGuard<'a, T, MockNest>;
    pub type FutureRwLockWriteGuard<'a, T> = future_rwlock::FutureRwLockWriteGuard<'a, T, MockNest>;
    pub type FutureRwdLock<T> = future_rwdlock::FutureRwdLock<T, MockNest>;
    pub type FutureRwdLockReadGuard<'a, T> =
        future_rwdlock::FutureRwdLockReadGuard<'a, T, MockNest>;
    pub type FutureRwdLockWriteGuard<'a, T> =
        future_rwdlock::FutureRwdLockWriteGuard<'a, T, MockNest>;
    pub type FutureRwdLockDiskGuard<'a, T> =
        future_rwdlock::FutureRwdLockDiskGuard<'a, T, MockNest>;
}
