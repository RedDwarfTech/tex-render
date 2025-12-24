use crate::rest::client::cv_client::get_queue_cv;
use std::time::Duration;
use tokio::time;

pub async fn _check_tpl() {
    let cv = get_queue_cv();
    cv.await;
}

pub async fn _check_tpl_task() {
    let mut interval = time::interval(Duration::from_millis(15000));
    loop {
        interval.tick().await;
        let check_result = _check_tpl();
        check_result.await;
    }
}
