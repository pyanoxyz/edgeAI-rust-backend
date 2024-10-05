

use crate::database::db_config::DBConfig;
use uuid::Uuid;
use rusqlite::params;
use zerocopy::AsBytes;
use chrono::Utc; // For getting the current UTC timestamp
use serde_json::{json, Value};
use std::error::Error;


impl DBConfig{
    // Function to store a new chat record with embeddings, timestamp, and compressed prompt
    pub fn store_chats(&self, user_id: &str, session_id: &str, prompt: &str, compressed_prompt: &str, response: &str, embeddings: &[f32], request_type: &str) -> Result<(), Box<dyn Error>> {
            
        // Lock the mutex to access the connection
        let connection = self.connection.lock()
        .map_err(|_| "Failed to acquire lock for connection")?;        
        let uuid = Uuid::new_v4().to_string();
        let vec_row_id = Self::generate_rowid();

        // Get the current UTC timestamp
        let timestamp = Utc::now().to_rfc3339();
        connection.execute(
            "INSERT INTO chats (id, user_id, session_id, vec_row_id, prompt, compressed_prompt, response, timestamp, request_type)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                uuid,
                user_id,
                session_id,
                vec_row_id,
                prompt,
                compressed_prompt,
                response,
                timestamp.as_str(),
                request_type     // Store UTC timestamp as TEXT
            ],
        ).map_err(|e| format!("Failed to insert chat record: {}", e))?;

        connection.execute(
            "INSERT INTO chat_embeddings (rowid,  embeddings)
                VALUES (?, ?)",
            params![
                vec_row_id,
                embeddings.as_bytes()         
                ],
        ).map_err(|e| format!("Failed to insert chat api embeddings record: {}", e))?;
        Ok(())

    }
     
    pub fn fetch_chats_for_user(&self, user_id: &str) -> Vec<Value> {
        // Lock the mutex to access the connection
        let connection = self.connection.lock().unwrap();
    
        // Prepare a SQL query to fetch all the chats for a specific session_id and user_id, sorted by timestamp
        let mut stmt = connection
            .prepare(
                "SELECT id, user_id, session_id, prompt, compressed_prompt, response, timestamp 
                 FROM chats 
                 WHERE user_id = ?
                 ORDER BY timestamp ASC",
            )
            .unwrap();
    
        // Create a vector to hold the chat entries in JSON format
        let mut chats: Vec<Value> = Vec::new();
    
        // Execute the query and iterate over the rows, collecting them into the vector
        let chat_iter = stmt
            .query_map([user_id], |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,  // id
                    "user_id": row.get::<_, String>(1)?,  // user_id
                    "session_id": row.get::<_, String>(2)?,  // session_id
                    "prompt": row.get::<_, String>(3)?,  // prompt
                    "compressed_prompt": row.get::<_, String>(4)?,  // compressed_prompt
                    "response": row.get::<_, String>(5)?,  // response
                    "timestamp": row.get::<_, String>(6)?,  // timestamp
                }))
            })
            .unwrap();
    
        // Collect all rows into the `chats` vector
        for chat in chat_iter {
            chats.push(chat.unwrap());
        }
    
        chats
    }

    pub fn fetch_chats_for_session_and_user(&self, session_id: &str, user_id: &str) -> Vec<Value> {
        // Lock the mutex to access the connection
        let connection = self.connection.lock().unwrap();
    
        // Prepare a SQL query to fetch all the chats for a specific session_id and user_id, sorted by timestamp
        let mut stmt = connection
            .prepare(
                "SELECT id, user_id, session_id, prompt, compressed_prompt, response, timestamp 
                 FROM chats 
                 WHERE session_id = ? AND user_id = ?
                 ORDER BY timestamp ASC",
            )
            .unwrap();
    
        // Create a vector to hold the chat entries
        let mut chats: Vec<Value> = Vec::new();
    
        // Execute the query and iterate over the rows, collecting them into the vector
        let chat_iter = stmt
            .query_map([session_id, user_id], |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,  // id
                    "user_id": row.get::<_, String>(1)?,  // user_id
                    "session_id": row.get::<_, String>(2)?,  // session_id
                    "prompt": row.get::<_, String>(3)?,  // prompt
                    "compressed_prompt": row.get::<_, String>(4)?,  // compressed_prompt
                    "response": row.get::<_, String>(5)?,  // response
                    "timestamp": row.get::<_, String>(6)?,  // timestamp
                }))
            })
            .unwrap();
    
        // Collect all rows into the `chats` vector
        for chat in chat_iter {
            chats.push(chat.unwrap());
        }
    
        chats
    }

    // Example of how to use the RwLock for reading
    // pub fn query_nearest_embeddings(&self, query_embeddings: Vec<f32>, limit: usize) -> Result<Vec<(i64, f64, String, String, String)>> {
    // let connection = self.connection.lock().unwrap();
    // let mut stmt = connection.prepare(
    //     "
    //     SELECT
    //         id,
    //         distance,
    //         prompt,
    //         compressed_prompt,
    //         response
    //     FROM chats
    //     WHERE embeddings MATCH ?1
    //     ORDER BY distance
    //     LIMIT ?2
    //     ",
    // )?;

    // let result = stmt.query_map(
    //     params![query_embeddings.as_bytes(), limit as i64], 
    //     |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?))  // Get the ID and similarity score
    // )?.collect::<Result<Vec<_>, _>>()?;

    // Ok(result)
    // }

}