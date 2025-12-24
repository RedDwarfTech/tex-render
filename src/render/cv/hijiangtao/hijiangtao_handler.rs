use std::collections::HashMap;

use crate::{
    model::{cv::cv_main::CvMainResp, request::cv::render_handle_request::RenderHandleRequest},
    render::cv::{
        handler::template_handler::TemplateHandler, hijiangtao::hijiangtao_cv_gen_impl::HijiangtaoCvGenImpl, cv_render::CvRender,
    },
};

pub struct HijiangtaoHandler {
    pub next: Option<Box<dyn TemplateHandler>>,
}

impl TemplateHandler for HijiangtaoHandler {
    fn handle_request(
        &self,
        request: RenderHandleRequest,
        cv_main: &CvMainResp,
    ) -> Result<(), &'static str> {
        if request.template_code == "hijiangtao" {
            println!("hijiangtao handle request: {}", request.template_code);
            let mut cv_map: HashMap<i32, &String> = HashMap::new();
            let modern = HijiangtaoCvGenImpl {};
            let start = modern.gen_cv_start(&request);
            let edu = modern.gen_edu(&request.file_path, cv_main);
            let work = modern.gen_work(cv_main);
            let skill = modern.gen_skill(cv_main);
            let project = modern.gen_project(cv_main);
            let lang = modern.gen_lang(cv_main);
            cv_map.insert(2, &edu);
            cv_map.insert(3, &work);
            cv_map.insert(4, &skill);
            cv_map.insert(5, &project);
            cv_map.insert(6, &lang);
            let order_array:Vec<i32> = cv_main
                .item_order
                .split(",")
                .map(|s| s.parse().unwrap())
                .collect();
            let mut content = String::from(start);
            for item_id in order_array {
                if item_id != 1 {
                    content.push_str(cv_map.get(&item_id).unwrap());
                }
            }
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
