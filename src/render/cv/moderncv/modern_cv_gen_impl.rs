use std::{
    fs::{self, OpenOptions},
    io::Write,
};

use crate::{
    model::{cv::cv_main::CvMainResp, request::cv::render_handle_request::RenderHandleRequest},
    render::cv::cv_render::CvRender,
    util::cv_util::{ get_project_str, get_skill_str, get_work_str},
};

pub struct ModernCvGenImpl {}

impl ModernCvGenImpl {}
/**
 * the moderncv fonts issue
 * https://apple.stackexchange.com/questions/205639/to-install-latin-modern-math-font-in-os-x
 */
impl CvRender for ModernCvGenImpl {
    fn gen_cv_start(&self, request: &RenderHandleRequest) -> String {
        if fs::metadata(request.file_path).is_ok() {
            fs::remove_file(request.file_path).unwrap();
        }
        let binding = request.cv_main.employee_name.clone().unwrap();
        let ss = binding.as_str();
        let first_name: String = ss.chars().skip(0).take(1).collect();
        let last_name: String = ss.chars().skip(1).take(ss.len() - 1).collect();
        let name = format!("{}{}{}{}{}", "\\name{", first_name, "}{", last_name, "}\n");
        let title = format!("{}{}{}", "\\title{", request.cv_main.cv_name, "}\n");
        let phone = format!(
            "{}{}{}",
            "\\phone[mobile]{",
            request.cv_main.phone.as_ref().unwrap(),
            "}\n"
        );
        let email = format!(
            "{}{}{}",
            "\\email{",
            request.cv_main.email.as_ref().unwrap(),
            "}\n"
        );
        let stackoverflow = format!(
            "{}{}{}",
            "\\social[stackoverflow]{",
            request.cv_main.stackoverflow.as_deref().unwrap_or_default(),
            "}\n"
        );
        let github = format!(
            "{}{}{}",
            "\\social[github]{",
            request.cv_main.github.as_deref().unwrap_or_default(),
            "}\n"
        );
        let extra = format!(
            "{}{}{}",
            "\\extrainfo{出生日期：",
            request.cv_main.birthday.as_deref().unwrap_or_default(),
            "}\n"
        );
        let message = format!(
            "{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
            "\\documentclass[",
            request
                .cv_main
                .font_size
                .clone()
                .unwrap_or(String::from("10pt")),
            ",a4paper,roman]{moderncv}\n\n",
            "\\moderncvstyle{",
            request
                .cv_main
                .theme
                .clone()
                .unwrap_or(String::from("classic")),
            "}\n",
            "\\moderncvcolor{",
            request
                .cv_main
                .main_color
                .clone()
                .unwrap_or(String::from("black")),
            "}\n",
            // https://tex.stackexchange.com/questions/532114/use-moderncv-casual-icons-in-moderncv-classic-layout
            "\\moderncvicons{awesome}\n\n",
            // https://tex.stackexchange.com/questions/687144/missing-character-there-is-no-%e8%92%8b-u848b-in-font-lmsans17-regularmapping-tex
            "\\usepackage{ctex}\n",
            "\\usepackage{fontspec}\n",
            "\\usepackage[scale=0.75]{geometry}\n\n",
            "\\setmainfont{lmroman10-regular.otf}\n",
            "\\setlength{\\footskip}{149.60005pt}\n",
            "\\setlength{\\hintscolumnwidth}{3cm}\n\n",
            name,
            title,
            phone,
            email,
            stackoverflow,
            github,
            extra,
            "\n\\begin{document}\n\n",
            "\\makecvtitle\n\n"
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
        let message = format!("{}", "\n\\section{教育经历}\n\n");
        return message;
    }

    fn gen_work(&self, cv_main: &CvMainResp) -> String {
        let work_items = get_work_str(&cv_main.work);
        let message = format!("{}{}", "\n\\section{工作经历}\n\n", work_items);
        return message;
    }

    fn gen_skill(&self, cv_main: &CvMainResp) -> String {
        let work_items = get_skill_str(&cv_main.skills);
        let message = format!("{}{}", "\n\\section{专业技能}\n\n", work_items);
        return message;
    }

    fn gen_project(&self, cv_main: &CvMainResp) -> String {
        let work_items = get_project_str(&cv_main.projects);
        let message = format!("{}{}", "\n\\section{项目经历}\n\n", work_items);
        return message;
    }

    fn gen_lang(&self, cv_main: &CvMainResp) -> String {
        match &cv_main.langs {
            Some(langs) => {
                if langs.len() == 0 {
                    return "".to_owned();
                }
                let lang_items = "";
                let message = format!("{}{}", "\n\\section{语言技能}\n\n", lang_items);
                return message;
            }
            None => {
                return "".to_owned();
            }
        };
    }
}
