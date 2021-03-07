use crate::base::*;
use crate::delays::*;
use crate::isr::*;
use crate::mutex::*;
use crate::prelude::v1::*;
use crate::queue::*;
use crate::semaphore::*;
use crate::shim::*;
use crate::task::*;
use crate::timers::*;
use crate::units::*;
use crate::utils::*;

/// A handle to the operating system to prevent calling non-ISR safe functions from ISRs.
#[derive(Clone, Copy)]
pub struct FreeRTOS {}

impl !ISRSafe for FreeRTOS {}

impl FreeRTOS {
    pub fn start_scheduler<F: FnOnce(FreeRTOS)>(setup_function: F) -> ! {
        setup_function(FreeRTOS {});

        unsafe {
            freertos_rs_vTaskStartScheduler();
        }
    }

    pub unsafe fn assume_init() -> FreeRTOS {
        FreeRTOS {}
    }

    /// Prepare a builder object for the new task.
    pub fn new_task<F>(
        &self,
        name: &str,
        stack_depth: u16,
        priority: TaskPriority,
        func: F,
    ) -> Result<TaskRemoteHandle, FreeRtosError>
    where
        F: FnOnce(&TaskSelfHandle, FreeRTOS) -> !,
        F: Send + 'static,
    {
        TaskRemoteHandle::new(self.clone(), name, stack_depth, priority, func)
    }

    /// Create a new delay helper, marking the current time as the start of the
    /// next measurement.
    pub fn new_delay(&self) -> TaskDelay {
        TaskDelay::new(self.clone())
    }

    /// Create a new timer with the set period.
    pub fn new_periodic_delay<D: DurationTicks>(&self, period: D) -> TaskDelayPeriodic {
        TaskDelayPeriodic::new(self.clone(), period)
    }

    /// Create a new timer builder.
    pub fn new_timer<D: DurationTicks>(&self, period: D) -> TimerBuilder<D> {
        TimerBuilder::new(self.clone(), period)
    }

    pub fn new_queue<T: Copy>(&self, max_size: usize) -> Result<Queue<T>, FreeRtosError> {
        Queue::new(self.clone(), max_size)
    }

    /// Create a new binary semaphore
    pub fn new_binary_semaphore(&self) -> Result<BinarySemaphore, FreeRtosError> {
        BinarySemaphore::new(self.clone())
    }

    /// Create a new counting semaphore
    pub fn new_counting_semaphore(
        &self,
        max: u32,
        initial: u32,
    ) -> Result<CountingSemaphore, FreeRtosError> {
        CountingSemaphore::new(self.clone(), max, initial)
    }

    /// Create a new mutex with the given inner value
    pub fn new_mutex<T>(&self, t: T) -> Result<Mutex<T>, FreeRtosError> {
        Mutex::new(self.clone(), t)
    }

    /// Create a new recursive mutex with the given inner value
    pub fn new_recursive_mutex<T>(&self, t: T) -> Result<RecursiveMutex<T>, FreeRtosError> {
        RecursiveMutex::new(self.clone(), t)
    }

    // Should only be used for testing purpose!
    pub fn invoke_assert() {
        unsafe {
            freertos_rs_invoke_configASSERT();
        }
    }

    /// Delay the execution of the current task.
    pub fn delay<D: DurationTicks>(&self, delay: D) {
        unsafe {
            freertos_rs_vTaskDelay(delay.to_ticks());
        }
    }

    pub fn get_tick_count(&self) -> FreeRtosTickType {
        unsafe { freertos_rs_xTaskGetTickCount() }
    }

    pub fn get_tick_count_duration(&self) -> Duration {
        Duration::ticks(self.get_tick_count())
    }

    pub fn get_number_of_tasks(&self) -> usize {
        unsafe { freertos_rs_get_number_of_tasks() as usize }
    }

    pub fn get_all_tasks(&self, tasks_len: Option<usize>) -> FreeRtosSchedulerState {
        let tasks_len = tasks_len.unwrap_or(self.get_number_of_tasks());
        let mut tasks = Vec::with_capacity(tasks_len as usize);
        let mut total_run_time = 0;

        unsafe {
            let filled = freertos_rs_get_system_state(
                tasks.as_mut_ptr(),
                tasks_len as FreeRtosUBaseType,
                &mut total_run_time,
            );
            tasks.set_len(filled as usize);
        }

        let tasks = tasks
            .into_iter()
            .map(|t| FreeRtosTaskStatus {
                task: unsafe { TaskRemoteHandle::from_raw(t.handle) },
                name: unsafe { str_from_c_string(t.task_name) }
                    .unwrap_or_else(|_| String::from("?")),
                task_number: t.task_number,
                task_state: t.task_state,
                current_priority: TaskPriority(t.current_priority as u8),
                base_priority: TaskPriority(t.base_priority as u8),
                run_time_counter: t.run_time_counter,
                stack_high_water_mark: t.stack_high_water_mark,
            })
            .collect();

        FreeRtosSchedulerState {
            tasks: tasks,
            total_run_time: total_run_time,
        }
    }
}
