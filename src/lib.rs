#![no_std]
#![feature(get_mut_unchecked)]

use cfg_if::cfg_if;

extern crate alloc;

mod interrupt;

cfg_if! {
    if #[cfg(target_os = "none")] {
        pub(crate) use interrupt::{push_off, pop_off};
    } else {
        pub mod mock {
            pub(crate) fn push_off() {}
            pub(crate) fn pop_off() {}
        }
        pub(crate) use mock::{push_off, pop_off};
    }
}

pub mod future_mutex;
pub mod future_rwlock;
pub mod rw_semaphore;
pub use {future_mutex::*, future_rwlock::*, rw_semaphore::*};

cfg_if! {
    if #[cfg(target_os = "none")] {
        pub mod mcslock;
        pub mod rwlock;
        pub use {rwlock::*, mcslock::*};
        cfg_if! {
            if #[cfg(feature = "ticket")] {
                pub mod ticket;
                pub use ticket::{TicketMutex as Mutex, TicketMutexGuard as MutexGuard};
            } else {
                pub mod spin;
                pub use spin::{SpinMutex as Mutex, SpinMutexGuard as MutexGuard};
            }
        }
    } else {
        pub use spin::*;
    }
}
