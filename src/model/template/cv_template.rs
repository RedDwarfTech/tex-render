use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct CvTemplate {
    pub id: i64,
    pub name: String,
    pub remark: String,
    pub created_time: i64,
    pub updated_time: i64,
    pub template_status: i32,
    pub template_id: i64,
    pub preview_url: Option<String>,
    pub template_code: Option<String>,
}