use std::{
    fs::{self, OpenOptions},
    io::Write,
};

use crate::{
    model::{cv::cv_main::CvMainResp, request::cv::render_handle_request::RenderHandleRequest},
    render::cv::cv_render::CvRender,
};

use super::weitian_cv_util::{
    get_weitian_edu_str, get_weitian_lang_str, get_weitian_project_str, get_weitian_skill_str,
    get_weitian_work_str,
};

pub struct WeitianCvGenImpl {}

impl WeitianCvGenImpl {}
/**
 * the moderncv fonts issue
 * https://apple.stackexchange.com/questions/205639/to-install-latin-modern-math-font-in-os-x
 */
impl CvRender for WeitianCvGenImpl {
    fn gen_cv_start(&self, request: &RenderHandleRequest) -> String {
        if fs::metadata(request.file_path).is_ok() {
            fs::remove_file(request.file_path).unwrap();
        }
        let binding = request.cv_main.employee_name.clone().unwrap();
        let ss = binding.as_str();
        let first_name: String = ss.chars().skip(0).take(1).collect();
        let last_name: String = ss.chars().skip(1).take(ss.len() - 1).collect();
        let name = format!(
            "{}{}{}{}{}",
            "\\name{", last_name, "}{", first_name, "}\n\n"
        );
        let _title = format!("{}{}{}", "\\title{", request.cv_main.cv_name, "}\n");
        let phone = format!(
            "{}{}{}",
            "\\mobile{",
            request.cv_main.phone.as_ref().unwrap(),
            "}\n"
        );
        let email = format!(
            "{}{}{}",
            "\\email{",
            request.cv_main.email.as_ref().unwrap(),
            "}\n"
        );
        let _stackoverflow = format!(
            "{}{}{}",
            "\\social[stackoverflow]{",
            request.cv_main.stackoverflow.as_deref().unwrap_or_default(),
            "}\n"
        );
        let github = format!(
            "{}{}{}",
            "\\github{",
            request
                .cv_main
                .github
                .as_deref()
                .unwrap_or_default()
                .split("/")
                .last()
                .unwrap(),
            "}\n"
        );
        let message = format!(
            "{}{}{}{}{}{}{}{}{}{}{}{}{}",
            "\\documentclass[zh]{weitian-resume}\n\n",
            "\\iconsize{\\Large}\n",
            "\\fileinfo{\n",
            // https://tex.stackexchange.com/questions/687144/missing-character-there-is-no-%e8%92%8b-u848b-in-font-lmsans17-regularmapping-tex
            "\\faCopyright{} \\the\\year, ",
            request.cv_main.employee_name.clone().unwrap_or_default(),
            " \\hspace{0.5em}\n",
            "\\faEdit{} \\today \n}\n\n",
            name,
            "\\profile{\n",
            phone,
            email,
            github,
            "}\n\n\\begin{document}\n\\makeheader\n",
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
        let edu_items = get_weitian_edu_str(&cv_main.edu);
        let message = format!(
            "{}{}",
            "\\sectionTitle{教育经历}{\\faGraduationCap}\n\n", edu_items
        );
        return message;
    }

    fn gen_work(&self, cv_main: &CvMainResp) -> String {
        let work_items = get_weitian_work_str(&cv_main.work);
        let message = format!(
            "{}{}",
            "\\sectionTitle{工作经历}{\\faBriefcase}\n\n", work_items
        );
        return message;
    }

    fn gen_skill(&self, cv_main: &CvMainResp) -> String {
        let work_items = get_weitian_skill_str(&cv_main.skills);
        let message = format!(
            "{}{}",
            "\\sectionTitle{专业技能}{\\faWrench}\n\n", work_items
        );
        return message;
    }

    fn gen_project(&self, cv_main: &CvMainResp) -> String {
        let work_items = get_weitian_project_str(&cv_main.projects);
        let message = format!("{}{}", "\\sectionTitle{项目经历}{\\faCode}\n\n", work_items);
        return message;
    }

    fn gen_lang(&self, cv_main: &CvMainResp) -> String {
        match &cv_main.langs {
            Some(langs) => {
                if langs.len() == 0 {
                    return "".to_owned();
                }
                let work_items = get_weitian_lang_str(&cv_main.langs);
                let message = format!(
                    "{}{}",
                    "\\sectionTitle{语言技能}{\\faLanguage}\n\n", work_items
                );
                return message;
            }
            None => {
                return "".to_owned();
            }
        };
    }
}
