use core::{
    cell::UnsafeCell,
    default::Default,
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::NestStrategy as IN;

pub struct TicketMutex<T: ?Sized, N: IN> {
    phantom: PhantomData<N>,
    next_ticket: AtomicUsize,
    next_serving: AtomicUsize,
    data: UnsafeCell<T>,
}

/// An RAII implementation of a “scoped lock” of a mutex.
/// When this structure is dropped (falls out of scope),
/// the lock will be unlocked.
///
pub struct TicketMutexGuard<'a, T: ?Sized + 'a, N: IN> {
    phantom: PhantomData<N>,
    next_serving: &'a AtomicUsize,
    ticket: usize,
    data: &'a mut T,
}

unsafe impl<N: IN, T: ?Sized + Send> Sync for TicketMutex<T, N> {}
unsafe impl<N: IN, T: ?Sized + Send> Send for TicketMutex<T, N> {}

impl<T, N: IN> TicketMutex<T, N> {
    #[inline(always)]
    pub const fn new(data: T) -> Self {
        TicketMutex {
            phantom: PhantomData,
            next_ticket: AtomicUsize::new(0),
            next_serving: AtomicUsize::new(0),
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

impl<T: ?Sized, N: IN> TicketMutex<T, N> {
    #[inline(always)]
    pub fn lock(&self) -> TicketMutexGuard<T, N> {
        N::push_off();
        let ticket = self.next_ticket.fetch_add(1, Ordering::Relaxed);
        while self.next_serving.load(Ordering::Acquire) != ticket {
            core::hint::spin_loop();
        }
        TicketMutexGuard {
            phantom: PhantomData,
            next_serving: &self.next_serving,
            ticket,
            // Safety
            // We know that we are the next ticket to be served,
            // so there's no other thread accessing the data.
            //
            // Every other thread has another ticket number so it's
            // definitely stuck in the spin loop above.
            data: unsafe { &mut *self.data.get() },
        }
    }

    #[inline(always)]
    pub fn try_lock(&self) -> Option<TicketMutexGuard<T, N>> {
        N::push_off();
        let ticket = self
            .next_ticket
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |ticket| {
                if self.next_serving.load(Ordering::Acquire) == ticket {
                    Some(ticket + 1)
                } else {
                    None
                }
            });
        if let Ok(ticket) = ticket {
            Some(TicketMutexGuard {
                phantom: PhantomData,
                next_serving: &self.next_serving,
                ticket,
                // Safety
                // We have a ticket that is equal to the next_serving ticket, so we know:
                // - that no other thread can have the same ticket id as this thread
                // - that we are the next one to be served so we have exclusive access to the data
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
        let ticket = self.next_ticket.load(Ordering::Relaxed);
        self.next_serving.load(Ordering::Relaxed) != ticket
    }
}

impl<'a, T: ?Sized, N: IN> Drop for TicketMutexGuard<'a, T, N> {
    /// The dropping of the TicketMutexGuard will release the lock it was created from.
    fn drop(&mut self) {
        let new_ticket = self.ticket + 1;
        self.next_serving.store(new_ticket, Ordering::Release);
        N::pop_off();
    }
}

impl<T: ?Sized + fmt::Debug, N: IN> fmt::Debug for TicketMutex<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_lock() {
            Some(guard) => write!(f, "Mutex {{ data: ")
                .and_then(|()| (&*guard).fmt(f))
                .and_then(|()| write!(f, "}}")),
            None => write!(f, "Mutex {{ <locked> }}"),
        }
    }
}

impl<T: ?Sized + Default, N: IN> Default for TicketMutex<T, N> {
    fn default() -> Self {
        TicketMutex::<T, N>::new(T::default())
    }
}

impl<T, N: IN> From<T> for TicketMutex<T, N> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

impl<'a, T: ?Sized + fmt::Display, N: IN> fmt::Display for TicketMutexGuard<'a, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized + fmt::Debug, N: IN> fmt::Debug for TicketMutexGuard<'a, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized, N: IN> Deref for TicketMutexGuard<'a, T, N> {
    type Target = T;
    fn deref(&self) -> &T {
        self.data
    }
}

impl<'a, T: ?Sized, N: IN> DerefMut for TicketMutexGuard<'a, T, N> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}
