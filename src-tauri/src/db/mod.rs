pub mod context;
pub mod meetings;
pub mod migrations;
pub mod rag;
pub mod translation;

use rusqlite::Connection;
use std::path::PathBuf;

/// Database manager with rusqlite connection and auto-migration.
pub struct DatabaseManager {
    conn: Connection,
}

impl DatabaseManager {
    /// Opens (or creates) the SQLite database at the standard app data path
    /// and runs all migrations.
    pub fn new(app_data_dir: PathBuf) -> Result<Self, DatabaseError> {
        // Ensure directory exists
        std::fs::create_dir_all(&app_data_dir).map_err(|e| {
            DatabaseError::Init(format!("Failed to create data directory: {}", e))
        })?;

        let db_path = app_data_dir.join("nexq.db");
        log::info!("Opening database at: {}", db_path.display());

        let conn = Connection::open(&db_path).map_err(|e| {
            DatabaseError::Init(format!("Failed to open database: {}", e))
        })?;

        let version: u32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version < 12 && db_path.metadata().map(|m| m.len() > 0).unwrap_or(false) {
            let backup = app_data_dir.join(format!("nexq-before-v12-{}.db", chrono::Utc::now().format("%Y%m%dT%H%M%S%f")));
            conn.execute("VACUUM INTO ?1", [backup.to_string_lossy().as_ref()])?;
        }
        // Run migrations
        migrations::run(&conn).map_err(|e| {
            DatabaseError::Migration(format!("Migration failed: {}", e))
        })?;

        Ok(Self { conn })
    }

    /// Returns a reference to the underlying SQLite connection.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Database initialization error: {0}")]
    Init(String),
    #[error("Migration error: {0}")]
    Migration(String),
    #[error("Query error: {0}")]
    Query(String),
    #[error("Not found: {0}")]
    NotFound(String),
}

impl From<rusqlite::Error> for DatabaseError {
    fn from(e: rusqlite::Error) -> Self {
        DatabaseError::Query(e.to_string())
    }
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn migration_backs_up_existing_preferences_and_meetings(){
        let dir=std::env::temp_dir().join(format!("meetinghelper-migration-{}",uuid::Uuid::new_v4()));
        {
            let db=DatabaseManager::new(dir.clone()).unwrap();let conn=db.connection();
            conn.execute("INSERT INTO app_state VALUES ('replyLanguage','tr')",[]).unwrap();
            crate::db::meetings::create_meeting(conn,"Saved meeting",None,"live").unwrap();
            conn.execute_batch("ALTER TABLE project_documents DROP COLUMN content; PRAGMA user_version=11;").unwrap();
        }
        let db=DatabaseManager::new(dir.clone()).unwrap();let conn=db.connection();
        assert_eq!(conn.query_row("PRAGMA user_version",[],|r|r.get::<_,u32>(0)).unwrap(),12);
        assert_eq!(conn.query_row("SELECT value FROM app_state WHERE key='replyLanguage'",[],|r|r.get::<_,String>(0)).unwrap(),"tr");
        assert_eq!(conn.query_row("SELECT count(*) FROM meetings",[],|r|r.get::<_,i64>(0)).unwrap(),1);
        let backup=std::fs::read_dir(&dir).unwrap().flatten().find(|e|e.file_name().to_string_lossy().starts_with("nexq-before-v12")).unwrap();
        let prior=Connection::open(backup.path()).unwrap();assert_eq!(prior.query_row("PRAGMA user_version",[],|r|r.get::<_,u32>(0)).unwrap(),11);
        drop(prior);drop(db);std::fs::remove_dir_all(dir).unwrap();
    }
}
/// SQLite integers are signed 64-bit; reject negative or out-of-range sizes.
pub fn row_size(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<usize> {
    let value: i64 = row.get(index)?;
    usize::try_from(value).map_err(|error| rusqlite::Error::FromSqlConversionFailure(
        index, rusqlite::types::Type::Integer, Box::new(error),
    ))
}
