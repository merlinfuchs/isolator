use std::sync::Arc;
use crate::GlobalState;
use std::thread;
use std::time::Duration;

pub fn thread_pool_manager() {}

// enforces the cpu time across cpu intensive wakeups
pub fn cpu_time_manager(state: Arc<GlobalState>) {
    loop {
        thread::sleep(Duration::from_millis(1));

        let mut runtimes_guard = state.runtimes.lock().unwrap();
        let runtimes = &mut *runtimes_guard;

        for (_, runtime) in runtimes {
            let mut time_table_guard = runtime.time_table.lock().unwrap();
            let time_table = &mut *time_table_guard;

            if let Some(current_wakeup) = time_table.current_wakeup {
                if let Some(cpu_time_limit) = time_table.cpu_time_limit {
                    let cpu_elapsed = time_table.cpu_time + current_wakeup.elapsed();

                    let mut isolate_guard = runtime.isolate_handle.lock().unwrap();
                    let isolate_handle = &mut *isolate_guard;

                    if let Some(isolate_handle) = isolate_handle {
                        if cpu_elapsed > cpu_time_limit {
                            isolate_handle.terminate_execution();
                        }
                    }
                }
            }
        }
    }
}