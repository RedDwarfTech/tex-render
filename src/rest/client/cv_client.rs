use crate::{
    model::{
        cv::{cv_gen::CvGen, cv_main::CvMainResp},
        project::tex_comp_queue::TexCompQueue,
        request::{
         proj::tex_proj_request::TexProjRequest,
         gen::render_result_request::RenderResultRequest,
        },
        template::cv_template::CvTemplate,
    },
    render::render_worker::render_impl,
};
use log::error;
use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
    Client,
};
use rust_wheel::{
    common::util::response_handler::success, config::app::app_conf_reader::get_app_config,
    model::response::api_response::ApiResponse, texhub::proj::compile_result::CompileResult
};

use std::sync::OnceLock;

pub fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(10))
            .pool_idle_timeout(std::time::Duration::from_secs(30))
            .pool_max_idle_per_host(1)
            .build()
            .expect("Failed to build reqwest client")
    })
}

pub fn http_client_sync() -> &'static reqwest::blocking::Client {
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(10))
            .pool_idle_timeout(std::time::Duration::from_secs(30))
            .pool_max_idle_per_host(20)
            .build()
            .expect("Failed to build reqwest client")
    })
}

pub async fn get_queue_cv() {
    let client = Client::new();
    let url_path = "/cv/gen/v1/pick";
    let url = format!("{}{}", get_app_config("cv.cv_api_url"), url_path);
    let response = client
        .get(url)
        .headers(construct_headers())
        .body("{}")
        .send()
        .await;
    match response {
        Ok(r) => {
            let text_response = r.text().await;
            match text_response {
                Ok(text) => {
                    let resp_result = serde_json::from_str::<ApiResponse<CvGen>>(&text);
                    match resp_result {
                        Ok(resp) => {
                            if success::<CvGen>(&resp) {
                                let queue_record: &CvGen = &resp.result;
                                if queue_record.template_id > 0 {
                                    get_template(queue_record).await;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error: cv client parse json failed,{},json:{}", e, text);
                        }
                    }
                }
                Err(e) => {
                    error!("text response error: {}", e)
                }
            }
        }
        Err(e) => error!("Error: {}", e),
    }
}

fn construct_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    let token: String = get_app_config("cv.x_access_token").to_owned();
    headers.insert("x-access-token", HeaderValue::from_str(&token).unwrap());
    headers.insert("user-id", HeaderValue::from_static("1"));
    headers.insert("app-id", HeaderValue::from_static("1"));
    headers.insert("x-request-id", HeaderValue::from_static("reqwest"));
    headers.insert("device-id", HeaderValue::from_static("reqwest"));
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers
}

pub async fn get_cv(cv_gen: &CvGen, cv_tpl: CvTemplate) {
    let client = Client::new();
    let url_path = format!("{}{}", "/cv/cv/v1/render-cv/", cv_gen.cv_id);
    let url = format!("{}{}", get_app_config("cv.cv_api_url"), url_path);
    let response = client.get(url).headers(construct_headers()).send().await;
    match response {
        Ok(r) => {
            let r: serde_json::Value = r.json().await.unwrap();
            let result = r.get("result").unwrap();
            let cv_main: CvMainResp = serde_json::from_value(result.clone()).unwrap();
            let render = render_impl(cv_gen, cv_tpl, cv_main);
            render.await;
        }
        Err(e) => error!("parse cv error: {}", e),
    }
}

pub async fn get_template(queue_gen: &CvGen) {
    let client = Client::new();
    let url_path = format!("{}{}", "/cv/tpl/v1/", queue_gen.template_id);
    let url = format!("{}{}", get_app_config("cv.cv_api_url"), url_path);
    let response = client.get(url).headers(construct_headers()).send().await;
    match response {
        Ok(r) => {
            // the return type of json() would normally be inferred
            // but I obviously don't have the schema so a generic
            // value works well enough for now
            let r: Result<ApiResponse<CvTemplate>, reqwest::Error> = r.json().await;
            if let Err(err_info) = r {
                error!("parse template info error,{}", err_info);
                return;
            }
            let cv_template: CvTemplate = r.unwrap().result;
            let cv_result = get_cv(queue_gen, cv_template);
            cv_result.await;
        }
        Err(e) => error!("get template error: {}", e),
    }
}

pub async fn update_gen_result(id: i64, file_name: &str, tex_file_name: &str) {
    let client = Client::new();
    let url_path = format!("{}", "/cv/gen/v1/result");
    let url = format!("{}{}", get_app_config("cv.cv_api_url"), url_path);
    let gen_req = RenderResultRequest {
        gen_status: 2,
        id: id,
        path: file_name.to_owned(),
        tex_file_path: tex_file_name.to_owned(),
    };
    let json_str = serde_json::to_string(&gen_req).unwrap();
    let response = client
        .put(url)
        .headers(construct_headers())
        .body(json_str)
        .send()
        .await;
    match response {
        Ok(resp) => {
            let text = resp.text().await.unwrap();
            println!("{}", text);
        }
        Err(e) => println!("update gen result error: {}", e),
    }
}

pub async fn update_queue_status(
    comp_status: i32,
    record_id: &i64,
    compile_result: Option<i32>,
) -> bool {
    let url_path = format!("{}", "/tex/project/compile/status");
    let url = format!("{}{}", get_app_config("cv.texhub_api_url"), url_path);
    let req_params: TexProjRequest = TexProjRequest {
        comp_status: comp_status,
        id: record_id.to_owned(),
        comp_result: if compile_result.is_some() {
            compile_result.unwrap()
        } else {
            CompileResult::Failure as i32
        },
    };
    let response = http_client()
        .put(&url)
        .headers(construct_headers())
        .body(serde_json::to_string(&req_params).unwrap())
        .send()
        .await;
    match response {
        Ok(r) => {
            let resp_text = r.text().await.unwrap();
            let comp_resp: Result<ApiResponse<TexCompQueue>, serde_json::Error> =
                serde_json::from_str(resp_text.as_str());
            if let Err(parse_err) = comp_resp.as_ref() {
                error!(
                    "parse response error,{}, resp text: {}, url: {}",
                    parse_err,
                    resp_text.as_str(),
                    url
                );
                return false;
            }
            if !success(&comp_resp.as_ref().unwrap()) {
                error!(
                    "update queue status error: {}",
                    serde_json::to_string(&comp_resp.unwrap()).unwrap()
                );
                return false;
            }
            return true;
        }
        Err(e) => {
            error!("update queue error: {}", e);
            return false;
        }
    }
}

pub fn update_queue_status_sync(
    comp_status: i32,
    record_id: &i64,
    compile_result: Option<i32>,
) -> bool {
    let url_path = format!("{}", "/tex/project/compile/status");
    let url = format!("{}{}", get_app_config("cv.texhub_api_url"), url_path);
    let req_params: TexProjRequest = TexProjRequest {
        comp_status: comp_status,
        id: record_id.to_owned(),
        comp_result: if compile_result.is_some() {
            compile_result.unwrap()
        } else {
            CompileResult::Failure as i32
        },
    };
    let response = http_client_sync()
        .put(&url)
        .headers(construct_headers())
        .body(serde_json::to_string(&req_params).unwrap())
        .send();
    match response {
        Ok(r) => {
            let resp_text = r.text().unwrap();
            let comp_resp: Result<ApiResponse<TexCompQueue>, serde_json::Error> =
                serde_json::from_str(resp_text.as_str());
            if let Err(parse_err) = comp_resp.as_ref() {
                error!(
                    "parse response error,{}, resp text: {}, url: {}",
                    parse_err,
                    resp_text.as_str(),
                    url
                );
                return false;
            }
            if !success(&comp_resp.as_ref().unwrap()) {
                error!(
                    "update queue status error: {}",
                    serde_json::to_string(&comp_resp.unwrap()).unwrap()
                );
                return false;
            }
            return true;
        }
        Err(e) => {
            error!("update queue error: {}", e);
            return false;
        }
    }
}