use std::{
    fs::{self, OpenOptions},
    io::Write,
};

use crate::{render::cv::cv_render::CvRender, model::{request::cv::render_handle_request::RenderHandleRequest, cv::cv_main::CvMainResp}};

pub struct ModernCvGenImpl {}

impl ModernCvGenImpl {
    
}
/**
 * the moderncv fonts issue
 * https://apple.stackexchange.com/questions/205639/to-install-latin-modern-math-font-in-os-x
 */
impl CvRender for ModernCvGenImpl {
    fn gen_cv_start(&self, request: &RenderHandleRequest) -> String {
        if fs::metadata(request.file_path).is_ok() {
            fs::remove_file(request.file_path).unwrap();
        }
        let message = format!(
            "{}{}{}{}",
            "\\documentclass[11pt,a4paper,sans]{moderncv}\n",
            "\\moderncvstyle{classic}\n",
            "\\name{John}{Doe}\n",
            "\\begin{document}\n"
        );
        return message;
    }

    fn _gen_summary(&self, _file_path: &str, _tpl_code: String) ->String {
        todo!()
    }

    fn _gen_section(&self, _file_path: &str) -> bool {
        todo!()
    }

    fn gen_cv_end(&self, file_path: &str,_tpl_code: String) -> bool {
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(file_path)
            .unwrap();
        let message = format!("{}{}", "\n", "\\end{document}");
        file.write_all(message.as_bytes()).unwrap();
        return true;
    }

    fn gen_edu(&self,_file_path: &str, _cv_main: &CvMainResp) -> String {
        let message = format!("{}", "\\section{Education}\n");
        return message;
    }

    fn gen_work(&self,_cv_main: &CvMainResp) -> String {
        todo!()
    }

    fn gen_skill(&self,_cv_main: &CvMainResp) -> String {
        todo!()
    }

    fn gen_project(&self,_cv_main: &CvMainResp) -> String {
        todo!()
    }

    fn gen_lang(&self, _cv_main: &CvMainResp) -> String {
        todo!()
    }
}
