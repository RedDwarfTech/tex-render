use serde::{Deserialize, Serialize};

#[derive(Debug,Serialize,Deserialize,Default,Clone)]
pub struct TexCompQueue {
    pub id: i64,
    pub created_time: i64,
    pub updated_time: i64,
    pub user_id: i64,
    pub comp_status: i32,
    pub project_id: String,
}