use std::sync::Arc;

use warp::filters::BoxedFilter;
use warp::{Filter, Reply};

use crate::execution::PlaylistBuilder;
use crate::playlist::OnDemandTimeRange;
use crate::server::types::{TsFile, VodQueryParams};
use crate::upload::Uploader;

pub mod types;

// Create a new output type node instead here.

/// Server factory, builds a
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
