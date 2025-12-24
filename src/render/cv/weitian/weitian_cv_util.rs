use crate::model::cv::{
    edu::edu::CvEduResp, lang::cv_lang_resp::CvLangResp, project::cv_project_resp::CvProjectResp,
    skill::cv_skill_resp::CvSkillResp, work::cv_work_resp::CvWorkResp,
};

pub fn get_weitian_edu_str(edus: &Option<Vec<CvEduResp>>) -> String {
    match edus {
        Some(edu) => {
            let mut s = String::new();
            for i in edu {
                let parts: Vec<&str> = i.admission.as_ref().unwrap().split("-").collect();
                let admission_date = format!("{}.{}", parts[0], parts[1]);
                let parts_graduation: Vec<&str> =
                    i.graduation.as_ref().unwrap().split("-").collect();
                let graduation_date = format!("{}.{}", parts_graduation[0], parts_graduation[1]);
                s += &format!(
                    "{}{}{}{}{}{}{}{}{}{}{}",
                    "\\begin{educations}\n\\education\n{",
                    admission_date,
                    "}\n[",
                    graduation_date,
                    "]\n{",
                    i.edu_addr.clone(),
                    "}\n{}{",
                    i.major.clone().unwrap(),
                    "}\n{",
                    i.degree.clone().unwrap(),
                    "}\n\\end{educations}\n\n"
                )
                .to_string();
            }
            return s;
        }
        None => return "".to_owned(),
    }
}

pub fn gen_weitian_work_items(content: String) -> String {
    if content.is_empty() {
        return content;
    }
    let content_array = content.split("* ");
    let mut content = "".to_string();
    content_array.for_each(|item| {
        if !item.is_empty() {
            content.push_str(&format!("{}{}", "\\item ", item));
        }
    });
    return content;
}

pub fn get_weitian_work_str(works: &Option<Vec<CvWorkResp>>) -> String {
    match works {
        Some(edu) => {
            let mut s = String::from("\\begin{experiences}\n");
            for i in edu {
                let parts: Vec<&str> = i.work_start.as_ref().unwrap().split("-").collect();
                let work_start = format!("{}.{}", parts[0], parts[1]);
                let parts_graduation: Vec<&str> = i.work_end.as_ref().unwrap().split("-").collect();
                let work_end = format!("{}.{}", parts_graduation[0], parts_graduation[1]);
                let work_item_content =
                    gen_weitian_work_items(i.duty.as_deref().unwrap_or_default().to_string());
                s += &format!(
                    "{}{}{}{}{}{}{}{}{}{}{}",
                    "\\experience\n[",
                    work_start,
                    "]\n{",
                    work_end,
                    "}\n{",
                    i.job.clone().unwrap(),
                    " @ ",
                    i.company,
                    "}\n[\\begin{itemize}\n",
                    work_item_content,
                    "\n\\end{itemize}]\n\n\\separator{0.5ex}\n"
                )
                .to_string();
            }
            s += &format!("{}", "\n\\end{experiences}\n\n");
            return s;
        }
        None => return "".to_owned(),
    }
}

pub fn get_weitian_skill_str(works: &Option<Vec<CvSkillResp>>) -> String {
    match works {
        Some(edu) => {
            let mut s = String::from("\\begin{competences}\n");
            for i in edu {
                s += &format!(
                    "{}{}{}{}{}",
                    "\\comptence{",
                    i.name.clone(),
                    "}{",
                    i.memo.as_ref().unwrap().to_string(),
                    "}\n"
                )
                .to_string();
            }
            s += &format!("{}", "\n\\end{competences}\n\n");
            return s;
        }
        None => return "".to_owned(),
    }
}

pub fn get_weitian_lang_str(works: &Option<Vec<CvLangResp>>) -> String {
    match works {
        Some(edu) => {
            let mut s = String::from("\\begin{competences}\n");
            for i in edu {
                s += &format!(
                    "{}{}{}{}{}{}{}",
                    "\\comptence{",
                    i.name.clone(),
                    "}{",
                    i.level.as_deref().unwrap_or_default(),
                    " --- ",
                    i.memo.as_ref().unwrap().to_string(),
                    "}\n"
                )
                .to_string();
            }
            s += &format!("{}", "\n\\end{competences}\n\n");
            return s;
        }
        None => return "".to_owned(),
    }
}

pub fn get_weitian_project_str(works: &Option<Vec<CvProjectResp>>) -> String {
    match works {
        Some(edu) => {
            let mut s = String::from("\\begin{experiences}\n");
            for i in edu {
                let parts: Vec<&str> = i.work_start.as_ref().unwrap().split("-").collect();
                let work_start = format!("{}.{}", parts[0], parts[1]);
                let parts_graduation: Vec<&str> = i.work_end.as_ref().unwrap().split("-").collect();
                let work_end = format!("{}.{}", parts_graduation[0], parts_graduation[1]);
                let work_item_content =
                    gen_weitian_work_items(i.duty.as_deref().unwrap_or_default().to_string());
                s += &format!(
                    "{}{}{}{}{}{}{}{}{}{}{}",
                    "\\experience\n[",
                    work_start,
                    "]\n{",
                    work_end,
                    "}\n{",
                    i.name.clone(),
                    " @ ",
                    i.company.clone().unwrap(),
                    "}\n[\\begin{itemize}\n",
                    work_item_content,
                    "\n\\end{itemize}]\n\n\\separator{0.5ex}\n"
                )
                .to_string();
            }
            s += &format!("{}", "\n\\end{experiences}\n\n");
            return s;
        }
        None => return "".to_owned(),
    }
}
