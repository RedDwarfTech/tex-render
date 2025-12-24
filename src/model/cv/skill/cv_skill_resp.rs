use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct CvSkillResp {
    pub id: i64,
    pub created_time: i64,
    pub updated_time: i64,
    pub cv_id: i64,
    pub user_id: i64,
    pub name: String,
    pub memo: Option<String>,
}
