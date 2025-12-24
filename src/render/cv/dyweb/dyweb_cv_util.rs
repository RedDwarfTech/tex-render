use crate::model::cv::{
    edu::edu::CvEduResp, project::cv_project_resp::CvProjectResp,
    skill::cv_skill_resp::CvSkillResp, work::cv_work_resp::CvWorkResp, lang::cv_lang_resp::CvLangResp,
};

pub fn get_dyweb_edu_str(edus: &Option<Vec<CvEduResp>>) -> String {
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
                    "\\subsection{",
                    i.edu_addr,
                    "}\n\\descript{",
                    i.degree.clone().unwrap(),
                    "}\n\\descript{",
                    i.major.clone().unwrap(),
                    "}\n\\location{",
                    admission_date,
                    "-",
                    graduation_date,
                    // https://tex.stackexchange.com/questions/688904/why-the-hfill-command-could-not-handle-the-newline
                    "}\n\\sectionsep\n\n",
                )
                .to_string();
            }
            return s;
        }
        None => return "".to_owned(),
    }
}

pub fn gen_dyweb_work_items(content: String) -> String {
    if content.is_empty() {
        return content;
    }
    let content_array = content.split("* ");
    let mut content = "".to_string();
    content.push_str(&format!("{}", "\\begin{tightemize}\n"));
    content_array.for_each(|item| {
        if !item.is_empty() {
            content.push_str(&format!("{}{}", "\\item ", item));
        }
    });
    content.push_str(&format!("{}", "\n\\end{tightemize}\n\\sectionsep\n\n"));
    return content;
}

pub fn get_dyweb_work_str(works: &Option<Vec<CvWorkResp>>) -> String {
    match works {
        Some(edu) => {
            let mut s = String::new();
            for i in edu {
                let parts: Vec<&str> = i.work_start.as_ref().unwrap().split("-").collect();
                let work_start = format!("{}.{}", parts[0], parts[1]);
                let parts_graduation: Vec<&str> = i.work_end.as_ref().unwrap().split("-").collect();
                let work_end = format!("{}.{}", parts_graduation[0], parts_graduation[1]);
                let work_item_content =
                    gen_dyweb_work_items(i.duty.as_deref().unwrap_or_default().to_string());
                s += &format!(
                    "{}{}{}{}{}{}{}{}{}",
                    "\\runsubsection{",
                    i.company,
                    "}\n\\location{",
                    work_start,
                    "-",
                    work_end,
                    "}\n\\vspace{\\topsep}\n",
                    work_item_content,
                    "\n\n"
                )
                .to_string();
            }
            return s;
        }
        None => return "".to_owned(),
    }
}

pub fn get_dyweb_skill_str(works: &Option<Vec<CvSkillResp>>) -> String {
    match works {
        Some(edu) => {
            let mut s = String::from("");
            for i in edu {
                s += &format!(
                    "{}{}{}{}{}",
                    "\\subsection{",
                    i.name.clone(),
                    "}\n\\location{",
                    i.memo.as_ref().unwrap().to_string(),
                    "}\n\n"
                )
                .to_string();
            }
            s += &format!("{}","\\end{minipage}\n").to_string();
            return s;
        }
        None => return "".to_owned(),
    }
}

pub fn get_lang_skill_str(works: &Option<Vec<CvLangResp>>) -> String {
    match works {
        Some(edu) => {
            let mut s = String::from("");
            for i in edu {
                s += &format!(
                    "{}{}{}{}{}",
                    "\\runsubsection{",
                    i.name.clone(),
                    "}\n\\location{",
                    i.memo.as_ref().unwrap().to_string(),
                    "}\n\n"
                )
                .to_string();
            }
            return s;
        }
        None => return "".to_owned(),
    }
}

pub fn get_dyweb_project_str(works: &Option<Vec<CvProjectResp>>) -> String {
    match works {
        Some(edu) => {
            let mut s = String::new();
            for i in edu {
                let parts: Vec<&str> = i.work_start.as_ref().unwrap().split("-").collect();
                let work_start = format!("{}.{}", parts[0], parts[1]);
                let parts_graduation: Vec<&str> = i.work_end.as_ref().unwrap().split("-").collect();
                let work_end = format!("{}.{}", parts_graduation[0], parts_graduation[1]);
                let work_item_content =
                    gen_dyweb_work_items(i.duty.as_deref().unwrap_or_default().to_string());
                s += &format!(
                    "{}{}{}{}{}{}{}{}{}",
                    "\\runsubsection{",
                    i.name.clone(),
                    "}\n\\location{",
                    work_start,
                    "-",
                    work_end,
                    "}\n\\vspace{\\topsep}\n",
                    work_item_content,
                    "\n\n"
                )
                .to_string();
            }
            return s;
        }
        None => return "".to_owned(),
    }
}
