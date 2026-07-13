use crate::model::project::tex_comp_queue::TexCompQueue;
use crate::model::request::queue::queue_start_time_req::QueueStartTimeReq;
use log::error;
use rust_wheel::{
    common::util::response_handler::success,
    config::app::app_conf_reader::get_app_config,
    model::response::api_response::ApiResponse,
};

use crate::rest::client::cv_client::{construct_headers, http_client, http_client_sync};

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
        Ok(r) => r,
        Err(e) => {
            error!(
                "Error sending request to texhub: {}, url: {}, x-request-id: {}",
                e, url, x_request_id
            );
            return;
        }
    };

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
            if !success::<String>(&resp) {
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

pub async fn update_queue_start_time(record_id: &i64, start_time: i64, x_request_id: &str) {
    let url_path = "/inner-tex/queue/start-time";
    let url = format!("{}{}", get_app_config("cv.texhub_api_url"), url_path);
    let request_body = QueueStartTimeReq {
        id: *record_id,
        start_time,
    };
    let body = match serde_json::to_string(&request_body) {
        Ok(body) => body,
        Err(e) => {
            error!(
                "serialize queue start time request failed: {}, qid: {}, x-request-id: {}",
                e, record_id, x_request_id
            );
            return;
        }
    };

    let response = match http_client(None)
        .put(url.clone())
        .headers(construct_headers(Some(x_request_id)))
        .body(body.clone())
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            error!(
                "update queue start time request failed: {}, url: {}, qid: {}, x-request-id: {}",
                e, url, record_id, x_request_id
            );
            return;
        }
    };

    let status = response.status();
    let resp_headers = response.headers().clone();
    let text = match response.text().await {
        Ok(t) => t,
        Err(e) => {
            error!(
                "update queue start time read response failed: {}, url: {}, qid: {}, status: {}, x-request-id: {}",
                e, url, record_id, status, x_request_id
            );
            return;
        }
    };

    let resp_result = serde_json::from_str::<ApiResponse<TexCompQueue>>(&text);
    match resp_result {
        Ok(resp) => {
            if !success::<TexCompQueue>(&resp) {
                error!(
                    "update queue start time failed: url: {}, qid: {}, status: {}, headers: {:?}, body: {}, x-request-id: {}",
                    url, record_id, status, resp_headers, text, x_request_id
                );
            }
        }
        Err(e) => {
            error!(
                "update queue start time parse response failed: {}, url: {}, qid: {}, status: {}, body: {}, x-request-id: {}",
                e, url, record_id, status, text, x_request_id
            );
        }
    }
}

pub fn update_queue_start_time_sync(record_id: &i64, start_time: i64, x_request_id: &str) {
    let url_path = "/inner-tex/queue/start-time";
    let url = format!("{}{}", get_app_config("cv.texhub_api_url"), url_path);
    let request_body = QueueStartTimeReq {
        id: *record_id,
        start_time,
    };
    let body = match serde_json::to_string(&request_body) {
        Ok(body) => body,
        Err(e) => {
            error!(
                "serialize queue start time request failed: {}, qid: {}, x-request-id: {}",
                e, record_id, x_request_id
            );
            return;
        }
    };

    let response = match http_client_sync()
        .put(url.clone())
        .headers(construct_headers(Some(x_request_id)))
        .body(body.clone())
        .send()
    {
        Ok(r) => r,
        Err(e) => {
            error!(
                "update queue start time request failed: {}, url: {}, qid: {}, x-request-id: {}",
                e, url, record_id, x_request_id
            );
            return;
        }
    };

    let status = response.status();
    let resp_headers = response.headers().clone();
    let text = match response.text() {
        Ok(t) => t,
        Err(e) => {
            error!(
                "update queue start time read response failed: {}, url: {}, qid: {}, status: {}, x-request-id: {}",
                e, url, record_id, status, x_request_id
            );
            return;
        }
    };

    let resp_result = serde_json::from_str::<ApiResponse<TexCompQueue>>(&text);
    match resp_result {
        Ok(resp) => {
            if !success::<TexCompQueue>(&resp) {
                error!(
                    "update queue start time failed: url: {}, qid: {}, status: {}, headers: {:?}, body: {}, x-request-id: {}",
                    url, record_id, status, resp_headers, text, x_request_id
                );
            }
        }
        Err(e) => {
            error!(
                "update queue start time parse response failed: {}, url: {}, qid: {}, status: {}, body: {}, x-request-id: {}",
                e, url, record_id, status, text, x_request_id
            );
        }
    }
}
