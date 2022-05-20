cfg_if::cfg_if! {
    if #[cfg(feature = "ticket")] {
        pub mod ticket;
        pub type Mutex<T, N> = ticket::TicketMutex<T, N>;
        pub type MutexGuard<'a, T, N> = ticket::TicketMutexGuard<'a, T, N>;
    } else {
        pub mod spin;
        pub type Mutex<T, N> = spin::SpinMutex<T, N>;
        pub type MutexGuard<T, N> = spin::SpinMutexGuard<T, N>;
    }
}

pub mod mcslock;
pub type MCSLock<T, N> = mcslock::MCSLock<T, N>;
pub type MCSLockGuard<'a, T, N> = mcslock::MCSLockGuard<'a, T, N>;
pub mod rwlock;
pub type RwLock<T, N> = rwlock::RwLock<T, N>;
pub type RwLockReadGuard<'a, T, N> = rwlock::RwLockReadGuard<'a, T, N>;
pub type RwLockWriteGuard<'a, T, N> = rwlock::RwLockWriteGuard<'a, T, N>;
pub type RwLockUpgradableGuard<'a, T, N> = rwlock::RwLockUpgradableGuard<'a, T, N>;

pub mod no_irq {
    use crate::nest::NoIrqNest;
    cfg_if::cfg_if! {
        if #[cfg(feature = "ticket")] {
            use super::ticket;
            pub type Mutex<T> = ticket::TicketMutex<T, NoIrqNest>;
            pub type MutexGuard<'a, T> = ticket::TicketMutexGuard<'a, T, NoIrqNest>;
        } else {
            use super::spin;
            pub type Mutex<T> = spin::SpinMutex<T>;
            pub type MutexGuard<T> = spin::SpinMutexGuard<T>;
        }
    }
    use super::mcslock;
    pub type MCSLock<T> = mcslock::MCSLock<T, NoIrqNest>;
    pub type MCSLockGuard<'a, T> = mcslock::MCSLockGuard<'a, T, NoIrqNest>;
    use super::rwlock;
    pub type RwLock<T> = rwlock::RwLock<T, NoIrqNest>;
    pub type RwLockReadGuard<'a, T> = rwlock::RwLockReadGuard<'a, T, NoIrqNest>;
    pub type RwLockWriteGuard<'a, T> = rwlock::RwLockWriteGuard<'a, T, NoIrqNest>;
    pub type RwLockUpgradableGuard<'a, T> = rwlock::RwLockUpgradableGuard<'a, T, NoIrqNest>;
}

pub mod mock {
    use crate::nest::MockNest;
    cfg_if::cfg_if! {
        if #[cfg(feature = "ticket")] {
            use super::ticket;
            pub type Mutex<T> = ticket::TicketMutex<T, MockNest>;
            pub type MutexGuard<'a, T> = ticket::TicketMutexGuard<'a, T, MockNest>;
        } else {
            use super::spin;
            pub type Mutex<T> = spin::SpinMutex<T, MockNest>;
            pub type MutexGuard<T> = spin::SpinMutexGuard<T, MockNest>;
        }
    }
    use super::mcslock;
    pub type MCSLock<T> = mcslock::MCSLock<T, MockNest>;
    pub type MCSLockGuard<'a, T> = mcslock::MCSLockGuard<'a, T, MockNest>;

    use super::rwlock;
    pub type RwLock<T> = rwlock::RwLock<T, MockNest>;
    pub type RwLockReadGuard<'a, T> = rwlock::RwLockReadGuard<'a, T, MockNest>;
    pub type RwLockWriteGuard<'a, T> = rwlock::RwLockWriteGuard<'a, T, MockNest>;
    pub type RwLockUpgradableGuard<'a, T> = rwlock::RwLockUpgradableGuard<'a, T, MockNest>;
}
