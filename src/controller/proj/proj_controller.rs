use actix_web::{web, HttpResponse};
use rust_wheel::common::wrapper::actix_http_resp::box_actix_rest_response;

use crate::{
    model::request::proj::{
        get_pdf_pos_params::GetPdfPosParams, get_src_pos_params::GetSrcPosParams,
    },
    service::project_service::{get_pdf_pos, get_src_pos},
};

async fn get_pdf_position(form: web::Query<GetPdfPosParams>) -> HttpResponse {
    let pos = get_pdf_pos(&form.0);
    box_actix_rest_response(pos)
}

async fn get_src_position(form: web::Query<GetSrcPosParams>) -> HttpResponse {
    let pos = get_src_pos(&form.0);
    box_actix_rest_response(pos)
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/tex/project")
            .route("/pos/pdf", web::get().to(get_pdf_position))
            .route("/pos/src", web::get().to(get_src_position)),
    );
}
