use actix_web::App;
use actix_web::HttpServer;
use common::request_id_middleware::RequestIdMiddleware;
use controller::tex::tex_controller;
use log::{error, info};
use task::app_init::initial_task;
use task::compile_task_consumer::consume_redis_stream;

use crate::controller::monitor::health_controller;
use crate::controller::proj::proj_controller;

mod common;
mod controller;
mod model;
mod render;
mod rest;
mod service;
mod task;
mod util;

#[actix_web::main]
async fn main() {
    if let Err(e) = initial_task().await {
        error!("initial task failed, {}", e);
        return;
    }
    let result = actix_main().await;
    match result {
        Ok(_) => {
            info!("start the server success")
        }
        Err(e) => {
            error!("start the actix failed,{}", e)
        }
    }
}

async fn actix_main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap(RequestIdMiddleware)
            .configure(tex_controller::config)
            .configure(health_controller::config)
            .configure(proj_controller::config)
    })
    .bind(("0.0.0.0", 8001))?
    .workers(3)
    .run()
    .await
}
