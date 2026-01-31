use super::{
    compile_task_consumer::consume_redis_stream,
    texhub::compile::check_expire_compile_task::check_expired_queue_task,
};
use log::{error, info};
use tokio::spawn;
use tokio_cron_scheduler::{Job, JobScheduler};

pub fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("log")?;
    log4rs::init_file("log4rs.yaml", Default::default())?;
    info!("log4rs initialized successfully");
    Ok(())
}

pub async fn initial_task() -> Result<(), Box<dyn std::error::Error>> {
    if let Err(e) = init_logging() {
        error!("Failed to initialize logging: {}", e);
        return Ok(());
    }

    let mut sched = JobScheduler::new().await?;

    // Add async job
    sched
        .add(Job::new_async("1/7 * * * * *", |uuid, mut l| {
            Box::pin(async move {
                info!("I run async every 7 seconds");
                check_expired_queue_task().await;
            })
        })?)
        .await?;

          // Start the scheduler
    sched.start().await?;
    
    spawn(async {
        consume_redis_stream().await;
    });

    Ok(())
}
