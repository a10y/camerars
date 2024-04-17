// Access the database internally here.

use chrono::{DateTime, Utc};

#[derive(PartialEq, Copy, Clone)]
pub enum PlaylistKind {
    VOD,
    LIVE,
}

pub struct Playlist {
    pub kind: PlaylistKind,
    pub files: Vec<PlaylistFile>,
}

pub struct OnDemandTimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl From<(DateTime<Utc>, DateTime<Utc>)> for OnDemandTimeRange {
    fn from(value: (DateTime<Utc>, DateTime<Utc>)) -> Self {
        Self {
            start: value.0,
            end: value.1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlaylistFile {
    pub duration: f64,
    pub id: String,
}
