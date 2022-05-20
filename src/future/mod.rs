pub mod rw_semaphore;
pub type RwSemaphore<N> = rw_semaphore::RwSemaphore<N>;

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

pub mod lslock;
pub type LsLock<T, N> = lslock::LsLock<T, N>;
pub type LsLockLongGuard<'a, T, N> = lslock::LsLockLongGuard<'a, T, N>;
pub type LsLockReadGuard<'a, T, N> = lslock::LsLockReadGuard<'a, T, N>;
pub type LsLockWriteGuard<'a, T, N> = lslock::LsLockWriteGuard<'a, T, N>;

pub mod no_irq {
    use super::rw_semaphore;
    use crate::nest::NoIrqNest;
    pub type RwSemaphore = rw_semaphore::RwSemaphore<NoIrqNest>;
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
    use super::lslock;
    pub type LsLock<T> = lslock::LsLock<T, NoIrqNest>;
    pub type LsLockLongGuard<'a, T> = lslock::LsLockLongGuard<'a, T, NoIrqNest>;
    pub type LsLockReadGuard<'a, T> = lslock::LsLockReadGuard<'a, T, NoIrqNest>;
    pub type LsLockWriteGuard<'a, T> = lslock::LsLockWriteGuard<'a, T, NoIrqNest>;
}

pub mod mock {
    use super::{future_mcslock, future_mutex, future_rwlock, lslock, rw_semaphore};
    use crate::nest::MockNest;
    pub type RwSemaphore = rw_semaphore::RwSemaphore<MockNest>;
    pub type FutureMCSLock<T> = future_mcslock::FutureMCSLock<T, MockNest>;
    pub type FutureMCSLockGuard<'a, T> = future_mcslock::FutureMCSLockGuard<'a, T, MockNest>;
    pub type FutureMutex<T> = future_mutex::FutureMutex<T, MockNest>;
    pub type FutureMutexGuard<'a, T> = future_mutex::FutureMutexGuard<'a, T, MockNest>;
    pub type FutureRwLock<T> = future_rwlock::FutureRwLock<T, MockNest>;
    pub type FutureRwLockReadGuard<'a, T> = future_rwlock::FutureRwLockReadGuard<'a, T, MockNest>;
    pub type FutureRwLockWriteGuard<'a, T> = future_rwlock::FutureRwLockWriteGuard<'a, T, MockNest>;
    pub type LsLock<T> = lslock::LsLock<T, MockNest>;
    pub type LsLockLongGuard<'a, T> = lslock::LsLockLongGuard<'a, T, MockNest>;
    pub type LsLockReadGuard<'a, T> = lslock::LsLockReadGuard<'a, T, MockNest>;
    pub type LsLockWriteGuard<'a, T> = lslock::LsLockWriteGuard<'a, T, MockNest>;
}
