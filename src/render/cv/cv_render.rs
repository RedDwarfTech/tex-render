use crate::model::{request::cv::render_handle_request::RenderHandleRequest, cv::cv_main::CvMainResp};

pub trait CvRender {
    fn gen_cv_start(&self,request: &RenderHandleRequest) ->String;
    fn _gen_summary(&self,file_path: &str,tpl_code: String) -> String;
    fn gen_edu(&self,file_path: &str, cv_main: &CvMainResp) -> String;
    fn gen_work(&self, cv_main: &CvMainResp) -> String;
    fn gen_skill(&self, cv_main: &CvMainResp) -> String;
    fn gen_project(&self, cv_main: &CvMainResp) -> String;
    fn gen_lang(&self, cv_main: &CvMainResp) -> String;
    fn _gen_section(&self,file_path: &str) -> bool;
    fn gen_cv_end(&self,file_path: &str,tpl_code: String) -> bool;
}
