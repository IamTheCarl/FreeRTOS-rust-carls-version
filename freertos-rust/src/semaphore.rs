use crate::base::*;
use crate::isr::*;
use crate::operating_system::*;
use crate::shim::*;
use crate::units::*;

pub trait Semaphore<D: DurationTicks>: Send + Sync {
    fn raw_handle(&self) -> FreeRtosSemaphoreHandle;

    /// Lock this semaphore in a RAII fashion
    fn lock(&self, max_wait: D) -> Result<SemaphoreGuard<D>, FreeRtosError>
    where
        Self: Sized,
    {
        self.take(max_wait)?;

        Ok(SemaphoreGuard { __semaphore: self })
    }

    fn take(&self, max_wait: D) -> Result<(), FreeRtosError> {
        unsafe {
            let res = freertos_rs_take_semaphore(self.raw_handle(), max_wait.to_ticks());

            if res == 0 {
                Ok(())
            } else {
                Err(FreeRtosError::Timeout)
            }
        }
    }

    fn give(&self) {
        unsafe {
            freertos_rs_give_semaphore(self.raw_handle());
        }
    }
}

/// Holds the lock to the semaphore until we are dropped
pub struct SemaphoreGuard<'a, D: DurationTicks + Sized> {
    __semaphore: &'a dyn Semaphore<D>,
}

impl<'a, D: DurationTicks> Drop for SemaphoreGuard<'a, D> {
    fn drop(&mut self) {
        self.__semaphore.give();
    }
}

/// A binary semaphore
pub struct BinarySemaphore {
    semaphore: FreeRtosSemaphoreHandle,
}

unsafe impl Send for BinarySemaphore {}
unsafe impl Sync for BinarySemaphore {}

impl !ISRSafe for BinarySemaphore {}

impl<D: DurationTicks> Semaphore<D> for BinarySemaphore {
    fn raw_handle(&self) -> FreeRtosSemaphoreHandle {
        self.semaphore
    }
}

impl Drop for BinarySemaphore {
    fn drop(&mut self) {
        unsafe {
            freertos_rs_delete_semaphore(self.semaphore);
        }
    }
}

impl BinarySemaphore {
    /// Create a new binary semaphore
    pub fn new(_os: FreeRTOS) -> Result<BinarySemaphore, FreeRtosError> {
        unsafe {
            let s = freertos_rs_create_binary_semaphore();
            if s == 0 as *const _ {
                return Err(FreeRtosError::OutOfMemory);
            }
            Ok(BinarySemaphore { semaphore: s })
        }
    }

    pub fn is_taken(&self) -> bool {
        unsafe { freertos_rs_semaphore_get_count(self.semaphore) == 0 }
    }
}

/// An ISR safe handle to a binary semaphore.
pub struct ISRBinarySemaphore {
    semaphore: FreeRtosSemaphoreHandle,
}

unsafe impl Send for ISRBinarySemaphore {}
unsafe impl Sync for ISRBinarySemaphore {}

impl ISRBinarySemaphore {
    pub fn take<F: FnMut()>(&self, context: &mut InterruptContext, mut closure: F) -> bool {
        if self.take_isr(context) {
            closure();

            self.give_isr(context);
            true
        } else {
            false
        }
    }

    fn take_isr(&self, context: &mut InterruptContext) -> bool {
        unsafe {
            let res = freertos_rs_take_semaphore_isr(self.semaphore, context.get_task_field_mut());

            // We successfully took it.
            res == 0
        }
    }

    fn give_isr(&self, context: &mut InterruptContext) {
        unsafe {
            freertos_rs_give_semaphore_isr(self.semaphore, context.get_task_field_mut());
        }
    }
}

impl ISRSafeHandle<ISRBinarySemaphore> for BinarySemaphore {
    unsafe fn new_isr_safe_handle(&self) -> ISRBinarySemaphore {
        ISRBinarySemaphore {
            semaphore: self.semaphore,
        }
    }
}

/// A counting semaphore.
pub struct CountingSemaphore {
    semaphore: FreeRtosSemaphoreHandle,
}

unsafe impl Send for CountingSemaphore {}
unsafe impl Sync for CountingSemaphore {}

impl !ISRSafe for CountingSemaphore {}

impl<D: DurationTicks> Semaphore<D> for CountingSemaphore {
    fn raw_handle(&self) -> FreeRtosSemaphoreHandle {
        self.semaphore
    }
}

impl Drop for CountingSemaphore {
    fn drop(&mut self) {
        unsafe {
            freertos_rs_delete_semaphore(self.semaphore);
        }
    }
}

impl CountingSemaphore {
    /// Create a new counting semaphore
    pub fn new(_os: FreeRTOS, max: u32, initial: u32) -> Result<CountingSemaphore, FreeRtosError> {
        unsafe {
            let s = freertos_rs_create_counting_semaphore(max, initial);
            if s == 0 as *const _ {
                return Err(FreeRtosError::OutOfMemory);
            }
            Ok(CountingSemaphore { semaphore: s })
        }
    }

    pub fn get_count(&self) -> u32 {
        unsafe { freertos_rs_semaphore_get_count(self.semaphore) }
    }
}
