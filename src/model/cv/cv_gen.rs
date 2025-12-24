use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct CvGen {
    pub id: i64,
    pub cv_name: String,
    pub remark: String,
    pub created_time: i64,
    pub updated_time: i64,
    pub user_id: i64,
    pub gen_status: i32,
    pub gen_time: Option<i64>,
    pub path: Option<String>,
    pub template_id: i64,
    pub cv_id: i64,
}