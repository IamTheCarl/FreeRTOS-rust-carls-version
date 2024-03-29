use crate::base::*;
use crate::isr::*;
use crate::operating_system::*;
use crate::prelude::v1::*;
use crate::shim::*;
use crate::units::*;

unsafe impl<T: Sized + Copy> Send for Queue<T> {}
unsafe impl<T: Sized + Copy> Sync for Queue<T> {}

impl<T: Sized + Copy> !ISRSafe for Queue<T> {}

/// A queue with a finite size. The items are owned by the queue and are
/// copied.
#[derive(Debug)]
pub struct Queue<T: Sized + Copy> {
    queue: FreeRtosQueueHandle,
    item_type: PhantomData<T>,
}

impl<T: Sized + Copy> Queue<T> {
    pub fn new(_os: FreeRTOS, max_size: usize) -> Result<Queue<T>, FreeRtosError> {
        let item_size = mem::size_of::<T>();

        let handle = unsafe { freertos_rs_queue_create(max_size as u32, item_size as u32) };

        if handle == 0 as *const _ {
            Err(FreeRtosError::OutOfMemory)
        } else {
            Ok(Queue {
                queue: handle,
                item_type: PhantomData,
            })
        }
    }

    /// Send an item to the end of the queue. Wait for the queue to have empty space for it.
    pub fn send<D: DurationTicks>(&self, item: T, max_wait: D) -> Result<(), FreeRtosError> {
        unsafe {
            if freertos_rs_queue_send(
                self.queue,
                &item as *const _ as FreeRtosVoidPtr,
                max_wait.to_ticks(),
            ) != 0
            {
                Err(FreeRtosError::QueueSendTimeout)
            } else {
                Ok(())
            }
        }
    }

    /// Wait for an item to be available on the queue.
    pub fn receive<D: DurationTicks>(&self, max_wait: D) -> Result<T, FreeRtosError> {
        unsafe {
            let mut buff = mem::zeroed::<T>();
            let r = freertos_rs_queue_receive(
                self.queue,
                &mut buff as *mut _ as FreeRtosMutVoidPtr,
                max_wait.to_ticks(),
            );
            if r == 0 {
                return Ok(buff);
            } else {
                return Err(FreeRtosError::QueueReceiveTimeout);
            }
        }
    }
}

impl<T: Sized + Copy> Drop for Queue<T> {
    fn drop(&mut self) {
        unsafe {
            freertos_rs_queue_delete(self.queue);
        }
    }
}

pub struct QueueISRHandle<T: Sized + Copy> {
    queue: FreeRtosQueueHandle,
    item_type: PhantomData<T>,
}

impl<T: Sized + Copy + ISRSafe> ISRSafeHandle<QueueISRHandle<T>> for Queue<T> {
    unsafe fn new_isr_safe_handle(&self) -> QueueISRHandle<T> {
        QueueISRHandle {
            queue: self.queue,
            item_type: self.item_type,
        }
    }
}

impl<T: Sized + Copy> QueueISRHandle<T> {
    /// Send an item to the end of the queue, from an interrupt.
    pub fn send(&self, context: &mut InterruptContext, item: T) -> Result<(), FreeRtosError> {
        unsafe {
            if freertos_rs_queue_send_isr(
                self.queue,
                &item as *const _ as FreeRtosVoidPtr,
                context.get_task_field_mut(),
            ) != 0
            {
                Err(FreeRtosError::QueueFull)
            } else {
                Ok(())
            }
        }
    }

    // Receive an item from the front of the queue, from an interrupt.
    pub fn receive<D: DurationTicks>(&self, context: &mut InterruptContext) -> Option<T> {
        unsafe {
            let mut buff = mem::zeroed::<T>();
            let r = freertos_rs_queue_receive_isr(
                self.queue,
                &mut buff as *mut _ as FreeRtosMutVoidPtr,
                context.get_task_field_mut(),
            );
            if r == 0 {
                Some(buff)
            } else {
                None
            }
        }
    }
}
