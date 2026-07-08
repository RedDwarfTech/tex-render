use log::error;
use rust_wheel::{
    common::util::response_handler::success, config::app::app_conf_reader::get_app_config,
    model::response::api_response::ApiResponse,
};

use crate::rest::client::cv_client::{construct_headers, http_client};

pub async fn update_expired_job(x_request_id: &str) {
    let url_path = "/inner-tex/queue/expire-check";
    let url = format!("{}{}", get_app_config("cv.texhub_api_url"), url_path);

    let request_body = "{\"expire_time:\": 1}";
    let response = match http_client(None)
        .post(url.clone())
        .headers(construct_headers(Some(x_request_id)))
        .body(request_body.to_string())
        .send()
        .await
    {
        Ok(r) => {
            r
        },
        Err(e) => {
            error!(
                "Error sending request to texhub: {}, url: {}, x-request-id: {}",
                e, url, x_request_id
            );
            return;
        }
    };

    // capture status and headers before consuming the body
    let status = response.status();
    let resp_headers = response.headers().clone();

    let text = match response.text().await {
        Ok(t) => t,
        Err(e) => {
            error!("text response error: {}, url: {}, status: {}", e, url, status);
            return;
        }
    };

    let resp_result = serde_json::from_str::<ApiResponse<String>>(&text);
    match resp_result {
        Ok(resp) => {
            if success::<String>(&resp) {
                
            } else {
                error!(
                    "texhub responded with failure: url: {}, status: {}, headers: {:?}, body: {}",
                    url, status, resp_headers, text
                );
            }
        }
        Err(e) => {
            error!(
                "Error: queue client parse json failed: {}, url: {}, status: {}, response_headers: {:?}, request_body: {}, response_body: {}",
                e, url, status, resp_headers, request_body, text
            );
        }
    }
}
