use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref INIT: Mutex<()> = {
        init_tracing_subscriber();
        Mutex::new(())
    };
}

fn init_tracing_subscriber() {
    println!("logger init");
    tracing_subscriber::fmt::init();
    tracing::info!("logger hello");
}

pub(crate) fn init_log_once() {
    let lock = INIT.lock().unwrap();
    drop(lock);
}
