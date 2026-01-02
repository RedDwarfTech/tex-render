#[derive(serde::Deserialize)]
pub struct GetPdfPosParams {
    pub project_id: String,
    pub path: String,
    pub file: String,
    pub main_file: String,
    pub line: u32,
    pub column: u32,
    pub create_time: i64,
}