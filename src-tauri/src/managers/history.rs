use crate::audio_toolkit::apply_correction_rules;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Local, Utc};
use log::{debug, error, info};
use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri_specta::Event;

/// Database migrations for transcription history.
/// Each migration is applied in order. The library tracks which migrations
/// have been applied using SQLite's user_version pragma.
///
/// Note: For users upgrading from tauri-plugin-sql, migrate_from_tauri_plugin_sql()
/// converts the old _sqlx_migrations table tracking to the user_version pragma,
/// ensuring migrations don't re-run on existing databases.
static MIGRATIONS: &[M] = &[
    M::up(
        "CREATE TABLE IF NOT EXISTS transcription_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_name TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            saved BOOLEAN NOT NULL DEFAULT 0,
            title TEXT NOT NULL,
            transcription_text TEXT NOT NULL
        );",
    ),
    M::up("ALTER TABLE transcription_history ADD COLUMN post_processed_text TEXT;"),
    M::up("ALTER TABLE transcription_history ADD COLUMN post_process_prompt TEXT;"),
    M::up("ALTER TABLE transcription_history ADD COLUMN post_process_requested BOOLEAN NOT NULL DEFAULT 0;"),
    M::up(
        "CREATE TABLE IF NOT EXISTS correction_rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            heard_text TEXT NOT NULL,
            heard_text_key TEXT NOT NULL UNIQUE,
            correct_text TEXT NOT NULL,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            source_history_entry_id INTEGER,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_correction_rules_enabled
            ON correction_rules(enabled, id);",
    ),
];

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct PaginatedHistory {
    pub entries: Vec<HistoryEntry>,
    pub has_more: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(tag = "action")]
