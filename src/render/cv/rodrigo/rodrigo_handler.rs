// use std::collections::HashMap;

use crate::{
    model::{cv::cv_main::CvMainResp, request::cv::render_handle_request::RenderHandleRequest},
    render::cv::{
        handler::template_handler::TemplateHandler, cv_render::CvRender, rodrigo::rodrigo_cv_gen_impl::RodrigoCvGenImpl,
    },
};

pub struct RodrigoHandler {
    pub next: Option<Box<dyn TemplateHandler>>,
}

impl TemplateHandler for RodrigoHandler {
    fn handle_request(
        &self,
        request: RenderHandleRequest,
        cv_main: &CvMainResp,
    ) -> Result<(), &'static str> {
        if request.template_code == "rodrigo" {
            println!("rodrigo handle request: {}", request.template_code);
            // let cv_map: HashMap<i32, &String> = HashMap::new();
            let modern = RodrigoCvGenImpl {};
            let start = modern.gen_cv_start(&request);
            let _edu = modern.gen_edu(&request.file_path, cv_main);
            let _work = modern.gen_work(cv_main);
            let _skill = modern.gen_skill(cv_main);
            let _project = modern.gen_project(cv_main);
            //cv_map.insert(2, &edu);
            //cv_map.insert(3, &work);
            //cv_map.insert(4, &skill);
            //cv_map.insert(5, &project);
            let order_array:Vec<i32> = cv_main
                .item_order
                .split(",")
                .map(|s| s.parse().unwrap())
                .collect();
            let content = String::from(start);
            // content.push_str(cv_map.get(&1).unwrap());
            for item_id in order_array {
                if item_id != 1 {
                    //content.push_str(cv_map.get(&item_id).unwrap());
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
