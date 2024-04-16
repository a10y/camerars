use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};

use crate::playlist::PlaylistFile;

/// Database for keeping track of a set of video files, used to construct new queries.
#[derive(Clone)]
pub struct Database {
    inner: Arc<Mutex<rusqlite::Connection>>,
}

impl Database {
    pub fn memory() -> Self {
        // Construct a new SQLite database in-memory.
        let db = rusqlite::Connection::open_in_memory().unwrap();
        setup_connection(&db);

        let db = Arc::new(Mutex::new(db));
        Self { inner: db }
    }

    pub fn file<P: AsRef<Path>>(file: P) -> Self {
        let db = rusqlite::Connection::open(file).unwrap();
        setup_connection(&db);

        let db = Arc::new(Mutex::new(db));
        Self { inner: db }
    }
}

impl Database {
    pub fn append_file(&self, ts: DateTime<Utc>, file: PlaylistFile) {
        // insert a new playlist file.
        // Query for playlist files, if possible.
        let db = self.inner.lock().unwrap();

        // We should be holding on to a writer as soon as we append a new file here.

        db.execute(
            "INSERT INTO video_files VALUES (?1, ?2, ?3)",
            (file.id.as_str(), ts, file.duration),
        )
        .unwrap();
    }

    pub fn query_files(
        &self,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Vec<PlaylistFile> {
        let db = self.inner.lock().unwrap();

        let start =
            start.unwrap_or_else(|| DateTime::<Utc>::from_str("0000-01-01 00:00:00Z").unwrap());
        let end = end.unwrap_or_else(|| DateTime::<Utc>::from_str("9999-12-31 23:59:59Z").unwrap());

        let mut stmt = db.prepare(
            "SELECT file_id, duration FROM video_files WHERE datetime(start_time) BETWEEN datetime(?1) AND datetime(?2)")
            .unwrap();

        let rows = stmt
            .query_map((start, end), |row| {
                Ok(PlaylistFile {
                    id: row.get(0)?,
                    duration: row.get(1)?,
                })
            })
            .unwrap()
            .map(|item| item.unwrap())
            .collect();

        rows
    }
}

fn setup_connection(db: &rusqlite::Connection) {
    db.execute_batch(
        r#"
            CREATE TABLE IF NOT EXISTS video_files (
                file_id TEXT,
                start_time DATETIME,
                duration REAL
            )
            "#,
    )
    .unwrap();
}

#[cfg(test)]
mod test {
    use std::ops::Add;
    use std::str::FromStr;

    use chrono::{DateTime, TimeDelta, Utc};

    use crate::db::Database;
    use crate::playlist::PlaylistFile;

    #[test]
    pub fn test_init() {
        let db = Database::memory();
        let db = db.inner.lock().unwrap();
        let count: usize = db
            .query_row_and_then("select count(*) as counter FROM video_files", [], |row| {
                row.get(0)
            })
            .unwrap();

        assert_eq!(count, 0);
    }

    #[test]
    pub fn test_rw() {
        let db = Database::memory();

        let t1 = DateTime::<Utc>::from_str("2000-01-01 00:00:00Z").unwrap();
        let t2 = t1.add(TimeDelta::seconds(30));
        let t3 = t1.add(TimeDelta::seconds(60));

        db.append_file(
            t1,
            PlaylistFile {
                id: "0001.ts".to_string(),
                duration: 15.16,
            },
        );
        db.append_file(
            t2,
            PlaylistFile {
                id: "0002.ts".to_string(),
                duration: 15.16,
            },
        );
        db.append_file(
            t3,
            PlaylistFile {
                id: "0003.ts".to_string(),
                duration: 15.16,
            },
        );

        assert_eq!(db.query_files(Some(t1), Some(t1)), vec![file("0001.ts")]);

        assert_eq!(
            db.query_files(Some(t2), None),
            vec![file("0002.ts"), file("0003.ts")]
        );

        assert_eq!(
            db.query_files(None, None,),
            vec![file("0001.ts"), file("0002.ts"), file("0003.ts")]
        );
    }

    fn file(name: &'static str) -> PlaylistFile {
        PlaylistFile {
            id: name.to_string(),
            duration: 15.16,
        }
    }
}
