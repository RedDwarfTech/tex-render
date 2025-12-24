use super::zheyuye_cv_util::{
    get_zheyuye_edu_str, get_zheyuye_lang_str, get_zheyuye_project_str, get_zheyuye_skill_str,
    get_zheyuye_work_str,
};
use crate::{
    model::{cv::cv_main::CvMainResp, request::cv::render_handle_request::RenderHandleRequest},
    render::cv::cv_render::CvRender,
};
use std::{
    fs::{self, OpenOptions},
    io::Write,
};

pub struct ZheyuyeCvGenImpl {}

impl ZheyuyeCvGenImpl {}

impl CvRender for ZheyuyeCvGenImpl {
    fn gen_cv_start(&self, request: &RenderHandleRequest) -> String {
        if fs::metadata(request.file_path).is_ok() {
            fs::remove_file(request.file_path).unwrap();
        }
        let cv_main = &request.cv_main;
        let message = format!(
            "{}{}{}{}{}{}{}{}{}{}{}{}",
            "\\documentclass{zheyuyesetting}\n\n",
            "\\begin{document}\n\n",
            "\\name{",
            cv_main.employee_name.clone().unwrap(),
            "}\n",
            "\\contactInfo{",
            cv_main.phone.clone().unwrap(),
            "}{",
            cv_main.email.clone().unwrap(),
            "}{",
            cv_main.github.clone().unwrap_or_default(),
            "}\n\n"
        );
        return message;
    }

    fn _gen_summary(&self, _file_path: &str, tpl_code: String) -> String {
        let message = format!("{}", tpl_code);
        return message;
    }

    fn _gen_section(&self, _file_path: &str) -> bool {
        todo!()
    }

    fn gen_cv_end(&self, file_path: &str, tpl_code: String) -> bool {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(file_path)
            .unwrap();
        let message = format!("{}{}{}", tpl_code, "\n", "\\end{document}");
        file.write_all(message.as_bytes()).unwrap();
        return true;
    }

    fn gen_edu(&self, _file_path: &str, cv_main: &CvMainResp) -> String {
        let edu_items = get_zheyuye_edu_str(&cv_main.edu);
        let message = format!(
            "{}{}",
            "\\logosection{\\faGraduationCap}{教育经历}\n\n", edu_items
        );
        return message;
    }

    fn gen_work(&self, cv_main: &CvMainResp) -> String {
        let work_items = get_zheyuye_work_str(&cv_main.work);
        let message = format!(
            "{}{}",
            "\\logosection{\\faSuitcase}{工作经历}\n\n", work_items
        );
        return message;
    }

    fn gen_skill(&self, cv_main: &CvMainResp) -> String {
        let work_items = get_zheyuye_skill_str(&cv_main.skills);
        if work_items.is_empty() {
            return "".to_owned();
        }
        let message = format!("{}{}", "\\logosection{\\faCogs}{专业技能}\n\n", work_items);
        return message;
    }

    fn gen_project(&self, cv_main: &CvMainResp) -> String {
        let work_items = get_zheyuye_project_str(&cv_main.projects);
        let message = format!(
            "{}{}",
            "\\logosection{\\faWrench}{项目经历}\n\n", work_items
        );
        return message;
    }

    fn gen_lang(&self, cv_main: &CvMainResp) -> String {
        match &cv_main.langs {
            Some(langs) => {
                if langs.len() == 0 {
                    return "".to_owned();
                }
                let work_items = get_zheyuye_lang_str(&cv_main.langs);
                let message = format!(
                    "{}{}",
                    "\\logosection{\\faLanguage}{语言技能}\n\n", work_items
                );
                return message;
            }
            None => {
                return "".to_owned();
            }
        };
    }
}