pub enum HistoryUpdatePayload {
    #[serde(rename = "added")]
    Added { entry: HistoryEntry },
    #[serde(rename = "updated")]
    Updated { entry: HistoryEntry },
    #[serde(rename = "deleted")]
    Deleted { id: i64 },
    #[serde(rename = "toggled")]
    Toggled { id: i64 },
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct HistoryEntry {
    pub id: i64,
    pub file_name: String,
    pub timestamp: i64,
    pub saved: bool,
    pub title: String,
    pub transcription_text: String,
    pub post_processed_text: Option<String>,
    pub post_process_prompt: Option<String>,
    pub post_process_requested: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct CorrectionRule {
    pub id: i64,
    pub heard_text: String,
    pub correct_text: String,
    pub enabled: bool,
    pub source_history_entry_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl crate::audio_toolkit::text::CorrectionRuleInput for CorrectionRule {
    fn heard_text(&self) -> &str {
        &self.heard_text
    }

    fn correct_text(&self) -> &str {
        &self.correct_text
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}

pub struct HistoryManager {
    app_handle: AppHandle,
    recordings_dir: PathBuf,
    db_path: PathBuf,
}

impl HistoryManager {
    pub fn new(app_handle: &AppHandle) -> Result<Self> {
        // Create recordings directory in app data dir
        let app_data_dir = crate::portable::app_data_dir(app_handle)?;
        let recordings_dir = app_data_dir.join("recordings");
        let db_path = app_data_dir.join("history.db");

        // Ensure recordings directory exists
        if !recordings_dir.exists() {
            fs::create_dir_all(&recordings_dir)?;
            debug!("Created recordings directory: {:?}", recordings_dir);
        }

        let manager = Self {
            app_handle: app_handle.clone(),
            recordings_dir,
            db_path,
        };

        // Initialize database and run migrations synchronously
        manager.init_database()?;

        Ok(manager)
    }

    fn init_database(&self) -> Result<()> {
        info!("Initializing database at {:?}", self.db_path);

        let mut conn = Connection::open(&self.db_path)?;

        // Handle migration from tauri-plugin-sql to rusqlite_migration
        // tauri-plugin-sql used _sqlx_migrations table, rusqlite_migration uses user_version pragma
        self.migrate_from_tauri_plugin_sql(&conn)?;

        // Create migrations object and run to latest version
        let migrations = Migrations::new(MIGRATIONS.to_vec());

        // Validate migrations in debug builds
        #[cfg(debug_assertions)]
        migrations.validate().expect("Invalid migrations");

        // Get current version before migration
        let version_before: i32 =
            conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        debug!("Database version before migration: {}", version_before);

        // Apply any pending migrations
        migrations.to_latest(&mut conn)?;

        // Get version after migration
        let version_after: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

        if version_after > version_before {
            info!(
                "Database migrated from version {} to {}",
                version_before, version_after
            );
        } else {
            debug!("Database already at latest version {}", version_after);
        }

        Ok(())
    }

    /// Migrate from tauri-plugin-sql's migration tracking to rusqlite_migration's.
    /// tauri-plugin-sql used a _sqlx_migrations table, while rusqlite_migration uses
    /// SQLite's user_version pragma. This function checks if the old system was in use
    /// and sets the user_version accordingly so migrations don't re-run.
    fn migrate_from_tauri_plugin_sql(&self, conn: &Connection) -> Result<()> {
        // Check if the old _sqlx_migrations table exists
        let has_sqlx_migrations: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !has_sqlx_migrations {
            return Ok(());
        }

        // Check current user_version
        let current_version: i32 =
            conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

        if current_version > 0 {
            // Already migrated to rusqlite_migration system
            return Ok(());
        }

        // Get the highest version from the old migrations table
        let old_version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM _sqlx_migrations WHERE success = 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if old_version > 0 {
            info!(
                "Migrating from tauri-plugin-sql (version {}) to rusqlite_migration",
                old_version
            );

            // Set user_version to match the old migration state
            conn.pragma_update(None, "user_version", old_version)?;

            // Optionally drop the old migrations table (keeping it doesn't hurt)
            // conn.execute("DROP TABLE IF EXISTS _sqlx_migrations", [])?;

            info!(
                "Migration tracking converted: user_version set to {}",
                old_version
            );
        }

        Ok(())
    }

    fn get_connection(&self) -> Result<Connection> {
        Ok(Connection::open(&self.db_path)?)
    }

    fn map_history_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryEntry> {
        Ok(HistoryEntry {
            id: row.get("id")?,
            file_name: row.get("file_name")?,
            timestamp: row.get("timestamp")?,
            saved: row.get("saved")?,
            title: row.get("title")?,
            transcription_text: row.get("transcription_text")?,
            post_processed_text: row.get("post_processed_text")?,
            post_process_prompt: row.get("post_process_prompt")?,
            post_process_requested: row.get("post_process_requested")?,
        })
    }

    fn map_correction_rule(row: &rusqlite::Row<'_>) -> rusqlite::Result<CorrectionRule> {
        Ok(CorrectionRule {
            id: row.get("id")?,
            heard_text: row.get("heard_text")?,
            correct_text: row.get("correct_text")?,
            enabled: row.get("enabled")?,
            source_history_entry_id: row.get("source_history_entry_id")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    fn normalize_correction_text(value: &str) -> String {
        value.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn correction_key(value: &str) -> String {
        Self::normalize_correction_text(value).to_lowercase()
    }

    fn validate_correction_rule_texts(heard_text: &str, correct_text: &str) -> Result<()> {
        if heard_text.is_empty() || correct_text.is_empty() {
            return Err(anyhow!(
                "Correction rule requires both heard and correct text"
            ));
        }

        if heard_text.eq_ignore_ascii_case(correct_text) {
            return Err(anyhow!("Correction rule must change the heard text"));
        }

        Ok(())
    }

    fn select_correction_rule_by_id(conn: &Connection, id: i64) -> Result<CorrectionRule> {
        conn.query_row(
            "SELECT id, heard_text, correct_text, enabled, source_history_entry_id, created_at, updated_at
             FROM correction_rules
             WHERE id = ?1",
            params![id],
            Self::map_correction_rule,
        )
        .map_err(Into::into)
    }

    fn has_duplicate_correction_rule_key(
        conn: &Connection,
        heard_text_key: &str,
        excluded_id: Option<i64>,
    ) -> Result<bool> {
        let duplicate_id: Option<i64> = match excluded_id {
            Some(id) => conn
                .query_row(
                    "SELECT id FROM correction_rules WHERE heard_text_key = ?1 AND id != ?2",
                    params![heard_text_key, id],
                    |row| row.get("id"),
                )
                .optional()?,
            None => conn
                .query_row(
                    "SELECT id FROM correction_rules WHERE heard_text_key = ?1",
                    params![heard_text_key],
                    |row| row.get("id"),
                )
                .optional()?,
        };

        Ok(duplicate_id.is_some())
    }

    pub fn recordings_dir(&self) -> &std::path::Path {
        &self.recordings_dir
    }

    pub fn create_correction_rule(
        &self,
        heard_text: String,
        correct_text: String,
        source_history_entry_id: Option<i64>,
    ) -> Result<CorrectionRule> {
        let heard_text = Self::normalize_correction_text(&heard_text);
        let correct_text = Self::normalize_correction_text(&correct_text);

        Self::validate_correction_rule_texts(&heard_text, &correct_text)?;

        let heard_text_key = Self::correction_key(&heard_text);
        let now = Utc::now().timestamp();
        let conn = self.get_connection()?;

        if Self::has_duplicate_correction_rule_key(&conn, &heard_text_key, None)? {
            return Err(anyhow!("Duplicate correction rule for '{}'", heard_text));
        }

        conn.execute(
            "INSERT INTO correction_rules (
                heard_text,
                heard_text_key,
                correct_text,
                enabled,
                source_history_entry_id,
                created_at,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                heard_text,
                heard_text_key,
                correct_text,
                true,
                source_history_entry_id,
                now,
                now
            ],
        )?;

        Self::select_correction_rule_by_id(&conn, conn.last_insert_rowid())
    }

    pub fn get_correction_rules(&self) -> Result<Vec<CorrectionRule>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, heard_text, correct_text, enabled, source_history_entry_id, created_at, updated_at
             FROM correction_rules
             ORDER BY id DESC",
        )?;

        let rules = stmt
            .query_map([], Self::map_correction_rule)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(anyhow::Error::from)?;

        Ok(rules)
    }

    pub fn get_enabled_correction_rules(&self) -> Result<Vec<CorrectionRule>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, heard_text, correct_text, enabled, source_history_entry_id, created_at, updated_at
             FROM correction_rules
             WHERE enabled = 1
             ORDER BY id ASC",
        )?;

        let rules = stmt
            .query_map([], Self::map_correction_rule)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(anyhow::Error::from)?;

        Ok(rules)
    }

    pub fn get_correction_vocabulary(&self) -> Result<Vec<String>> {
        Ok(self
            .get_enabled_correction_rules()?
            .into_iter()
            .map(|rule| rule.correct_text)
            .collect())
    }

    pub fn set_correction_rule_enabled(&self, id: i64, enabled: bool) -> Result<CorrectionRule> {
        let now = Utc::now().timestamp();
        let conn = self.get_connection()?;
        let updated = conn.execute(
            "UPDATE correction_rules
             SET enabled = ?1, updated_at = ?2
             WHERE id = ?3",
            params![enabled, now, id],
        )?;

        if updated == 0 {
            return Err(anyhow!("Correction rule {} not found", id));
        }

        Self::select_correction_rule_by_id(&conn, id)
    }

    pub fn update_correction_rule(
        &self,
        id: i64,
        heard_text: Option<String>,
        correct_text: Option<String>,
        enabled: Option<bool>,
    ) -> Result<CorrectionRule> {
        let conn = self.get_connection()?;
        let existing = Self::select_correction_rule_by_id(&conn, id)?;
        let heard_text = heard_text
            .as_deref()
            .map(Self::normalize_correction_text)
            .unwrap_or(existing.heard_text);
        let correct_text = correct_text
            .as_deref()
            .map(Self::normalize_correction_text)
            .unwrap_or(existing.correct_text);
        let enabled = enabled.unwrap_or(existing.enabled);

        Self::validate_correction_rule_texts(&heard_text, &correct_text)?;

        let heard_text_key = Self::correction_key(&heard_text);

        if Self::has_duplicate_correction_rule_key(&conn, &heard_text_key, Some(id))? {
            return Err(anyhow!("Duplicate correction rule for '{}'", heard_text));
        }

        let updated = conn.execute(
            "UPDATE correction_rules
             SET heard_text = ?1,
                 heard_text_key = ?2,
                 correct_text = ?3,
                 enabled = ?4,
                 updated_at = ?5
             WHERE id = ?6",
            params![
                heard_text,
                heard_text_key,
                correct_text,
                enabled,
                Utc::now().timestamp(),
                id
            ],
        )?;

        if updated == 0 {
            return Err(anyhow!("Correction rule {} not found", id));
        }

        Self::select_correction_rule_by_id(&conn, id)
    }

    pub fn delete_correction_rule(&self, id: i64) -> Result<()> {
        let conn = self.get_connection()?;
        let deleted = conn.execute("DELETE FROM correction_rules WHERE id = ?1", params![id])?;

        if deleted == 0 {
            return Err(anyhow!("Correction rule {} not found", id));
        }

        Ok(())
    }

    pub fn apply_enabled_correction_rules(&self, text: &str) -> Result<String> {
        let rules = self.get_enabled_correction_rules()?;
        Ok(apply_correction_rules(text, &rules))
    }

    /// Save a new history entry to the database.
    /// The WAV file should already have been written to the recordings directory.
    pub fn save_entry(
        &self,
        file_name: String,
        transcription_text: String,
        post_process_requested: bool,
        post_processed_text: Option<String>,
        post_process_prompt: Option<String>,
    ) -> Result<HistoryEntry> {
        let timestamp = Utc::now().timestamp();
        let title = self.format_timestamp_title(timestamp);

        let conn = self.get_connection()?;
        conn.execute(
            "INSERT INTO transcription_history (
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                post_process_requested
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &file_name,
                timestamp,
                false,
                &title,
                &transcription_text,
                &post_processed_text,
                &post_process_prompt,
                post_process_requested,
            ],
        )?;

        let entry = HistoryEntry {
            id: conn.last_insert_rowid(),
            file_name,
            timestamp,
            saved: false,
            title,
            transcription_text,
            post_processed_text,
            post_process_prompt,
            post_process_requested,
        };

        debug!("Saved history entry with id {}", entry.id);

        self.cleanup_old_entries()?;

        // Emit typed event for real-time frontend updates
        if let Err(e) = (HistoryUpdatePayload::Added {
            entry: entry.clone(),
        })
        .emit(&self.app_handle)
        {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(entry)
    }

    /// Update an existing history entry with new transcription results (used by retry).
    pub fn update_transcription(
        &self,
        id: i64,
        transcription_text: String,
        post_processed_text: Option<String>,
        post_process_prompt: Option<String>,
    ) -> Result<HistoryEntry> {
        let conn = self.get_connection()?;
        let updated = conn.execute(
            "UPDATE transcription_history
             SET transcription_text = ?1,
                 post_processed_text = ?2,
                 post_process_prompt = ?3
             WHERE id = ?4",
            params![
                transcription_text,
                post_processed_text,
                post_process_prompt,
                id
            ],
        )?;

        if updated == 0 {
            return Err(anyhow!("History entry {} not found", id));
        }

        let entry = conn
            .query_row(
                "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, post_process_requested
                 FROM transcription_history WHERE id = ?1",
                params![id],
                Self::map_history_entry,
            )?;

        debug!("Updated transcription for history entry {}", id);

        if let Err(e) = (HistoryUpdatePayload::Updated {
            entry: entry.clone(),
        })
        .emit(&self.app_handle)
        {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(entry)
    }

    pub fn cleanup_old_entries(&self) -> Result<()> {
        let retention_period = crate::settings::get_recording_retention_period(&self.app_handle);

        match retention_period {
            crate::settings::RecordingRetentionPeriod::Never => {
                // Don't delete anything
                return Ok(());
            }
            crate::settings::RecordingRetentionPeriod::PreserveLimit => {
                // Use the old count-based logic with history_limit
                let limit = crate::settings::get_history_limit(&self.app_handle);
                return self.cleanup_by_count(limit);
            }
            _ => {
                // Use time-based logic
                return self.cleanup_by_time(retention_period);
            }
        }
    }

    fn delete_entries_and_files(&self, entries: &[(i64, String)]) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        let conn = self.get_connection()?;
        let mut deleted_count = 0;

        for (id, file_name) in entries {
            // Delete database entry
            conn.execute(
                "DELETE FROM transcription_history WHERE id = ?1",
                params![id],
            )?;

            // Delete WAV file
            let file_path = self.recordings_dir.join(file_name);
            if file_path.exists() {
                if let Err(e) = fs::remove_file(&file_path) {
                    error!("Failed to delete WAV file {}: {}", file_name, e);
                } else {
                    debug!("Deleted old WAV file: {}", file_name);
                    deleted_count += 1;
                }
            }
        }

        Ok(deleted_count)
    }

    fn cleanup_by_count(&self, limit: usize) -> Result<()> {
        let conn = self.get_connection()?;

        // Get all entries that are not saved, ordered by timestamp desc
        let mut stmt = conn.prepare(
            "SELECT id, file_name FROM transcription_history WHERE saved = 0 ORDER BY timestamp DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>("id")?, row.get::<_, String>("file_name")?))
        })?;

        let mut entries: Vec<(i64, String)> = Vec::new();
        for row in rows {
            entries.push(row?);
        }

        if entries.len() > limit {
            let entries_to_delete = &entries[limit..];
            let deleted_count = self.delete_entries_and_files(entries_to_delete)?;

            if deleted_count > 0 {
                debug!("Cleaned up {} old history entries by count", deleted_count);
            }
        }

        Ok(())
    }

    fn cleanup_by_time(
        &self,
        retention_period: crate::settings::RecordingRetentionPeriod,
    ) -> Result<()> {
        let conn = self.get_connection()?;

        // Calculate cutoff timestamp (current time minus retention period)
        let now = Utc::now().timestamp();
        let cutoff_timestamp = match retention_period {
            crate::settings::RecordingRetentionPeriod::Days3 => now - (3 * 24 * 60 * 60), // 3 days in seconds
            crate::settings::RecordingRetentionPeriod::Weeks2 => now - (2 * 7 * 24 * 60 * 60), // 2 weeks in seconds
            crate::settings::RecordingRetentionPeriod::Months3 => now - (3 * 30 * 24 * 60 * 60), // 3 months in seconds (approximate)
            _ => unreachable!("Should not reach here"),
        };

        // Get all unsaved entries older than the cutoff timestamp
        let mut stmt = conn.prepare(
            "SELECT id, file_name FROM transcription_history WHERE saved = 0 AND timestamp < ?1",
        )?;

        let rows = stmt.query_map(params![cutoff_timestamp], |row| {
            Ok((row.get::<_, i64>("id")?, row.get::<_, String>("file_name")?))
        })?;

        let mut entries_to_delete: Vec<(i64, String)> = Vec::new();
        for row in rows {
            entries_to_delete.push(row?);
        }

        let deleted_count = self.delete_entries_and_files(&entries_to_delete)?;

        if deleted_count > 0 {
            debug!(
                "Cleaned up {} old history entries based on retention period",
                deleted_count
            );
        }

        Ok(())
    }

    pub async fn get_history_entries(
        &self,
        cursor: Option<i64>,
        limit: Option<usize>,
    ) -> Result<PaginatedHistory> {
        let conn = self.get_connection()?;
        let limit = limit.map(|l| l.min(100));

        let mut entries: Vec<HistoryEntry> = match (cursor, limit) {
            (Some(cursor_id), Some(lim)) => {
                let fetch_count = (lim + 1) as i64;
                let mut stmt = conn.prepare(
                    "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, post_process_requested
                     FROM transcription_history
                     WHERE id < ?1
                     ORDER BY id DESC
                     LIMIT ?2",
                )?;
                let result = stmt
                    .query_map(params![cursor_id, fetch_count], Self::map_history_entry)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                result
            }
            (None, Some(lim)) => {
                let fetch_count = (lim + 1) as i64;
                let mut stmt = conn.prepare(
                    "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, post_process_requested
                     FROM transcription_history
                     ORDER BY id DESC
                     LIMIT ?1",
                )?;
                let result = stmt
                    .query_map(params![fetch_count], Self::map_history_entry)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                result
            }
            (_, None) => {
                let mut stmt = conn.prepare(
                    "SELECT id, file_name, timestamp, saved, title, transcription_text, post_processed_text, post_process_prompt, post_process_requested
                     FROM transcription_history
                     ORDER BY id DESC",
                )?;
                let result = stmt
                    .query_map([], Self::map_history_entry)?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                result
            }
        };

        let has_more = limit.is_some_and(|lim| entries.len() > lim);
        if has_more {
            entries.pop();
        }

        Ok(PaginatedHistory { entries, has_more })
    }

    #[cfg(test)]
    fn get_latest_entry_with_conn(conn: &Connection) -> Result<Option<HistoryEntry>> {
        let mut stmt = conn.prepare(
            "SELECT
                id,
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                post_process_requested
             FROM transcription_history
             ORDER BY timestamp DESC
             LIMIT 1",
        )?;

        let entry = stmt.query_row([], Self::map_history_entry).optional()?;
        Ok(entry)
    }

    /// Get the latest entry with non-empty transcription text.
    pub fn get_latest_completed_entry(&self) -> Result<Option<HistoryEntry>> {
        let conn = self.get_connection()?;
        Self::get_latest_completed_entry_with_conn(&conn)
    }

    fn get_latest_completed_entry_with_conn(conn: &Connection) -> Result<Option<HistoryEntry>> {
        let mut stmt = conn.prepare(
            "SELECT
                id,
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                post_process_requested
             FROM transcription_history
             WHERE transcription_text != ''
             ORDER BY timestamp DESC
             LIMIT 1",
        )?;

        let entry = stmt.query_row([], Self::map_history_entry).optional()?;
        Ok(entry)
    }

    pub async fn toggle_saved_status(&self, id: i64) -> Result<()> {
        let conn = self.get_connection()?;

        // Get current saved status
        let current_saved: bool = conn.query_row(
            "SELECT saved FROM transcription_history WHERE id = ?1",
            params![id],
            |row| row.get("saved"),
        )?;

        let new_saved = !current_saved;

        conn.execute(
            "UPDATE transcription_history SET saved = ?1 WHERE id = ?2",
            params![new_saved, id],
        )?;

        debug!("Toggled saved status for entry {}: {}", id, new_saved);

        // Emit history updated event
        if let Err(e) = (HistoryUpdatePayload::Toggled { id }).emit(&self.app_handle) {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(())
    }

    pub fn get_audio_file_path(&self, file_name: &str) -> PathBuf {
        self.recordings_dir.join(file_name)
    }

    pub async fn get_entry_by_id(&self, id: i64) -> Result<Option<HistoryEntry>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT
                id,
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                post_process_requested
             FROM transcription_history
             WHERE id = ?1",
        )?;

        let entry = stmt.query_row([id], Self::map_history_entry).optional()?;

        Ok(entry)
    }

    pub async fn delete_entry(&self, id: i64) -> Result<()> {
        let conn = self.get_connection()?;

        // Get the entry to find the file name
        if let Some(entry) = self.get_entry_by_id(id).await? {
            // Delete the audio file first
            let file_path = self.get_audio_file_path(&entry.file_name);
            if file_path.exists() {
                if let Err(e) = fs::remove_file(&file_path) {
                    error!("Failed to delete audio file {}: {}", entry.file_name, e);
                    // Continue with database deletion even if file deletion fails
                }
            }
        }

        // Delete from database
        conn.execute(
            "DELETE FROM transcription_history WHERE id = ?1",
            params![id],
        )?;

        debug!("Deleted history entry with id: {}", id);

        // Emit history updated event
        if let Err(e) = (HistoryUpdatePayload::Deleted { id }).emit(&self.app_handle) {
            error!("Failed to emit history-updated event: {}", e);
        }

        Ok(())
    }

    fn format_timestamp_title(&self, timestamp: i64) -> String {
        if let Some(utc_datetime) = DateTime::from_timestamp(timestamp, 0) {
            // Convert UTC to local timezone
            let local_datetime = utc_datetime.with_timezone(&Local);
            local_datetime.format("%B %e, %Y - %l:%M%p").to_string()
        } else {
            format!("Recording {}", timestamp)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE transcription_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_name TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                saved BOOLEAN NOT NULL DEFAULT 0,
                title TEXT NOT NULL,
                transcription_text TEXT NOT NULL,
                post_processed_text TEXT,
                post_process_prompt TEXT,
                post_process_requested BOOLEAN NOT NULL DEFAULT 0
            );
            CREATE TABLE correction_rules (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                heard_text TEXT NOT NULL,
                heard_text_key TEXT NOT NULL UNIQUE,
                correct_text TEXT NOT NULL,
                enabled BOOLEAN NOT NULL DEFAULT 1,
                source_history_entry_id INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );",
        )
        .expect("create transcription_history table");
        conn
    }

