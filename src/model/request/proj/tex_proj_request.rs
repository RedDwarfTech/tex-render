use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct TexProjRequest {
    /// 渲染状态
    pub comp_status: i32,
    /// 渲染记录ID
    pub id: i64,
    pub comp_result: i32,
}