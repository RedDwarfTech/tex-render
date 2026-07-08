use crate::model::project::compile_app_params::generate_x_request_id;
use crate::rest::client::texhub_queue_client::update_expired_job;
use crate::util::request_context::run_with_request_id;

pub async fn get_expired_queue_task() {
    let x_request_id = generate_x_request_id();
    run_with_request_id(x_request_id.clone(), || async move {
        update_expired_job(&x_request_id).await;
        // make other task could be invoke
        tokio::task::yield_now().await;
    })
    .await;
}

/**
 * this task check the compile queue expired task
 * the compile that takes too long time exceed the limit and change the status to exceed
 */
pub async fn check_expired_queue_task() {
        let check_result = get_expired_queue_task();
        check_result.await;
}
