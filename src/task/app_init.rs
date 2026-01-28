use super::{
    compile_task_consumer::consume_redis_stream,
    texhub::compile::check_expire_compile_task::check_expired_queue_task,
};
use log::{error, info};
use tokio::spawn;

pub fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("log")?;
    log4rs::init_file("log4rs.yaml", Default::default())?;
    info!("log4rs initialized successfully");
    Ok(())
}

pub fn initial_task() {
    if let Err(e) = init_logging() {
        error!("Failed to initialize logging: {}", e);
        return;
    }

    spawn(async {
        consume_redis_stream().await;
    });
    
    spawn(async {
        check_expired_queue_task().await;
    });
}
