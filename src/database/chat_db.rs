use crate::database::db_config::DBConfig;
use uuid::Uuid;
use rusqlite::params;
use zerocopy::AsBytes;
use chrono::Utc; // For getting the current UTC timestamp
use serde_json::{ json, Value };
use std::error::Error;
use bytemuck::cast_slice;

impl DBConfig {
    // Function to store a new chat record with embeddings, timestamp, and compressed prompt
    pub fn store_chats(
        &self,
        user_id: &str,
        session_id: &str,
        prompt: &str,
        compressed_prompt_response: &str,
        response: &str,
        embeddings: &[f32],
        request_type: &str
    ) -> Result<(), Box<dyn Error>> {
        // Lock the mutex to access the connection
        let connection = self.connection
            .lock()
            .map_err(|_| "Failed to acquire lock for connection")?;
        let uuid = Uuid::new_v4().to_string();
        let vec_row_id = Self::generate_rowid();

        // Get the current UTC timestamp
        let timestamp = Utc::now().to_rfc3339();
        connection
            .execute(
                "INSERT INTO chats (id, user_id, session_id, vec_row_id, prompt, compressed_prompt_response, response, timestamp, request_type)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    uuid,
                    user_id,
                    session_id,
                    vec_row_id,
                    prompt,
                    compressed_prompt_response,
                    response,
                    timestamp.as_str(),
                    request_type // Store UTC timestamp as TEXT
                ]
            )
            .map_err(|e| format!("Failed to insert chat record: {}", e))?;

