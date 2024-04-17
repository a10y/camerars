use warp::reply::Response;
use warp::{http, Reply};

use crate::playlist::{Playlist, PlaylistKind};

impl Reply for Playlist {
    fn into_response(self) -> Response {
        let mut body = String::new();
        body.push_str("#EXTM3U\r\n");

        if matches!(self.kind, PlaylistKind::VOD) {
            body.push_str("#EXT-X-PLAYLIST-TYPE:VOD\r\n");
        }
        body.push_str("#EXT-X-TARGETDURATION:15\r\n");
        body.push_str("#EXT-X-VERSION:4\r\n");
        body.push_str("#EXT-X-MEDIA-SEQUENCE:1\r\n");
        body.push_str("\r\n");

        for file in self.files {
            body.push_str(format!("#EXTINF:{}\r\n", file.duration).as_str());
            body.push_str(format!("files/{}\r\n", file.id.as_str()).as_str());
        }

        if matches!(self.kind, PlaylistKind::VOD) {
            body.push_str("#EXT-X-ENDLIST\r\n");
        }

        http::Response::builder()
            .header("content-type", "application/x-mpegURL")
            .body(body.into())
            .unwrap()
    }
}
