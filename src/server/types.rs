use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use warp::reply::Response;
use warp::{http, Reply};

#[derive(Serialize, Deserialize)]
pub(crate) struct VodQueryParams {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

pub(crate) struct TsFile {
    pub(crate) data: Vec<u8>,
}

impl Reply for TsFile {
    fn into_response(self) -> Response {
        http::Response::builder()
            .header("content-type", "video/MP2T")
            .body(self.data.into())
            .unwrap()
    }
}
