use crate::model::project::compile_app_params::CompileAppParams;
use crate::model::project::tex_file_compile_status::TeXFileCompileStatus;
use crate::render::render_worker::{render_texhub_project, render_texhub_project_sse};
use crate::render::texhub::pipeline::pipeline_render_works::render_texhub_project_pipeline;
use crate::rest::client::cv_client::{update_queue_status, update_queue_status_sync};
use crate::util::request_context::{run_with_request_id, with_request_id};
use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::{web, HttpResponse, Responder};
use log::{error, warn};
use rust_wheel::common::util::net::sse_stream::SseStream;
use rust_wheel::model::response::api_response::ApiResponse;
use rust_wheel::texhub::proj::compile_result::CompileResult;
use tokio::sync::mpsc::UnboundedSender;
use tokio::{sync::mpsc::UnboundedReceiver, task};

pub async fn compile_tex(params: web::Json<CompileAppParams>) -> impl Responder {
    run_with_request_id(params.x_request_id.clone(), || async {
        let resp = render_texhub_project(&params).await;
        let res = ApiResponse {
            result: resp,
            ..Default::default()
        };
        HttpResponse::Ok().json(res)
    })
    .await
}

pub async fn compile_tex_sse(params: web::Query<CompileAppParams>) -> HttpResponse {
    let params = params.into_inner();
    let x_request_id = params.x_request_id.clone();
    let (tx, rx): (UnboundedSender<String>, UnboundedReceiver<String>) =
        tokio::sync::mpsc::unbounded_channel();
    task::spawn(async move {
        run_with_request_id(x_request_id, || async move {
            let output = render_texhub_project_sse(&params, tx).await;
            if let Err(re) = output {
                error!("Failed to compile, {}", re);
            }
        })
        .await
    });
    HttpResponse::Ok()
        .insert_header(CacheControl(vec![CacheDirective::NoCache]))
        .content_type("text/event-stream")
        .streaming(SseStream {
            receiver: Some(rx),
        })
}

pub async fn compile_tex_from_mq(params: CompileAppParams) {
    let x_request_id = params.x_request_id.clone();
    task::spawn_blocking(move || {
        with_request_id(&x_request_id, || {
            let compile_result = render_texhub_project_pipeline(&params);
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(update_queue_compile_result(params, compile_result));
        });
    });
}

pub async fn update_queue_compile_result(
    params_arc: CompileAppParams,
    compile_result: Option<CompileResult>,
) {
    if compile_result.is_none() {
        warn!("compile result is none, params:{:?}", params_arc);
    }
    let u_result = update_queue_status(
        TeXFileCompileStatus::Compiled as i32,
        &params_arc.qid,
        Some(compile_result.unwrap() as i32),
        &params_arc.x_request_id,
    )
    .await;
    if !u_result {
        error!("Failed to update result status, params: {:?}", &params_arc)
    }
}

pub fn update_queue_compile_result_sync(
    params_arc: CompileAppParams,
    compile_result: Option<CompileResult>,
) {
    if compile_result.is_none() {
        warn!("compile result is none, params:{:?}", params_arc);
    }
    let u_result = update_queue_status_sync(
        TeXFileCompileStatus::Compiled as i32,
        &params_arc.qid,
        Some(compile_result.unwrap() as i32),
        &params_arc.x_request_id,
    );
    if !u_result {
        error!("Failed to update result status, params: {:?}", &params_arc)
    }
}

pub async fn update_queue_compile_status(
    params_arc: CompileAppParams,
    compile_status: TeXFileCompileStatus,
) {
    let u_result = update_queue_status(
        compile_status as i32,
        &params_arc.qid,
        Some(CompileResult::Unknown as i32),
        &params_arc.x_request_id,
    )
    .await;
    if !u_result {
        error!("Failed to update queue status, params: {:?}", &params_arc)
    }
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/render/compile/v1")
            .route("/project", web::post().to(compile_tex))
            .route("/project/sse", web::get().to(compile_tex_sse)),
    );
}
