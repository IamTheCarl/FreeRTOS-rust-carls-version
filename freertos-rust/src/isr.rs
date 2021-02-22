use crate::base::*;
use crate::shim::*;

/// Keep track of whether we need to yield the execution to a different
/// task at the end of the interrupt.
///
/// Should be dropped as the last thing inside a interrupt.
pub struct InterruptContext {
    x_higher_priority_task_woken: FreeRtosBaseType,
}

impl InterruptContext {
    /// Instantiate a new context.
    pub unsafe fn new() -> InterruptContext {
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
