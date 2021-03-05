use freertos_rust::*;

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

        println!("Starting FreeRTOS app ...");
        os.new_task("parent", 128, TaskPriority(2), |self_handle, os| {
            let a = os.new_mutex(0).unwrap();

            self_handle
                .spawn_child("child", 128, TaskPriority(3), |_self_handle, _os| loop {
                    {
                        let mut a = a.lock(Duration::infinite()).unwrap();
                        *a += 1;
                        println!("Child A: {}", *a);
                    }
                    os.delay(Duration::ms(1000));
                })
                .unwrap();

            loop {
                {
                    let mut a = a.lock(Duration::infinite()).unwrap();
                    *a += 1;
                    println!("Parent A: {}", *a);
                }
                os.delay(Duration::ms(1000));
            }
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
