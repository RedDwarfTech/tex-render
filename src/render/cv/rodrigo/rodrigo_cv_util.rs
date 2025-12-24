use crate::model::cv::{
    edu::edu::CvEduResp, project::cv_project_resp::CvProjectResp,
    skill::cv_skill_resp::CvSkillResp, work::cv_work_resp::CvWorkResp,
};

pub fn get_rodrigo_edu_str(edus: &Option<Vec<CvEduResp>>) -> String {
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
                    "{}{}{}{}{}{}{}{}{}{}{}{}{}",
                    "\\datedline{\\textbf{",
                    i.edu_addr,
                    "}}{\\dateRange{",
                    admission_date,
                    "}{",
                    graduation_date,
                    "}}\n\\datedline{\\tripleInfo{",
                    i.major.clone().unwrap(),
                    "}{",
                    i.degree.clone().unwrap(),
                    "}{}}{",
                    i.city.clone().unwrap(),
                    "}\n\n"
                )
                .to_string();
            }
            return s;
        }
        None => return "".to_owned(),
    }
}

pub fn gen_rodrigo_work_items(content: String) -> String {
    if content.is_empty() {
        return content;
    }
    let content_array = content.split("* ");
    let mut content = "".to_string();
    content.push_str(&format!("{}", "\\begin{itemize}\n"));
    content_array.for_each(|item| {
        if !item.is_empty() {
            content.push_str(&format!("{}{}", "\\item ", item));
        }
    });
    content.push_str(&format!("{}", "\n\\end{itemize}\n\n"));
    return content;
}

pub fn get_rodrigo_work_str(works: &Option<Vec<CvWorkResp>>) -> String {
    match works {
        Some(edu) => {
            let mut s = String::new();
            for i in edu {
                let parts: Vec<&str> = i.work_start.as_ref().unwrap().split("-").collect();
                let work_start = format!("{}.{}", parts[0], parts[1]);
                let parts_graduation: Vec<&str> = i.work_end.as_ref().unwrap().split("-").collect();
                let work_end = format!("{}.{}", parts_graduation[0], parts_graduation[1]);
                let work_item_content =
                    gen_rodrigo_work_items(i.duty.as_deref().unwrap_or_default().to_string());
                s += &format!(
                    "{}{}{}{}{}{}{}{}{}{}{}{}{}",
                    "\\datedline{\\textbf{",
                    i.company,
                    "}}{\\dateRange{",
                    work_start,
                    "}{",
                    work_end,
                    "}}\n\\datedline{",
                    i.job.clone().unwrap(),
                    "}{",
                    i.city.clone().unwrap(),
                    "}\n\n",
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

pub fn get_rodrigo_skill_str(works: &Option<Vec<CvSkillResp>>) -> String {
    match works {
        Some(edu) => {
            let mut s = String::from("\\begin{itemize}[parsep=0.5ex]\n");
            for i in edu {
                s += &format!(
                    "{}{}{}{}{}",
                    "\\item {",
                    i.name.clone(),
                    ": ",
                    i.memo.as_ref().unwrap().to_string(),
                    "}\n"
                )
                .to_string();
            }
            s += &format!("{}", "\\end{itemize}\n\n").to_string();
            return s;
        }
        None => return "".to_owned(),
    }
}

pub fn get_rodrigo_project_str(works: &Option<Vec<CvProjectResp>>) -> String {
    match works {
        Some(edu) => {
            let mut s = String::new();
            for i in edu {
                let parts: Vec<&str> = i.work_start.as_ref().unwrap().split("-").collect();
                let work_start = format!("{}.{}", parts[0], parts[1]);
                let parts_graduation: Vec<&str> = i.work_end.as_ref().unwrap().split("-").collect();
                let work_end = format!("{}.{}", parts_graduation[0], parts_graduation[1]);
                let work_item_content =
                    gen_rodrigo_work_items(i.duty.as_deref().unwrap_or_default().to_string());
                s += &format!(
                    "{}{}{}{}{}{}{}{}",
                    "\\datedline{\\textbf{",
                    i.name,
                    "}}{\\dateRange{",
                    work_start,
                    "}{",
                    work_end,
                    "}}\n\n",
                    work_item_content
                )
                .to_string();
            }
            return s;
        }
        None => return "".to_owned(),
    }
}