    fn correction_rule(
        id: i64,
        heard_text: &str,
        correct_text: &str,
        enabled: bool,
    ) -> CorrectionRule {
        CorrectionRule {
            id,
            heard_text: heard_text.to_string(),
            correct_text: correct_text.to_string(),
            enabled,
            source_history_entry_id: None,
            created_at: 100,
            updated_at: 100,
        }
    }

    fn insert_entry(conn: &Connection, timestamp: i64, text: &str, post_processed: Option<&str>) {
        conn.execute(
            "INSERT INTO transcription_history (
                file_name,
                timestamp,
                saved,
                title,
                transcription_text,
                post_processed_text,
                post_process_prompt,
                post_process_requested
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                format!("handy-{}.wav", timestamp),
                timestamp,
                false,
                format!("Recording {}", timestamp),
                text,
                post_processed,
                Option::<String>::None,
                false,
            ],
        )
        .expect("insert history entry");
    }

    #[test]
    fn get_latest_entry_returns_none_when_empty() {
        let conn = setup_conn();
        let entry = HistoryManager::get_latest_entry_with_conn(&conn).expect("fetch latest entry");
        assert!(entry.is_none());
    }

    #[test]
    fn get_latest_entry_returns_newest_entry() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "first", None);
        insert_entry(&conn, 200, "second", Some("processed"));

        let entry = HistoryManager::get_latest_entry_with_conn(&conn)
            .expect("fetch latest entry")
            .expect("entry exists");

        assert_eq!(entry.timestamp, 200);
        assert_eq!(entry.transcription_text, "second");
        assert_eq!(entry.post_processed_text.as_deref(), Some("processed"));
    }

    #[test]
    fn get_latest_completed_entry_skips_empty_entries() {
        let conn = setup_conn();
        insert_entry(&conn, 100, "completed", None);
        insert_entry(&conn, 200, "", None);

        let entry = HistoryManager::get_latest_completed_entry_with_conn(&conn)
            .expect("fetch latest completed entry")
            .expect("completed entry exists");

        assert_eq!(entry.timestamp, 100);
        assert_eq!(entry.transcription_text, "completed");
    }

    #[test]
    fn apply_correction_rules_replaces_phrase_case_insensitively() {
        let rules = vec![correction_rule(1, "Live Zap", "Livess App", true)];

        let corrected = apply_correction_rules("Open live zap, then Live Zap again.", &rules);

        assert_eq!(corrected, "Open Livess App, then Livess App again.");
    }

    #[test]
    fn apply_correction_rules_respects_word_boundaries() {
        let rules = vec![correction_rule(1, "zap", "app", true)];

        let corrected = apply_correction_rules("zap zapping zap.", &rules);

        assert_eq!(corrected, "app zapping app.");
    }

    #[test]
    fn apply_correction_rules_skips_disabled_rules() {
        let rules = vec![correction_rule(1, "Live Zap", "Livess App", false)];

        let corrected = apply_correction_rules("Live Zap", &rules);

        assert_eq!(corrected, "Live Zap");
    }

    #[test]
    fn correction_rule_validation_rejects_empty_text() {
        assert!(HistoryManager::validate_correction_rule_texts("", "Livess App").is_err());
        assert!(HistoryManager::validate_correction_rule_texts("Live Zap", "").is_err());
    }

    #[test]
    fn correction_rule_validation_rejects_noop_rule() {
        assert!(HistoryManager::validate_correction_rule_texts("Live Zap", "live zap").is_err());
    }

    #[test]
    fn duplicate_correction_rule_key_is_detected_case_insensitively() {
        let conn = setup_conn();
        conn.execute(
            "INSERT INTO correction_rules (
                heard_text,
                heard_text_key,
                correct_text,
                enabled,
                source_history_entry_id,
                created_at,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "Live Zap",
                HistoryManager::correction_key("Live Zap"),
                "Livess App",
                true,
                Option::<i64>::None,
                100,
                100
            ],
        )
        .expect("insert correction rule");

        let duplicate_key = HistoryManager::correction_key("live   zap");

        assert!(
            HistoryManager::has_duplicate_correction_rule_key(&conn, &duplicate_key, None)
                .expect("check duplicate")
        );
        assert!(
            !HistoryManager::has_duplicate_correction_rule_key(&conn, &duplicate_key, Some(1))
                .expect("check excluded duplicate")
        );
    }
}
