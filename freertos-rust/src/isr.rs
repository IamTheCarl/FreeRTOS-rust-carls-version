use crate::base::*;
use crate::shim::*;

pub auto trait ISRSafe {}

/// Structs that implement this can have an ISR safe handle created.
pub trait ISRSafeHandle<SafeForm: ISRSafe> {
    /// Create an ISR safe handle to this object. Calling functions on it from within a task will not cause
    /// memory corruption issues but may mess with the deterministic qualities of FreeRTOs.
    ///
    /// Safety:
    /// The ISR safe form of a struct contains a reference to the original struct. If the original goes out of scope
    /// before this reference does, you'll run the risk of an invalid memory access.
    unsafe fn new_isr_safe_handle(&self) -> SafeForm;

    /// This is identical to the `to_isr_safe_form` function, except it requires that self have a static lifetime. Since that
    /// object will never be disposed of, you can safely assume that it will always be valid.
    fn new_isr_safe_handle_form_static(&'static self) -> SafeForm {
        unsafe { self.new_isr_safe_handle() }
    }
}

/// Keep track of whether we need to yield the execution to a different
/// task at the end of the interrupt.
///
/// Should be dropped as the last thing inside a interrupt.
pub struct InterruptContext {
    x_higher_priority_task_woken: FreeRtosBaseType,
}

impl InterruptContext {
    /// Instantiate a new context.
    pub fn new() -> InterruptContext {
        InterruptContext {
            x_higher_priority_task_woken: 0,
        }
    }

    pub unsafe fn get_task_field_mut(&self) -> FreeRtosBaseTypeMutPtr {
        self.x_higher_priority_task_woken as *mut _
    }
}

impl Drop for InterruptContext {
    fn drop(&mut self) {
        if self.x_higher_priority_task_woken == 1 {
            unsafe {
                freertos_rs_isr_yield();
            }
        }
    }
}
