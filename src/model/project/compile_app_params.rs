
#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct CompileAppParams {
    pub file_path: String,
    pub out_path: String,
    pub project_id: String,
    pub req_time: i64,
    pub qid: i64,
    pub version_no: String,
    pub log_file_name: String,
    pub proj_created_time: i64,
}