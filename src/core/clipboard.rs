use arboard::Clipboard;
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{params, Connection, Result};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ClipboardEntry {
    pub id: i64,
    pub content: String,
    pub content_type: String,
    pub created_at: DateTime<Utc>,
    pub pinned: bool,
}

/// Initialize clipboard table in database
pub fn init_clipboard_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS clipboard_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL,
            content_type TEXT NOT NULL DEFAULT 'text',
            created_at INTEGER NOT NULL,
            pinned INTEGER NOT NULL DEFAULT 0,
            deleted INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clipboard_created ON clipboard_history(created_at)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clipboard_pinned ON clipboard_history(pinned)",
        [],
    )?;
    Ok(())
}

/// Add new clipboard entry (returns true if actually added, false if duplicate)
pub fn add_entry(conn: &Connection, content: &str, content_type: &str) -> Result<bool> {
    // Skip empty content
    if content.trim().is_empty() {
        return Ok(false);
    }

    // Check for duplicate (last entry with same content)
    let mut stmt = conn.prepare(
        "SELECT id FROM clipboard_history WHERE content = ?1 AND deleted = 0
         ORDER BY created_at DESC LIMIT 1",
    )?;
    let exists = stmt.exists(params![content])?;

    if exists {
        // Update timestamp of existing entry instead of creating duplicate
        conn.execute(
            "UPDATE clipboard_history SET created_at = ?1 WHERE content = ?2 AND deleted = 0",
            params![Utc::now().timestamp(), content],
        )?;
        return Ok(false);
    }

    let now = Utc::now().timestamp();
    conn.execute(
        "INSERT INTO clipboard_history (content, content_type, created_at, pinned, deleted)
         VALUES (?1, ?2, ?3, 0, 0)",
        params![content, content_type, now],
    )?;
    Ok(true)
}

/// Get clipboard history (non-deleted, ordered by pinned first then created_at desc)
pub fn get_history(conn: &Connection, limit: u32) -> Result<Vec<ClipboardEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, content, content_type, created_at, pinned
         FROM clipboard_history
         WHERE deleted = 0
         ORDER BY pinned DESC, created_at DESC
         LIMIT ?1",
    )?;

    let entries = stmt.query_map(params![limit], |row| {
        Ok(ClipboardEntry {
            id: row.get(0)?,
            content: row.get(1)?,
            content_type: row.get(2)?,
            created_at: Utc.timestamp_opt(row.get::<_, i64>(3)?, 0).unwrap(),
            pinned: row.get::<_, i32>(4)? != 0,
        })
    })?;

    entries.collect()
}

/// Toggle pin status
pub fn toggle_pin(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE clipboard_history SET pinned = NOT pinned WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

/// Soft delete entry
pub fn delete_entry(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE clipboard_history SET deleted = 1 WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

/// Cleanup old entries (older than 24 hours, not pinned)
pub fn cleanup_expired(conn: &Connection) -> Result<usize> {
    let cutoff = (Utc::now() - chrono::Duration::hours(24)).timestamp();
    let deleted = conn.execute(
        "DELETE FROM clipboard_history WHERE created_at < ?1 AND pinned = 0",
        params![cutoff],
    )?;
    Ok(deleted)
}

/// Copy content back to clipboard
pub fn copy_to_clipboard(content: &str) -> anyhow::Result<()> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(content)?;
    Ok(())
}

/// Clipboard monitor that runs in background thread
pub struct ClipboardMonitor {
    pub receiver: Receiver<String>,
}

impl ClipboardMonitor {
    pub fn start() -> Self {
        let (tx, rx): (Sender<String>, Receiver<String>) = channel();

        thread::spawn(move || {
            let mut clipboard = match Clipboard::new() {
                Ok(c) => c,
                Err(_) => return,
            };

            let mut last_content = clipboard.get_text().unwrap_or_default();

            loop {
                thread::sleep(Duration::from_millis(500));

                if let Ok(current) = clipboard.get_text() {
                    if current != last_content && !current.is_empty() {
                        last_content = current.clone();
                        let _ = tx.send(current);
                    }
                }
            }
        });

        ClipboardMonitor { receiver: rx }
    }
}

/// Format time ago for display
pub fn format_time_ago(time: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(time);

    if duration.num_seconds() < 60 {
        "just now".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{}m ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h ago", duration.num_hours())
    } else {
        format!("{}d ago", duration.num_days())
    }
}
