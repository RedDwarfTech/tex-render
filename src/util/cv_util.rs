use crate::model::cv::{
    project::cv_project_resp::CvProjectResp, skill::cv_skill_resp::CvSkillResp,
    work::cv_work_resp::CvWorkResp,
};

pub fn gen_work_items(content: String) -> String {
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
    content.push_str(&format!("{}", "\n\\end{itemize}\n"));
    return content;
}

pub fn get_work_str(works: &Option<Vec<CvWorkResp>>) -> String {
    match works {
        Some(edu) => {
            let mut s = String::new();
            for i in edu {
                let parts: Vec<&str> = i.work_start.as_ref().unwrap().split("-").collect();
                let work_start = format!("{}.{}", parts[0], parts[1]);
                let parts_graduation: Vec<&str> = i.work_end.as_ref().unwrap().split("-").collect();
                let work_end: String = format!("{}.{}", parts_graduation[0], parts_graduation[1]);
                let work_item_content =
                    gen_work_items(i.duty.as_deref().unwrap_or_default().to_string());
                s += &format!(
                    "{}{}{}{}{}{}{}{}{}{}{}{}{}",
                    "\\cventry{",
                    work_start,
                    "--",
                    work_end,
                    "}{",
                    i.job.as_ref().unwrap().to_string(),
                    "}{",
                    i.company.to_owned(),
                    "}{",
                    i.city.as_ref().unwrap().to_owned(),
                    "}{}{\n",
                    work_item_content,
                    "}\n\n"
                )
                .to_string();
            }
            return s;
        }
        None => return "".to_owned(),
    }
}

pub fn get_skill_str(works: &Option<Vec<CvSkillResp>>) -> String {
    match works {
        Some(edu) => {
            let mut s = String::new();
            for i in edu {
                s += &format!(
                    "{}{}{}{}{}",
                    "\\cvitem{",
                    i.name.clone(),
                    "}{\\small ",
                    i.memo.as_ref().unwrap().to_string(),
                    "}\n"
                )
                .to_string();
            }
            return s;
        }
        None => return "".to_owned(),
    }
}

pub fn get_project_str(works: &Option<Vec<CvProjectResp>>) -> String {
    match works {
        Some(edu) => {
            let mut s = String::new();
            for i in edu {
                let parts: Vec<&str> = i.work_start.as_ref().unwrap().split("-").collect();
                let work_start = format!("{}.{}", parts[0], parts[1]);
                let parts_graduation: Vec<&str> = i.work_end.as_ref().unwrap().split("-").collect();
                let work_end = format!("{}.{}", parts_graduation[0], parts_graduation[1]);
                let work_item_content =
                    gen_work_items(i.duty.as_deref().unwrap_or_default().to_string());
                s += &format!(
                    "{}{}{}{}{}{}{}{}{}{}{}{}{}",
                    "\\cventry{",
                    work_start,
                    "--",
                    work_end,
                    "}{",
                    i.name.clone(),
                    "}{",
                    i.company.as_deref().unwrap_or_default().to_owned(),
                    "}{",
                    i.city.as_deref().unwrap_or_default(),
                    "}{}{\n",
                    work_item_content,
                    "}\n\n"
                )
                .to_string();
            }
            return s;
        }
        None => return "".to_owned(),
    }
}
