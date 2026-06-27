// db.rs — SQLite config read/write for Vispa Flow
//
// Single SQLite file at %APPDATA%\VispaFlow\config.db.
// Table: settings (key TEXT PRIMARY KEY, value TEXT)
// Tauri commands: get_setting(key), set_setting(key, value)

use rusqlite::{params, Connection, Result as SqlResult};
use std::path::PathBuf;

const DB_NAME: &str = "config.db";

/// Default settings rows inserted on first run.
const DEFAULTS: &[(&str, &str)] = &[
    ("hotkey",          "ctrl+alt+space"),
    ("model_override",  ""),
    ("use_cloud",       "false"),
    ("cloud_provider",  "groq"),
    ("cloud_api_key",   ""),
    ("language",        "auto"),
    ("remove_fillers",  "true"),
    ("warm_seconds",    "60"),
];

/// Return the path to config.db, creating parent dirs if needed.
pub fn db_path() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    let dir = PathBuf::from(appdata).join("VispaFlow");
    std::fs::create_dir_all(&dir).ok();
    dir.join(DB_NAME)
}

/// Return the path to crash.log in %APPDATA%\VispaFlow.
pub fn crash_log_path() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    let dir = PathBuf::from(appdata).join("VispaFlow");
    std::fs::create_dir_all(&dir).ok();
    dir.join("crash.log")
}

/// Open a connection and ensure the schema + defaults exist.
pub fn open() -> SqlResult<Connection> {
    let path = db_path();
    let conn = Connection::open(&path)?;
    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL DEFAULT ''
        );",
    )?;

    for (key, value) in DEFAULTS {
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_setting(key: String) -> Result<String, String> {
    let conn = open().map_err(|e| e.to_string())?;
    let val: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .unwrap_or_default();
    Ok(val)
}

#[tauri::command]
pub fn set_setting(key: String, value: String) -> Result<(), String> {
    let conn = open().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Load all settings as a vector of (key, value) pairs.
pub fn load_all() -> Vec<(String, String)> {
    let Ok(conn) = open() else { return vec![] };
    let Ok(mut stmt) = conn.prepare("SELECT key, value FROM settings") else { return vec![] };
    stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
}

/// Convenience: get a single setting value, or a default string.
pub fn get(key: &str) -> String {
    let Ok(conn) = open() else { return String::new() };
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .unwrap_or_default()
}
