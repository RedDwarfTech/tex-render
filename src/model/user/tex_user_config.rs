use serde::Serialize;
use serde::Deserialize;

#[derive(Debug,Serialize,Deserialize,Default,Clone)]
pub struct TexUserConfig {
    pub id: i64,
    pub config_key: String,
    pub remark: String,
    pub created_time: i64,
    pub updated_time: i64,
    pub config_value: String,
    pub user_id: i64,
}