        connection
            .execute(
                "INSERT INTO chat_embeddings (rowid,  embeddings)
                VALUES (?, ?)",
                params![vec_row_id, embeddings.as_bytes()]
            )
            .map_err(|e| format!("Failed to insert chat api embeddings record: {}", e))?;
        Ok(())
    }

    pub fn fetch_chats_all(&self) -> Vec<Value> {
        // Lock the mutex to access the connection
        let connection = self.connection.lock().unwrap();

        // Prepare a SQL query to fetch all the chats for a specific session_id and user_id, sorted by timestamp
        let mut stmt = connection
            .prepare(
                "SELECT id, user_id, session_id, prompt, compressed_prompt_response, response, timestamp, request_type
                FROM chats
                WHERE user_id = ?
                ORDER BY timestamp ASC"
            )
            .unwrap();

        // Create a vector to hold the chat entries in JSON format
        let mut chats: Vec<Value> = Vec::new();
        // Execute the query and iterate over the rows, collecting them into the vector
        let chat_iter = stmt
            .query_map(["user_id"], |row| {
                Ok(
                    json!({
                    "id": row.get::<_, String>(0)?,  // id
                    "user_id": row.get::<_, String>(1)?,  // user_id
                    "session_id": row.get::<_, String>(2)?,  // session_id
                    "prompt": row.get::<_, String>(3)?,  // prompt
                    "compressed_prompt_response": row.get::<_, String>(4)?,  // compressed_prompt
                    "response": row.get::<_, String>(5)?,  // response
                    "timestamp": row.get::<_, String>(6)?,  // timestamp
                    "request_type": row.get::<_, String>(7)?,  // timestamp
                    
                })
                )
            })
            .unwrap();

        // Collect all rows into the `chats` vector
        for chat in chat_iter {
            chats.push(chat.unwrap());
        }

        chats
    }

    pub fn fetch_chats_for_session(&self, session_id: &str, skip: u32, limit: u32) -> Vec<Value> {
        // Lock the mutex to access the connection
        let connection = self.connection.lock().unwrap();

        // Prepare a SQL query to fetch all the chats for a specific session_id and user_id, sorted by timestamp
        let mut stmt = connection
            .prepare(
                "SELECT id, user_id, session_id, prompt, response, timestamp, request_type
                 FROM chats 
                 WHERE session_id = ?
                 ORDER BY timestamp DESC
                 LIMIT ?
                 OFFSET ?"
            )
            .unwrap();

        // Create a vector to hold the chat entries
        let mut chats: Vec<Value> = Vec::new();

        // Execute the query and iterate over the rows, collecting them into the vector
        let chat_iter = stmt
            .query_map([session_id, &limit.to_string(), &skip.to_string()], |row| {
                Ok(
                    json!({
                    "id": row.get::<_, String>(0)?,  // id
                    "user_id": row.get::<_, String>(1)?,  // user_id
                    "session_id": row.get::<_, String>(2)?,  // session_id
                    "prompt": row.get::<_, String>(3)?,  // prompt
                    "response": row.get::<_, String>(4)?,  // response
                    "timestamp": row.get::<_, String>(5)?,  // timestamp
                    "request_type": row.get::<_, String>(6)?,  // timestamp
                })
                )
            })
            .unwrap();

        // Collect all rows into the `chats` vector
        for chat in chat_iter {
            chats.push(chat.unwrap());
        }

        chats
    }

    pub fn fetch_chats_for_request_type(
        &self,
        request_type: &str,
        skip: u32,
        limit: u32
    ) -> Vec<Value> {
        // Lock the mutex to access the connection
        let connection = self.connection.lock().unwrap();

        // Prepare a SQL query to fetch all the chats for a specific session_id and user_id, sorted by timestamp
        let mut stmt = connection
            .prepare(
                "SELECT id, user_id, session_id, prompt, response, timestamp, request_type
                 FROM chats 
                 WHERE request_type = ?
                 ORDER BY timestamp DESC
                 LIMIT ?
                 OFFSET ?"
            )
            .unwrap();

        // Create a vector to hold the chat entries
        let mut chats: Vec<Value> = Vec::new();

        // Execute the query and iterate over the rows, collecting them into the vector
        let chat_iter = stmt
            .query_map([request_type, &limit.to_string(), &skip.to_string()], |row| {
                Ok(
                    json!({
                    "id": row.get::<_, String>(0)?,  // id
                    "user_id": row.get::<_, String>(1)?,  // user_id
                    "session_id": row.get::<_, String>(2)?,  // session_id
                    "prompt": row.get::<_, String>(3)?,  // prompt
                    "response": row.get::<_, String>(4)?,  // response
                    "timestamp": row.get::<_, String>(5)?,  // timestamp
                    "request_type": row.get::<_, String>(6)?,  // timestamp
                })
                )
            })
            .unwrap();

        // Collect all rows into the `chats` vector
        for chat in chat_iter {
            chats.push(chat.unwrap());
        }

        chats
    }
    
    pub fn get_last_n_chats(&self, session_id: &str, n: usize) -> Result<Vec<String>, Box<dyn Error>> {
        // Lock the mutex to access the connection
        let connection = self.connection.lock()
            .map_err(|_| "Failed to acquire lock for connection")?;
    
        // Prepare the SQL statement
        let mut stmt = connection.prepare(
            "SELECT compressed_prompt_response 
             FROM chats
             WHERE session_id = ?
             ORDER BY timestamp DESC
             LIMIT ?"
        ).map_err(|e| format!("Failed to prepare query: {}", e))?;
    
        // Execute the query and map the results
        let chats_iter = stmt.query_map(
            params![session_id, n as i64],  // Cast 'n' to i64 for SQLite
            |row| {
                let compressed_prompt_response: String = row.get(0)?;
                Ok(compressed_prompt_response)
            }
        ).map_err(|e| format!("Failed to query last {} chats: {}", n, e))?;
    
        // Collect the results into a vector of strings
        let chats: Vec<String> = chats_iter
            .collect::<Result<Vec<String>, _>>()  // Collect into Vec<String>
            .map_err(|e| format!("Failed to collect chat results: {}", e))?;
    
        Ok(chats)
    }
    
    pub fn query_nearest_embeddings(&self, query_embeddings: Vec<f32>, limit: usize) -> Result<Vec<(i64, f64, String, String)>, Box<dyn std::error::Error>> {
        let connection = self.connection.lock().unwrap();
        let query_embedding_bytes = cast_slice(&query_embeddings);
        // Prepare the SQL statement to find the nearest embeddings
        let mut stmt = connection.prepare(
            r#"
            SELECT
                rowid,
                distance
            FROM chat_embeddings
            WHERE embeddings MATCH ?
            ORDER BY distance
            LIMIT ?
            "#,
        ).map_err(|e| format!("Failed to prepare query: {}", e))?;

        // Execute the query and collect the nearest embeddings
        let nearest_embeddings: Vec<(i64, f64)> = stmt
            .query_map(params![query_embedding_bytes, limit as i64], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?)) // rowid and distance
            })
            .map_err(|e| format!("Failed to execute query: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect nearest embeddings: {}", e))?;



        // Step 2: For each rowid, collect content and file_path from context_children table, and convert to JSON.
        let mut query_context: Vec<(i64, f64, String, String)> = Vec::new();
    
           // For each nearest embedding, fetch the prompt and compressed_prompt
        for (rowid, distance) in nearest_embeddings {
            let mut stmt = connection.prepare(
                r#"
                SELECT
                    prompt, compressed_prompt_response
                FROM context_children
                WHERE vec_rowid = ?
                "#,
            ).map_err(|e| format!("Failed to prepare context query: {}", e))?;

            let context_iter = stmt
                .query_map(params![rowid], |row| {
                    let prompt: String = row.get(0)?;
                    let compressed_prompt: String = row.get(1)?;
                    Ok((rowid, distance, prompt, compressed_prompt))
                })
                .map_err(|e| format!("Failed to execute context query: {}", e))?;

            for context in context_iter {
                query_context.push(context.map_err(|e| format!("Failed to collect context data: {}", e))?);
            }
        }


        Ok(query_context)
    }
}
