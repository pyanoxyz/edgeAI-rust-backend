

use crate::database::db_config::DBConfig;
use crate::embeddings;
use uuid::Uuid;
use rusqlite::params;
use rusqlite::Result;
use zerocopy::AsBytes;
use chrono::Utc; // For getting the current UTC timestamp
use serde_json::{json, Value};


impl DBConfig{
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
        end_line: usize, 
        start_line: usize, 
        file_path: &str,
        embeddings: &[f32],
    ) {
        // Lock the mutex to access the connection
        let connection = self.connection.lock().unwrap();
        
        // Generate UUIDs for the child and the vector embedding
        let uuid = Uuid::new_v4().to_string();
        let vec_row_id = Uuid::new_v4().to_string();
    
        // Get the current UTC timestamp
        let timestamp = Utc::now().to_rfc3339();
    
        // Insert into context_children
        connection.execute(
            "INSERT INTO context_children (
                id, user_id, session_id, parent_path, chunk_type, content, 
                end_line, file_path, start_line, vec_rowid, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                uuid,
                user_id,
                session_id,
                parent_path,
                chunk_type,
                content,
                end_line,
                file_path,
                start_line,
                vec_row_id,
                timestamp.as_str(),
            ],
        ).unwrap();
    
        // Insert into context_embeddings
        connection.execute(
            "INSERT INTO context_embeddings (id, embeddings) VALUES (?, ?)",
            params![
                vec_row_id,
                embeddings.as_bytes(),  // You can pass the float array directly in rusqlite
            ],
        ).unwrap();
    }


    // pub fn fetch_chats_for_user(&self, user_id: &str) -> Vec<Value> {
    //     // Lock the mutex to access the connection
    //     let connection = self.connection.lock().unwrap();
    
    //     // Prepare a SQL query to fetch all the chats for a specific session_id and user_id, sorted by timestamp
    //     let mut stmt = connection
    //         .prepare(
    //             "SELECT id, user_id, session_id, prompt, compressed_prompt, response, timestamp 
    //              FROM chats 
    //              WHERE user_id = ?
    //              ORDER BY timestamp ASC",
    //         )
    //         .unwrap();
    
    //     // Create a vector to hold the chat entries in JSON format
    //     let mut chats: Vec<Value> = Vec::new();
    
    //     // Execute the query and iterate over the rows, collecting them into the vector
    //     let chat_iter = stmt
    //         .query_map([user_id], |row| {
    //             Ok(json!({
    //                 "id": row.get::<_, String>(0)?,  // id
    //                 "user_id": row.get::<_, String>(1)?,  // user_id
    //                 "session_id": row.get::<_, String>(2)?,  // session_id
    //                 "prompt": row.get::<_, String>(3)?,  // prompt
    //                 "compressed_prompt": row.get::<_, String>(4)?,  // compressed_prompt
    //                 "response": row.get::<_, String>(5)?,  // response
    //                 "timestamp": row.get::<_, String>(6)?,  // timestamp
    //             }))
    //         })
    //         .unwrap();
    
    //     // Collect all rows into the `chats` vector
    //     for chat in chat_iter {
    //         chats.push(chat.unwrap());
    //     }
    
    //     chats
    // }

    // pub fn fetch_chats_for_session_and_user(&self, session_id: &str, user_id: &str) -> Vec<Value> {
    //     // Lock the mutex to access the connection
    //     let connection = self.connection.lock().unwrap();
    
    //     // Prepare a SQL query to fetch all the chats for a specific session_id and user_id, sorted by timestamp
    //     let mut stmt = connection
    //         .prepare(
    //             "SELECT id, user_id, session_id, prompt, compressed_prompt, response, timestamp 
    //              FROM chats 
    //              WHERE session_id = ? AND user_id = ?
    //              ORDER BY timestamp ASC",
    //         )
    //         .unwrap();
    
    //     // Create a vector to hold the chat entries
    //     let mut chats: Vec<Value> = Vec::new();
    
    //     // Execute the query and iterate over the rows, collecting them into the vector
    //     let chat_iter = stmt
    //         .query_map([session_id, user_id], |row| {
    //             Ok(json!({
    //                 "id": row.get::<_, String>(0)?,  // id
    //                 "user_id": row.get::<_, String>(1)?,  // user_id
    //                 "session_id": row.get::<_, String>(2)?,  // session_id
    //                 "prompt": row.get::<_, String>(3)?,  // prompt
    //                 "compressed_prompt": row.get::<_, String>(4)?,  // compressed_prompt
    //                 "response": row.get::<_, String>(5)?,  // response
    //                 "timestamp": row.get::<_, String>(6)?,  // timestamp
    //             }))
    //         })
    //         .unwrap();
    
    //     // Collect all rows into the `chats` vector
    //     for chat in chat_iter {
    //         chats.push(chat.unwrap());
    //     }
    
    //     chats
    // }

    // // Example of how to use the RwLock for reading
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