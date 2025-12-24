use serde::{Deserialize, Serialize};

/// Request body used to report render result back to the CV API.
///
/// Matches the fields used in `update_gen_result` in `cv_client.rs`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderResultRequest {
    pub gen_status: i32,
    pub id: i64,
    pub path: String,
    pub tex_file_path: String,
}
