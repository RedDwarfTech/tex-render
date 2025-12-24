use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct CompileOutput {
    pub project_id: String,
    pub out_path: String,
    pub req_time: i64
}