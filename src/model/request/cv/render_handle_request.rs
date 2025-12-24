use serde::Deserialize;
use serde::Serialize;

use crate::model::cv::cv_main::CvMainResp;

#[derive(Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct RenderHandleRequest<'r> {
    pub template_code: String,
    pub file_path: &'r str,
    pub cv_main: CvMainResp
}