use serde::{Deserialize, Serialize};

use super::{
    edu::edu::CvEduResp, project::cv_project_resp::CvProjectResp,
    skill::cv_skill_resp::CvSkillResp, work::cv_work_resp::CvWorkResp, lang::cv_lang_resp::CvLangResp,
};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct CvMainResp {
    pub id: i64,
    pub cv_name: String,
    pub created_time: i64,
    pub updated_time: i64,
    pub user_id: i64,
    pub cv_status: i32,
    pub template_id: i64,
    pub employee_name: Option<String>,
    pub birthday: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub stackoverflow: Option<String>,
    pub github: Option<String>,
    pub blog: Option<String>,
    pub item_order: String,
    pub main_color: Option<String>,
    pub theme: Option<String>,
    pub font_size: Option<String>,
    pub edu: Option<Vec<CvEduResp>>,
    pub work: Option<Vec<CvWorkResp>>,
    pub skills: Option<Vec<CvSkillResp>>,
    pub projects: Option<Vec<CvProjectResp>>,
    pub langs: Option<Vec<CvLangResp>>,
}
