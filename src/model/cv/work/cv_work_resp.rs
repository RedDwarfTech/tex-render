use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct CvWorkResp {
    pub id: i64,
    pub company: String,
    pub created_time: i64,
    pub updated_time: i64,
    pub cv_id: i64,
    pub job: Option<String>,
    pub city: Option<String>,
    pub work_start: Option<String>,
    pub work_end: Option<String>,
    pub user_id: i64,
    pub duty: Option<String>,
}
