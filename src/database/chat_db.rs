
use rusqlite::{ffi::sqlite3_auto_extension, Connection, Result};
use sqlite_vec::sqlite3_vec_init;
use std::fs;
use log::debug;
use std::path::PathBuf;
use chrono::Utc; // For getting the current UTC timestamp
use std::io::prelude::*;
use dirs;
use std::sync::Mutex;
use rusqlite::params;
use zerocopy::AsBytes;

struct PyanoDBConfig{
    pub pyano_db_file: PathBuf
}

// Function to register the sqlite-vec extension
fn register_sqlite_vec(connection: &Connection) -> Result<()> {
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
    }
    Ok(())
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
        let connection = Connection::open(&config.pyano_db_file).unwrap();
                
        // Register the sqlite-vec extension to support vector operations
        register_sqlite_vec(&connection).unwrap();

        connection.execute(
            "
            CREATE virtual table chats using vec0 (
                id INTEGER PRIMARY KEY,
                user_id TEXT NOT NULL,
                session_id TEXT NOT NULL,
                prompt TEXT,
                compressed_prompt TEXT,
                response TEXT,
                embeddings float[512],
                timestamp TEXT  -- Store UTC timestamp as TEXT
            );
            ",
            [],  // Empty array for parameters since none are needed
        ).unwrap();

        PyanoDB {
            connection: Mutex::new(connection),  // Wrapping the connection
        }
    }
    
    // Function to store a new chat record with embeddings, timestamp, and compressed prompt
    pub fn store(&self, user_id: &str, session_id: &str, prompt: &str, compressed_prompt: &str, response: &str, embeddings: &[f32]) {
        
         // Lock the mutex to access the connection
        let connection = self.connection.lock().unwrap();

        // Get the current UTC timestamp
        let timestamp = Utc::now().to_rfc3339();


        connection.execute(
            "INSERT INTO chats (user_id, session_id, prompt, compressed_prompt, response, embeddings, timestamp)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                user_id,
                session_id,
                prompt,
                compressed_prompt,
                response,
                embeddings.as_bytes(),  // Store embeddings as a BLOB
                timestamp.as_str(),      // Store UTC timestamp as TEXT
            ],
        ).unwrap();
    }
    // // Function to query chat records along with embeddings
    // pub fn query_chats(&self) {
    //     let mut statement = self
    //         .connection
    //         .prepare("SELECT id, user_id, session_id, prompt, response, embeddings, timestamp FROM chats")
    //         .unwrap();

    //     while let State::Row = statement.next().unwrap() {
    //         let id: i64 = statement.read(0).unwrap();
    //         let user_id: String = statement.read(1).unwrap();
    //         let session_id: String = statement.read(2).unwrap();
    //         let prompt: String = statement.read(3).unwrap();
    //         let response: String = statement.read(4).unwrap();
            
    //         // Read embeddings as VECTOR
    //         let embeddings: Vec<f32> = statement.read::<Vec<f32>>(5).unwrap();
    //         let timestamp: String = statement.read(6).unwrap();

    //         println!(
    //             "ID: {}, User ID: {}, Session ID: {}, Prompt: {}, Response: {}, Embeddings: {:?}, Timestamp: {}",
    //             id, user_id, session_id, prompt, response, embeddings, timestamp
    //         );
    //     }
    // }
    //  // Function to find the most similar embeddings using cosine similarity
    //  pub fn find_similar(&self, query_embedding: &[f32], top_k: i64) {
    //     let mut statement = self
    //         .connection
    //         .prepare(
    //             "
    //             SELECT id, user_id, session_id, prompt, response, embeddings, timestamp,
    //                 vec_cosine_similarity(embeddings, ?) AS similarity
    //             FROM chats
    //             ORDER BY similarity DESC
    //             LIMIT ?;
    //         ",
    //         )
    //         .unwrap();

    //     statement.bind(1, query_embedding).unwrap();
    //     statement.bind(2, top_k).unwrap();

    //     while let State::Row = statement.next().unwrap() {
    //         let id: i64 = statement.read(0).unwrap();
    //         let user_id: String = statement.read(1).unwrap();
    //         let session_id: String = statement.read(2).unwrap();
    //         let prompt: String = statement.read(3).unwrap();
    //         let response: String = statement.read(4).unwrap();
    //         let embeddings: Vec<f32> = statement.read::<Vec<f32>>(5).unwrap();
    //         let timestamp: String = statement.read(6).unwrap();
    //         let similarity: f32 = statement.read(7).unwrap();

    //         println!(
    //             "ID: {}, User ID: {}, Session ID: {}, Prompt: {}, Response: {}, Embeddings: {:?}, Similarity: {}, Timestamp: {}",
    //             id, user_id, session_id, prompt, response, embeddings, similarity, timestamp
    //         );
    //     }
    // }

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
                response,

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

pub static DB_INSTANCE: Lazy<PyanoDB> = Lazy::new(|| {
    let config = PyanoDBConfig::new();
    PyanoDB::new(&config)
});