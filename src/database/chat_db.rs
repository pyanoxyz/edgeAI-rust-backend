
use rusqlite::{ffi::sqlite3_auto_extension, Connection, Result};
use sqlite_vec::sqlite3_vec_init;
use std::fs;
use log::{debug, info};
use std::path::PathBuf;
use chrono::Utc; // For getting the current UTC timestamp
use dirs;
use std::sync::Mutex;
use rusqlite::params;
use zerocopy::AsBytes;
use uuid::Uuid;

pub struct PyanoDBConfig{
    pub pyano_db_file: PathBuf
}

impl PyanoDBConfig{
    pub fn new() -> Self{
        let home_directory = dirs::home_dir().unwrap();
        let root_pyano_dir = home_directory.join(".pyano");
        let pyano_data_dir = root_pyano_dir.join("database");
        if !pyano_data_dir.exists() {
            fs::create_dir_all(&pyano_data_dir).unwrap();
        }
        let pyano_db_file = pyano_data_dir.join("chats.db");
        debug!("Database file {:?}", pyano_db_file);
        PyanoDBConfig {
            pyano_db_file,
        }
    }
}

// Struct to manage the SQLite connection
pub struct PyanoDB {
    connection: Mutex<Connection>,  // Wrapping the connection in Mutex for thread-safe access
}

impl PyanoDB {
    // Function to create a new database connection (or open existing one)
    pub fn new(config: &PyanoDBConfig) -> Self {
        // Register the sqlite-vec extension to support vector operations
        unsafe {
            sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
        }        
        let connection = Connection::open(&config.pyano_db_file).unwrap();

        connection.execute(
            "
            CREATE TABLE IF NOT EXISTS chats (
                id TEXT PRIMARY KEY,  -- UUID as primary key
                user_id TEXT NOT NULL,
                session_id TEXT NOT NULL,
                prompt TEXT,
                compressed_prompt TEXT,
                response TEXT,
                timestamp TEXT,
                request_type TEXT
            );
            ",
            [],  // Empty array for parameters since none are needed
        ).unwrap();
        // Check if the 'chat_embeddings' virtual table already exists
        let table_exists: bool = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='chat_embeddings';",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0) > 0;

        // Create the 'chat_embeddings' table only if it doesn't exist
        if !table_exists {
        connection.execute(
            "
            CREATE VIRTUAL TABLE chat_embeddings USING vec0 (id TEXT PRIMARY KEY, session_id TEXT, embeddings float[384]);
            ",
            [],
        ).unwrap();
        }

        PyanoDB {
            connection: Mutex::new(connection),  // Wrapping the connection
        }
    }
    
    // Function to store a new chat record with embeddings, timestamp, and compressed prompt
    pub fn store(&self, user_id: &str, session_id: &str, prompt: &str, compressed_prompt: &str, response: &str, embeddings: &[f32], request_type: &str) {
        
         // Lock the mutex to access the connection
        let connection = self.connection.lock().unwrap();
        let uuid = Uuid::new_v4().to_string();

        // Get the current UTC timestamp
        let timestamp = Utc::now().to_rfc3339();


        connection.execute(
            "INSERT INTO chats (id, user_id, session_id, prompt, compressed_prompt, response, timestamp, request_type)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                uuid,
                user_id,
                session_id,
                prompt,
                compressed_prompt,
                response,
                timestamp.as_str(),
                request_type     // Store UTC timestamp as TEXT
            ],
        ).unwrap();

        connection.execute(
            "INSERT INTO chat_embeddings (id, session_id,  embeddings)
             VALUES (?, ?, ?)",
            params![
                uuid,
                session_id,
                embeddings.as_bytes()         
                ],
        ).unwrap();

    }
   
    // Example of how to use the RwLock for reading
    pub fn query_nearest_embeddings(&self, query_embeddings: Vec<f32>, limit: usize) -> Result<Vec<(i64, f64, String, String, String)>> {
        let connection = self.connection.lock().unwrap();
        let mut stmt = connection.prepare(
            "
            SELECT
                id,
                distance,
                prompt,
                compressed_prompt,
                response
            FROM chats
            WHERE embeddings MATCH ?1
            ORDER BY distance
            LIMIT ?2
            ",
        )?;

        let result = stmt.query_map(
            params![query_embeddings.as_bytes(), limit as i64], 
            |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?))  // Get the ID and similarity score
        )?.collect::<Result<Vec<_>, _>>()?;

        Ok(result)
    }
}
// Create a singleton instance of the database connection
use once_cell::sync::Lazy;

use crate::request_type;

pub static DB_INSTANCE: Lazy<PyanoDB> = Lazy::new(|| {
    let config = PyanoDBConfig::new();
    PyanoDB::new(&config)
});