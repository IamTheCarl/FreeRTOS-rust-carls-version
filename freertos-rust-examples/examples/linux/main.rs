use freertos_rust::*;
use std::sync::Arc;

#[global_allocator]
static GLOBAL: FreeRtosAllocator = FreeRtosAllocator;

fn main() {
    let x = Box::new(15);
    println!("Boxed int '{}' (allocator test)", x);

    unsafe {
        FREERTOS_HOOKS.set_on_assert(|| println!("Assert hook called"));
    }

    println!("Starting scheduler");
    FreeRTOS::start_scheduler(|os| {
        //println!("Calling assert ...");
        //FreeRTOS::invoke_assert();

        let value = Arc::new(os.new_mutex(0).unwrap());

        {
            let value = value.clone();

            println!("Starting FreeRTOS app ...");
            os.new_task("A", 128, TaskPriority(2), move |_self_handle, os| loop {
                {
                    let mut value = value.lock(Duration::infinite()).unwrap();
                    *value += 1;
                    println!("A: {}", *value);
                }
                os.delay(Duration::ms(1000));
            })
            .unwrap();
        }

        os.new_task("B", 128, TaskPriority(3), move |_self_handle, os| loop {
            // Error shows up on this line "TaskSelfHandle is not sync"

            {
                let mut value = value.lock(Duration::infinite()).unwrap();
                *value += 1;
                println!("B: {}", *value);
            }
            os.delay(Duration::ms(1000));
        })
        .unwrap();

        println!("Task registered");
        //let free = freertos_rs_xPortGetFreeHeapSize();
        // println!("Free Memory: {}!", free);
    });
}

#[test]
fn many_boxes() {
    init_allocator();
    println!("many_boxes... ");
    for i in 0..10 {
        // .. HEAP_SIZE
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
    println!("[ok]");
}
