use crate::rest::client::texhub_queue_client::update_expired_job;
use std::time::Duration;
use log::{info, warn};
use tokio::time;

pub async fn get_expired_queue_task() {
    warn!("trigger check expired compile queue task");
    let cv = update_expired_job();
    cv.await;
    // make other task could be invoke
    tokio::task::yield_now().await;
}

/**
 * this task check the compile queue expired task
 * the compile that takes too long time exceed the limit and change the status to exceed
 */
pub async fn check_expired_queue_task() {
    let mut interval = time::interval(Duration::from_millis(15000));
    loop {
        interval.tick().await;
        let check_result = get_expired_queue_task();
        check_result.await;
    }
}
