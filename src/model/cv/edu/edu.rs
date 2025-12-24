use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct CvEduResp {
    pub id: i64,
    pub edu_addr: String,
    pub created_time: i64,
    pub updated_time: i64,
    pub cv_id: i64,
    pub degree: Option<String>,
    pub major: Option<String>,
    pub city: Option<String>,
    pub user_id: i64,
    pub admission: Option<String>,
    pub graduation: Option<String>,
}