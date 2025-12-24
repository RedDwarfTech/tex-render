use crate::model::{
    cv::cv_main::CvMainResp, request::cv::render_handle_request::RenderHandleRequest,
};

pub trait TemplateHandler: Send + Sync {
    fn handle_request(
        &self,
        request: RenderHandleRequest,
        cv_main: &CvMainResp,
    ) -> Result<(), &'static str>;
    fn _set_next(&mut self, handler: Box<dyn TemplateHandler>);
}
