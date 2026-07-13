use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct QueueStartTimeReq {
    pub id: i64,
    pub start_time: i64,
}
