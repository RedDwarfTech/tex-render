use crate::{model::user::tex_user_config::TexUserConfig, rest::client::cv_client::http_client};
use log::error;
use rust_wheel::{
    config::app::app_conf_reader::get_app_config, model::response::api_response::ApiResponse,
};

pub async fn get_one_user_config(uid: i64, key: &str) -> Option<TexUserConfig> {
    let url_path = format!(
        "{}{}{}{}",
        "/inner-tex/appconf/user-one-config?user_id=", uid, "&key=", key
    );
    let url = format!("{}{}", get_app_config("cv.texhub_api_url"), url_path);
    // Send request and bail out early on error to keep nesting shallow.
    let resp = match http_client().get(url.clone()).send().await {
        Ok(r) => r,
        Err(e) => {
            error!("get user conf error: {}, url: {}", e, url);
            return None;
        }
    };

    // Read body as text so we can log raw response for debugging, then parse JSON.
    let body_text = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            error!("read response body failed: {}, url: {}", e, url);
            return None;
        }
    };
    match serde_json::from_str::<ApiResponse<TexUserConfig>>(&body_text) {
        Ok(api_resp) => Some(api_resp.result),
        Err(e) => {
            error!(
                "parse user config info error: {}, url: {}, body: {}",
                e, url, body_text
            );
            None
        }
    }
}
