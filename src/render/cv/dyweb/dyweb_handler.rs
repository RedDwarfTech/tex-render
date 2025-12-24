use std::collections::HashMap;

use crate::{
    model::{cv::cv_main::CvMainResp, request::cv::render_handle_request::RenderHandleRequest},
    render::cv::{
        handler::template_handler::TemplateHandler, cv_render::CvRender, dyweb::dyweb_cv_gen_impl::DywebCvGenImpl,
    },
};

pub struct DywebHandler {
    pub next: Option<Box<dyn TemplateHandler>>,
}

impl TemplateHandler for DywebHandler {
    fn handle_request(
        &self,
        request: RenderHandleRequest,
        cv_main: &CvMainResp,
    ) -> Result<(), &'static str> {
        if request.template_code == "dyweb" {
            println!("Dyweb handle request: {}", request.template_code);
            let mut cv_map: HashMap<i32, &String> = HashMap::new();
            let modern = DywebCvGenImpl {};
            let start = modern.gen_cv_start(&request);
            let edu = modern.gen_edu(&request.file_path, cv_main);
            let work = modern.gen_work(cv_main);
            let skill = modern.gen_skill(cv_main);
            let project = modern.gen_project(cv_main);
            let lang = modern.gen_lang(cv_main);
            cv_map.insert(2, &edu);
            cv_map.insert(3, &skill);
            cv_map.insert(4, &work);
            cv_map.insert(5, &project);
            cv_map.insert(6, &lang);
            let mut content = String::from(start);
            content.push_str(cv_map.get(&2).unwrap());
            content.push_str(cv_map.get(&3).unwrap());
            content.push_str(cv_map.get(&4).unwrap());
            content.push_str(cv_map.get(&5).unwrap());
            content.push_str(cv_map.get(&6).unwrap());
            modern.gen_cv_end(&request.file_path, content);
            Ok(())
        } else {
            match &self.next {
                Some(next_handler) => next_handler.handle_request(request, cv_main),
                None => Err("No handler can handle this request."),
            }
        }
    }

    fn _set_next(&mut self, handler: Box<dyn TemplateHandler>) {
        self.next.replace(handler);
    }
}
