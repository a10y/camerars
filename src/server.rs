use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use warp::filters::BoxedFilter;
use warp::reply::Response;
use warp::{http, Filter, Reply};

use crate::execution::PlaylistBuilder;
use crate::playlist::OnDemandTimeRange;
use crate::upload::Uploader;

#[derive(Serialize, Deserialize)]
pub struct VodQueryParams {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

struct TsFile {
    data: Vec<u8>,
}

impl Reply for TsFile {
    fn into_response(self) -> Response {
        http::Response::builder()
            .header("content-type", "video/MP2T")
            .body(self.data.into())
            .unwrap()
    }
}

pub fn make_server<U: Uploader + 'static>(
    pb: PlaylistBuilder,
    uploader: Arc<U>,
) -> BoxedFilter<(impl Reply,)> {
    let uploader = Arc::clone(&uploader);

    let file = warp::path!("files" / String).map(move |file_id: String| {
        let file_data = uploader.read_chunk(file_id.as_str());
        TsFile { data: file_data }
    });

    let vod = warp::path!("vod").and(warp::query::<VodQueryParams>()).map(
        move |vod_query_params: VodQueryParams| {
            let start = vod_query_params.start_time;
            let end = vod_query_params.end_time;

            // Construct a new playlist from our output example
            let playlist = pb.build_on_demand(OnDemandTimeRange { start, end });

            playlist
        },
    );

    warp::get().and(file.or(vod)).boxed()
}
