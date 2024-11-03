use std::sync::Arc;

use warp::filters::BoxedFilter;
use warp::{Filter, Reply};

use crate::execution::PlaylistBuilder;
use crate::playlist::{OnDemandTimeRange, Playlist};
use crate::server::types::{TsFile, VodQueryParams};
use crate::upload::Uploader;

pub mod types;

/// Server factory, builds a
pub fn backend<U: Uploader + 'static>(
    pb: PlaylistBuilder,
    uploader: Arc<U>,
) -> BoxedFilter<(impl Reply,)> {
    let uploader = Arc::clone(&uploader);

    // file server. uses object_storage directly.
    let file_route = warp::path!("files" / String)
        .and(warp::any().map(move || uploader.clone()))
        .then(file_handler);

    let vod_route = warp::path!("vod")
        .and(warp::query::<VodQueryParams>())
        .and(warp::any().map(move || pb.clone()))
        .then(vod_handler);

    warp::get().and(file_route.or(vod_route)).boxed()
}

async fn file_handler<U: Uploader>(file_id: String, uploader: Arc<U>) -> TsFile {
    let data = uploader.read_chunk(file_id.as_str()).await;
    TsFile { data }
}

async fn vod_handler(vod_params: VodQueryParams, builder: PlaylistBuilder) -> Playlist {
    let start = vod_params.start_time;
    let end = vod_params.end_time;

    // Construct a new playlist from our output example
    builder.build_on_demand(OnDemandTimeRange { start, end })
}
