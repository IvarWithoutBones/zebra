use core::{
    arch::asm,
    cell::UnsafeCell,
    hint::spin_loop,
    marker::{Send, Sync},
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Spinlock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T> Send for Spinlock<T> {}
unsafe impl<T> Sync for Spinlock<T> {}

impl<T> Spinlock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> SpinlockGuard<T> {
        // Disable interrupts
        unsafe {
            let sstatus = {
                let sstatus: usize;
                asm!("csrr {}, sstatus", out(reg) sstatus);
                sstatus
            };
            asm!("csrw sstatus, {}", in(reg) sstatus & !(1 << 1));
        }

        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::SeqCst)
            .is_err()
        {
            spin_loop();
        }

        SpinlockGuard { lock: self }
    }

    pub fn lock_with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.lock();
        f(guard.deref_mut())
    }
}

pub struct SpinlockGuard<'a, T> {
    lock: &'a Spinlock<T>,
}

impl<'a, T> Deref for SpinlockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> DerefMut for SpinlockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T> Drop for SpinlockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);

        // Enable interrupts
        unsafe {
            let sstatus = {
                let sstatus: usize;
                asm!("csrr {}, sstatus", out(reg) sstatus);
                sstatus
            };
            asm!("csrw sstatus, {}", in(reg) sstatus | 1 << 1);
        }
    }
}
