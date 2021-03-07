use crate::base::*;
use crate::isr::*;
use crate::operating_system::*;
use crate::prelude::v1::*;
use crate::shim::*;
use crate::units::*;
use crate::utils::*;

unsafe impl Send for TaskRemoteHandle {}
impl !ISRSafe for TaskRemoteHandle {}

pub trait TaskHandle {
    fn raw_handle(&self) -> FreeRtosTaskHandle;

    /// Get the name of the current task.
    fn get_name(&self) -> Result<String, ()> {
        unsafe {
            let name_ptr = freertos_rs_task_get_name(self.raw_handle());
            let name = str_from_c_string(name_ptr);
            if let Ok(name) = name {
                return Ok(name);
            }

            Err(())
        }
    }

    /// Get the minimum amount of stack that was ever left on this task.
    fn get_stack_high_water_mark(&self) -> u32 {
        unsafe { freertos_rs_get_stack_high_water_mark(self.raw_handle()) as u32 }
    }

    /// Get an ISR safe handle.
    /// This is safe because tasks never terminate.
    fn new_isr_safe_handle(&self) -> TaskISRHandle {
        TaskISRHandle {
            task_handle: self.raw_handle(),
        }
    }
}

impl ISRSafeHandle<TaskISRHandle> for dyn TaskHandle {
    unsafe fn new_isr_safe_handle(&self) -> TaskISRHandle {
        TaskISRHandle {
            task_handle: self.raw_handle(),
        }
    }
}

/// Task's execution priority. Low priority numbers denote low priority tasks.
#[derive(Debug, Copy, Clone)]
pub struct TaskPriority(pub u8);

/// Notification to be sent to a task.
#[derive(Debug, Copy, Clone)]
pub enum TaskNotification {
    /// Send the event, unblock the task, the task's notification value isn't changed.
    NoAction,
    /// Perform a logical or with the task's notification value.
    SetBits(u32),
    /// Increment the task's notification value by one.
    Increment,
    /// Set the task's notification value to this value.
    OverwriteValue(u32),
    /// Try to set the task's notification value to this value. Succeeds
    /// only if the task has no pending notifications. Otherwise, the
    /// notification call will fail.
    SetValue(u32),
}

impl TaskNotification {
    fn to_freertos(&self) -> (u32, u8) {
        match *self {
            TaskNotification::NoAction => (0, 0),
            TaskNotification::SetBits(v) => (v, 1),
            TaskNotification::Increment => (0, 2),
            TaskNotification::OverwriteValue(v) => (v, 3),
            TaskNotification::SetValue(v) => (v, 4),
        }
    }
}

impl TaskPriority {
    fn to_freertos(&self) -> FreeRtosUBaseType {
        self.0 as FreeRtosUBaseType
    }
}

pub struct TaskSelfHandle {
    task_handle: FreeRtosTaskHandle,
}

impl<'env> !Send for TaskSelfHandle {}
impl<'env> !Sync for TaskSelfHandle {}
impl<'env> !ISRSafe for TaskSelfHandle {}

impl<'env> TaskHandle for TaskSelfHandle {
    fn raw_handle(&self) -> FreeRtosTaskHandle {
        self.task_handle
    }
}

impl TaskSelfHandle {
    /// A task can delete itself.
    /// This is unsafe, because if another task depends on our stack, or whoever spawned us still has a handle,
    /// they can hold an invalid reference. Also note that the drop methods for any objects in the current stack
    /// frame will not be called. Make sure this is the only thing left in the scope when called.
    pub unsafe fn delete(&self) -> ! {
        freertos_rs_delete_task(self.task_handle);

        // Task should be deleted by this point.
        unreachable!()
    }

    /// Take the notification and either clear the notification value or decrement it by one.
    pub fn take_notification<D: DurationTicks>(&self, clear: bool, wait_for: D) -> u32 {
        unsafe { freertos_rs_task_notify_take(if clear { 1 } else { 0 }, wait_for.to_ticks()) }
    }

    /// Wait for a notification to be posted.
    pub fn wait_for_notification<D: DurationTicks>(
        &self,
        clear_bits_enter: u32,
        clear_bits_exit: u32,
        wait_for: D,
    ) -> Result<u32, FreeRtosError> {
        let mut val = 0;
        let r = unsafe {
            freertos_rs_task_notify_wait(
                clear_bits_enter,
                clear_bits_exit,
                &mut val as *mut _,
                wait_for.to_ticks(),
            )
        };

        if r == 0 {
            Ok(val)
        } else {
            Err(FreeRtosError::Timeout)
        }
    }

    pub fn new_remote_handle(&self) -> TaskRemoteHandle {
        TaskRemoteHandle {
            task_handle: self.task_handle,
        }
    }
}

/// Handle for a FreeRTOS task
#[derive(Debug)]
pub struct TaskRemoteHandle {
    task_handle: FreeRtosTaskHandle,
}

impl TaskHandle for TaskRemoteHandle {
    fn raw_handle(&self) -> FreeRtosTaskHandle {
        self.task_handle
    }
}

impl TaskRemoteHandle {
    /// Spawn a new independent task.
    pub fn new<F>(
        _os: FreeRTOS,
        name: &str,
        stack_depth: u16,
        priority: TaskPriority,
        func: F,
    ) -> Result<TaskRemoteHandle, FreeRtosError>
    where
        F: FnOnce(&TaskSelfHandle, FreeRTOS) -> !,
        F: Send + 'static,
    {
        TaskRemoteHandle::spawn(name, stack_depth, priority, func)
    }

