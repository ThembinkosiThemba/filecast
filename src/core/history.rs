use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{Connection, Result, params};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct RecentAccess {
    pub path: PathBuf,
    pub last_accessed: DateTime<Utc>,
    pub access_count: i32,
}

#[derive(Clone)]
pub struct CommandHistory {
    pub command: String,
    pub path: PathBuf,
    pub last_run: DateTime<Utc>,
    pub run_count: i32,
}

#[derive(Clone)]
pub struct AppLaunchHistory {
    pub app_name: String,
    pub desktop_path: PathBuf,
    pub last_launched: DateTime<Utc>,
    pub launch_count: i32,
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
    conn.execute(
        "CREATE TABLE IF NOT EXISTS command_history (
            command TEXT NOT NULL,
            path TEXT NOT NULL,
            last_run INTEGER NOT NULL,
            run_count INTEGER NOT NULL,
            PRIMARY KEY (command, path)
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS app_launch_history (
            app_name TEXT NOT NULL,
            desktop_path TEXT PRIMARY KEY,
            last_launched INTEGER NOT NULL,
            launch_count INTEGER NOT NULL
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

pub fn log_command(conn: &Connection, command: &str, path: &Path) -> Result<()> {
    let path_str = path.to_string_lossy().to_string();
    let now = Utc::now().timestamp();

    let mut stmt =
        conn.prepare("SELECT run_count FROM command_history WHERE command = ?1 AND path = ?2")?;
    let mut rows = stmt.query(params![command, path_str])?;

    if let Some(row) = rows.next()? {
        let run_count: i32 = row.get(0)?;
        conn.execute(
            "UPDATE command_history SET last_run = ?1, run_count = ?2 WHERE command = ?3 AND path = ?4",
            params![now, run_count + 1, command, path_str],
        )?;
    } else {
        conn.execute(
            "INSERT INTO command_history (command, path, last_run, run_count) VALUES (?1, ?2, ?3, ?4)",
            params![command, path_str, now, 1],
        )?;
    }

    Ok(())
}

pub fn get_command_history(conn: &Connection, limit: u32) -> Result<Vec<CommandHistory>> {
    let mut stmt = conn.prepare(
        "SELECT command, path, last_run, run_count FROM command_history ORDER BY last_run DESC LIMIT ?1",
    )?;

    let iter = stmt.query_map(params![limit], |row| {
        let command: String = row.get(0)?;
        let path_str: String = row.get(1)?;
        let last_run_ts: i64 = row.get(2)?;
        let run_count: i32 = row.get(3)?;

        Ok(CommandHistory {
            command,
            path: PathBuf::from(path_str),
            last_run: Utc.timestamp_opt(last_run_ts, 0).unwrap(),
            run_count,
        })
    })?;

    let mut history = Vec::new();
    for entry in iter {
        history.push(entry?);
    }

    Ok(history)
}

pub fn log_app_launch(conn: &Connection, app_name: &str, desktop_path: &Path) -> Result<()> {
    let path_str = desktop_path.to_string_lossy().to_string();
    let now = Utc::now().timestamp();

    let mut stmt =
        conn.prepare("SELECT launch_count FROM app_launch_history WHERE desktop_path = ?1")?;
    let mut rows = stmt.query(params![path_str])?;

    if let Some(row) = rows.next()? {
        let launch_count: i32 = row.get(0)?;
        conn.execute(
            "UPDATE app_launch_history SET app_name = ?1, last_launched = ?2, launch_count = ?3 WHERE desktop_path = ?4",
            params![app_name, now, launch_count + 1, path_str],
        )?;
    } else {
        conn.execute(
            "INSERT INTO app_launch_history (app_name, desktop_path, last_launched, launch_count) VALUES (?1, ?2, ?3, ?4)",
            params![app_name, path_str, now, 1],
        )?;
    }

    Ok(())
}

pub fn get_app_launch_history(conn: &Connection, limit: u32) -> Result<Vec<AppLaunchHistory>> {
    let mut stmt = conn.prepare(
        "SELECT app_name, desktop_path, last_launched, launch_count FROM app_launch_history ORDER BY last_launched DESC LIMIT ?1",
    )?;

    let iter = stmt.query_map(params![limit], |row| {
        let app_name: String = row.get(0)?;
        let path_str: String = row.get(1)?;
        let last_launched_ts: i64 = row.get(2)?;
        let launch_count: i32 = row.get(3)?;

        Ok(AppLaunchHistory {
            app_name,
            desktop_path: PathBuf::from(path_str),
            last_launched: Utc.timestamp_opt(last_launched_ts, 0).unwrap(),
            launch_count,
        })
    })?;

    let mut history = Vec::new();
    for entry in iter {
        history.push(entry?);
    }

    Ok(history)
}
