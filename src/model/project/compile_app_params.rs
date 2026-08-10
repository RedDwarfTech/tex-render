use uuid::Uuid;

fn default_x_request_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn generate_x_request_id() -> String {
    default_x_request_id()
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct CompileAppParams {
    pub file_path: String,
    pub out_path: String,
    pub project_id: String,
    pub req_time: i64,
    pub qid: i64,
    pub version_no: i64,
    pub log_file_name: String,
    pub proj_created_time: i64,
    #[serde(default = "default_x_request_id")]
    pub x_request_id: String,
}