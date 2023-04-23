use core::{
    arch::asm,
    cell::UnsafeCell,
    hint::spin_loop,
    marker::{Send, Sync},
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T> Send for SpinLock<T> {}
unsafe impl<T> Sync for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> SpinLockGuard<T> {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::SeqCst)
            .is_err()
        {
            spin_loop();
        }

        let interrupts_enabled_before = {
            let sstatus: u64;
            unsafe {
                asm!("csrr {}, sstatus", out(reg) sstatus);
            }
            sstatus & (1 << 1) != 0
        };

        SpinLockGuard {
            lock: self,
            interrupts_enabled_before,
        }
    }

    pub fn lock_with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.lock();
        f(guard.deref_mut())
    }
}

pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
    interrupts_enabled_before: bool,
}

impl<'a, T> Deref for SpinLockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> DerefMut for SpinLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T> Drop for SpinLockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);

        unsafe {
            asm!("csrs sstatus, {}", in(reg) (self.interrupts_enabled_before as u64) << 1);
        }
    }
}
