use super::dyweb_cv_util::{
    get_dyweb_edu_str, get_dyweb_project_str, get_dyweb_skill_str, get_dyweb_work_str, get_lang_skill_str,
};
use crate::{
    model::{cv::cv_main::CvMainResp, request::cv::render_handle_request::RenderHandleRequest},
    render::cv::cv_render::CvRender,
};
use std::{
    fs::{self, OpenOptions},
    io::Write,
};

pub struct DywebCvGenImpl {}

impl DywebCvGenImpl {}

impl CvRender for DywebCvGenImpl {
    fn gen_cv_start(&self, request: &RenderHandleRequest) -> String {
        if fs::metadata(request.file_path).is_ok() {
            fs::remove_file(request.file_path).unwrap();
        }
        let cv_main = &request.cv_main;
        let message = format!(
            "{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
            "\\documentclass{deedy-resume-openfont}\n\n",
            "\\usepackage{fancyhdr}\n\n",
            "\\pagestyle{fancy}\n",
            "\\fancyhf{}\n\n",
            "\\begin{document}\n\n",
            "\\namesection{",
            cv_main.employee_name.clone().unwrap(),
            "}{}",
            "{\\urlstyle{same}\\href{",
            cv_main.email.clone().unwrap(),
            "}{",
            cv_main.email.clone().unwrap(),
            "} | ",
            cv_main.phone.clone().unwrap(),
            " } \n\n"
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
        let message = format!("{}{}{}", tpl_code, "\n", "\\end{minipage}\n \\end{document}");
        file.write_all(message.as_bytes()).unwrap();
        return true;
    }

    fn gen_edu(&self, _file_path: &str, cv_main: &CvMainResp) -> String {
        let edu_items = get_dyweb_edu_str(&cv_main.edu);
        let message = format!(
            "{}{}",
            "\\begin{minipage}[t]{0.25\\textwidth}\n\n \\section{教育经历}\n\\sectionsep\n\n", edu_items
        );
        return message;
    }

    fn gen_work(&self, cv_main: &CvMainResp) -> String {
        let work_items = get_dyweb_work_str(&cv_main.work);
        let message = format!(
            "{}{}",
            "\\hfill\n \\begin{minipage}[t]{0.73\\textwidth}\n\n \\section{工作经历}\n\\sectionsep\n\n", work_items
        );
        return message;
    }

    fn gen_skill(&self, cv_main: &CvMainResp) -> String {
        let work_items = get_dyweb_skill_str(&cv_main.skills);
        let message = format!("{}{}", "\\section{专业技能}\n\\sectionsep\n\n", work_items);
        return message;
    }

    fn gen_project(&self, cv_main: &CvMainResp) -> String {
        let work_items = get_dyweb_project_str(&cv_main.projects);
        let message = format!("{}{}", "\\section{项目经历}\n\\sectionsep\n\n", work_items);
        return message;
    }

    fn gen_lang(&self, cv_main: &CvMainResp) -> String {
        match &cv_main.langs {
            Some(langs) => {
                if langs.len() == 0 {
                    return "".to_owned();
                }
                let work_items = get_lang_skill_str(&cv_main.langs);
                let message = format!(
                    "{}{}",
                    "\\section{语言技能}\n\\sectionsep\n\n", work_items
                );
                return message;
            }
            None => {
                return "".to_owned();
            }
        };
    }
}
