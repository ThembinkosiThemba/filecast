use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{Connection, Result, params};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct RecentAccess {
    pub path: PathBuf,
    pub last_accessed: DateTime<Utc>,
    pub access_count: i32,
}

pub fn initialise(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS recent_access (
            path TEXT PRIMARY KEY,
            last_accessed INTEGER NOT NULL,
            access_count INTEGER NOT NULL
        )",
        [],
    )?;
    Ok(conn)
}

pub fn log_access(conn: &Connection, path: &Path) -> Result<()> {
    let path_str = path.to_string_lossy().to_string();
    let now = Utc::now().timestamp();

    let mut stmt = conn.prepare("SELECT access_count FROM recent_access WHERE path = ?1")?;
    let mut rows = stmt.query(params![path_str])?;

    if let Some(row) = rows.next()? {
        let access_count: i32 = row.get(0)?;
        conn.execute(
            "UPDATE recent_access SET last_accessed = ?1, access_count = ?2 WHERE path = ?3",
            params![now, access_count + 1, path_str],
        )?;
    } else {
        conn.execute(
            "INSERT INTO recent_access (path, last_accessed, access_count) VALUES (?1, ?2, ?3)",
            params![path_str, now, 1],
        )?;
    }

    Ok(())
}

pub fn get_recent_files(conn: &Connection, limit: u32) -> Result<Vec<RecentAccess>> {
    let mut stmt = conn.prepare(
        "SELECT path, last_accessed, access_count FROM recent_access ORDER BY last_accessed DESC LIMIT ?1",
    )?;

    let recent_files_iter = stmt.query_map(params![limit], |row| {
        let path_str: String = row.get(0)?;
        let last_accessed_ts: i64 = row.get(1)?;
        let access_count: i32 = row.get(2)?;

        Ok(RecentAccess {
            path: PathBuf::from(path_str),
            last_accessed: Utc.timestamp_opt(last_accessed_ts, 0).unwrap(),
            access_count,
        })
    })?;

    let mut recent_files = Vec::new();
    for recent_file in recent_files_iter {
        recent_files.push(recent_file?);
    }

    Ok(recent_files)
}