    /// Construct task from raw FreeRTOS handle.
    pub unsafe fn from_raw(task_handle: FreeRtosTaskHandle) -> TaskRemoteHandle {
        TaskRemoteHandle { task_handle }
    }

    unsafe fn spawn_inner(
        f: Box<dyn FnOnce(&TaskSelfHandle, FreeRTOS) -> !>,
        name: &str,
        stack_size: u16,
        priority: TaskPriority,
    ) -> Result<TaskRemoteHandle, FreeRtosError> {
        let f = Box::new(f);
        let param_ptr = &*f as *const _ as *mut _;

        let (success, task_handle) = {
            let name = name.as_bytes();
            let name_len = name.len();
            let mut task_handle = mem::zeroed::<CVoid>();

            let ret = freertos_rs_spawn_task(
                thread_start,
                param_ptr,
                name.as_ptr(),
                name_len as u8,
                stack_size,
                priority.to_freertos(),
                &mut task_handle,
            );

            (ret == 0, task_handle)
        };

        if success {
            mem::forget(f);
        } else {
            return Err(FreeRtosError::OutOfMemory);
        }

        extern "C" fn thread_start(main: *mut CVoid) -> *mut CVoid {
            unsafe {
                let b = Box::from_raw(main as *mut Box<dyn FnOnce(&TaskSelfHandle, FreeRTOS) -> !>);

                let self_handle = TaskSelfHandle {
                    task_handle: freertos_rs_get_current_task(),
                };
                let os = FreeRTOS {};

                b(&self_handle, os);
            }
        }

        Ok(TaskRemoteHandle {
            task_handle: task_handle as usize as *const _,
        })
    }

    fn spawn<F>(
        name: &str,
        stack_size: u16,
        priority: TaskPriority,
        f: F,
    ) -> Result<TaskRemoteHandle, FreeRtosError>
    where
        F: FnOnce(&TaskSelfHandle, FreeRTOS) -> !,
        F: Send + 'static,
    {
        unsafe {
            return TaskRemoteHandle::spawn_inner(Box::new(f), name, stack_size, priority);
        }
    }

    /// Forcibly set the notification value for this task.
    pub fn set_notification_value(&self, val: u32) {
        self.notify(TaskNotification::OverwriteValue(val))
    }

    /// Notify this task.
    pub fn notify(&self, notification: TaskNotification) {
        unsafe {
            let n = notification.to_freertos();
            freertos_rs_task_notify(self.raw_handle(), n.0, n.1);
        }
    }
}

pub struct TaskISRHandle {
    task_handle: FreeRtosTaskHandle,
}

impl TaskISRHandle {
    /// Notify this task from an interrupt.
    pub fn notify(
        &self,
        context: &InterruptContext,
        notification: TaskNotification,
    ) -> Result<(), FreeRtosError> {
        unsafe {
            let n = notification.to_freertos();
            let t = freertos_rs_task_notify_isr(
                self.task_handle,
                n.0,
                n.1,
                context.get_task_field_mut(),
            );
            if t != 0 {
                Err(FreeRtosError::QueueFull)
            } else {
                Ok(())
            }
        }
    }
}

#[derive(Debug)]
pub struct FreeRtosSchedulerState {
    pub tasks: Vec<FreeRtosTaskStatus>,
    pub total_run_time: u32,
}

impl fmt::Display for FreeRtosSchedulerState {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt.write_str("FreeRTOS tasks\r\n")?;

        write!(fmt, "{id: <6} | {name: <16} | {state: <9} | {priority: <8} | {stack: >10} | {cpu_abs: >10} | {cpu_rel: >4}\r\n",
               id = "ID",
               name = "Name",
               state = "State",
               priority = "Priority",
               stack = "Stack left",
               cpu_abs = "CPU",
               cpu_rel = "%"
        )?;

        for task in &self.tasks {
            write!(fmt, "{id: <6} | {name: <16} | {state: <9} | {priority: <8} | {stack: >10} | {cpu_abs: >10} | {cpu_rel: >4}\r\n",
                   id = task.task_number,
                   name = task.name,
                   state = format!("{:?}", task.task_state),
                   priority = task.current_priority.0,
                   stack = task.stack_high_water_mark,
                   cpu_abs = task.run_time_counter,
                   cpu_rel = if self.total_run_time > 0 && task.run_time_counter <= self.total_run_time {
                       let p = (((task.run_time_counter as u64) * 100) / self.total_run_time as u64) as u32;
                       let ps = if p == 0 && task.run_time_counter > 0 {
                           "<1".to_string()
                       } else {
                           p.to_string()
                       };
                       format!("{: >3}%", ps)
                   } else {
                       "-".to_string()
                   }
            )?;
        }

        if self.total_run_time > 0 {
            write!(fmt, "Total run time: {}\r\n", self.total_run_time)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct FreeRtosTaskStatus {
    pub task: TaskRemoteHandle,
    pub name: String,
    pub task_number: FreeRtosUBaseType,
    pub task_state: FreeRtosTaskState,
    pub current_priority: TaskPriority,
    pub base_priority: TaskPriority,
    pub run_time_counter: FreeRtosUnsignedLong,
    pub stack_high_water_mark: FreeRtosUnsignedShort,
}
