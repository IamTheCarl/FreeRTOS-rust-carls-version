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
        //FreeRtosUtils::invoke_assert();

        println!("Starting FreeRTOS app ...");
        os.new_task()
            .name("hello")
            .stack_size(128)
            .priority(TaskPriority(2))
            .start(|_self_handle, os| {
                let mut i = 0;
                loop {
                    println!("Hello from Task! {}", i);
                    os.delay(Duration::ms(1000));
                    i = i + 1;
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
