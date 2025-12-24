use crate::{
    model::{cv::cv_main::CvMainResp, request::cv::render_handle_request::RenderHandleRequest},
    render::cv::handler::template_handler::TemplateHandler,
};

pub struct ModerncvHandler1 {}

impl TemplateHandler for ModerncvHandler1 {
    fn handle_request(
        &self,
        request: RenderHandleRequest,
        _main: &CvMainResp,
    ) -> Result<(), &'static str> {
        if request.template_code == "moderncv" {
            println!("ConcreteHandler1 handle request: {}", request.template_code);
            //modern.gen_cv_start(&request);
        }
        Ok(())
    }

    fn _set_next(&mut self, _handler: Box<dyn TemplateHandler>) {}
}
