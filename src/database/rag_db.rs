

use crate::database::db_config::DBConfig;
use uuid::Uuid;
use rusqlite::params;
use rusqlite::Result;
use zerocopy::AsBytes;
use chrono::Utc; // For getting the current UTC timestamp
use serde_json::{json, Value};
use rand::Rng;

impl DBConfig{


    fn generate_rowid() -> u64 {
        let mut rng = rand::thread_rng();
        rng.gen_range(1_000_000_000_000_000..=9_999_999_999_999_999)
    }

    // Function to store a new chat record with embeddings, timestamp, and compressed prompt
    pub fn store_parent_context(&self, user_id: &str, session_id: &str, parent_path: &str) {
            
        // Lock the mutex to access the connection
    let connection = self.connection.lock().unwrap();
    let uuid = Uuid::new_v4().to_string();

    // Get the current UTC timestamp
    let timestamp = Utc::now().to_rfc3339();
    connection.execute(
        "INSERT INTO context_parent (id, user_id, session_id, parent_path, timestamp)
            VALUES (?, ?, ?, ?, ?)",
        params![
            uuid,
            user_id,
            session_id,
            parent_path,            
            timestamp.as_str(),
        ],
    ).unwrap();

    }
    
    pub fn store_children_context(
        &self, 
        user_id: &str, 
        session_id: &str, 
        parent_path: &str, 
        chunk_type: &str, 
        content: &str,
        compressed_content: &str,
        end_line: usize, 
        start_line: usize, 
        file_path: &str,
        embeddings: &[f32],
    ) {
        // Lock the mutex to access the connection
        let connection = self.connection.lock().unwrap();
        
        // Generate UUIDs for the child and the vector embedding
        let uuid = Uuid::new_v4().to_string();
        let vec_row_id = Self::generate_rowid();
    
        // Get the current UTC timestamp
        let timestamp = Utc::now().to_rfc3339();
    
        // Insert into context_children
        connection.execute(
            "INSERT INTO context_children (
                id, user_id, session_id, parent_path, chunk_type, content, compressed_content,
                end_line, file_path, start_line, vec_rowid, timestamp
            ) VALUES (?, ?, ?, ?,?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                uuid,
                user_id,
                session_id,
                parent_path,
                chunk_type,
                content,
                compressed_content,
                end_line,
                file_path,
                start_line,
                vec_row_id,
                timestamp.as_str(),
            ],
        ).unwrap();
    
        // Insert into context_embeddings
        connection.execute(
            "INSERT INTO context_embeddings (rowid, embeddings) VALUES (?, ?)",
            params![
                vec_row_id,
                embeddings.as_bytes(),  // You can pass the float array directly in rusqlite
            ],
        ).unwrap();
    }


    pub fn fetch_session_context_files(&self, user_id: &str, session_id: &str) -> Vec<Value> {
        // Lock the mutex to access the connection
        let connection = self.connection.lock().unwrap();
    
        // Prepare a SQL query to fetch all the chats for a specific session_id and user_id, sorted by timestamp
        let mut stmt = connection
            .prepare(
                "SELECT user_id, session_id, parent_path, timestamp 
                 FROM context_parent 
                 WHERE user_id = ? and session_id = ?
                 ORDER BY timestamp ASC",
            )
            .unwrap();
    
        // Create a vector to hold the chat entries in JSON format
        let mut context_files: Vec<Value> = Vec::new();
    
        // Execute the query and iterate over the rows, collecting them into the vector
        let context_iter = stmt
            .query_map([user_id, session_id], |row| {
                Ok(json!({
                    "user_id": row.get::<_, String>(0)?,  // user_id
                    "session_id": row.get::<_, String>(1)?,  // session_id
                    "path": row.get::<_, String>(2)?,  // prompt
                    "timestamp": row.get::<_, String>(3)?,  // timestamp
                }))
            })
            .unwrap();
    
        // Collect all rows into the `chats` vector
        for chat in context_iter {
            context_files.push(chat.unwrap());
        }
    
        context_files
    }

    pub fn query_session_context(
        &self,
        user_id: &str,
        session_id: &str,
        query_embeddings: Vec<f32>    
    ) -> Result<Vec<serde_json::Value>> {
        let connection = self.connection.lock().unwrap();
        
        // Step 1: Query nearest embeddings based on the vector search.
        let nearest_embeddings: Vec<(i64, f64)> = connection
            .prepare(
                r#"
                SELECT
                    rowid,
                    distance
                FROM context_embeddings
                WHERE embeddings MATCH ?1
                ORDER BY distance
                LIMIT 30
                "#,
            )?
            .query_map([query_embeddings.as_bytes()], |row| {
                Ok((row.get(0)?, row.get(1)?)) // rowid and distance
            })?
            .collect::<Result<Vec<_>, _>>()?;
    
        // Step 2: For each rowid, collect content and file_path from context_children table, and convert to JSON.
        let mut context_files: Vec<serde_json::Value> = Vec::new();
    
        for (rowid, distance) in nearest_embeddings {
            let mut stmt = connection.prepare(
                r#"
                SELECT
                    content,
                    file_path,
                    chunk_type,
                    timestamp
                FROM context_children
                WHERE vec_rowid = ?1
                "#,
            )?;
    
            let context_iter = stmt.query_map([rowid], |row| {
                Ok(json!({
                    "rowid": rowid,
                    "distance": distance,
                    "content": row.get::<_, String>(0)?,   // content
                    "file_path": row.get::<_, String>(1)?, // file_path
                    "chunk_type": row.get::<_, String>(2)?, // chunk_type
                    "timestamp": row.get::<_, String>(3)?,  // timestamp
                }))
            })?;
    
            // Collect all rows into the `context_files` vector as JSON objects.
            for context in context_iter {
                context_files.push(context?);
            }
        }
    
        Ok(context_files)
    }
    
    // let result = stmt.query_map(
    //     params![query_embeddings.as_bytes(), limit as i64], 
    //     |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?))  // Get the ID and similarity score
    // )?.collect::<Result<Vec<_>, _>>()?;

    // Ok(result)
    // }

}