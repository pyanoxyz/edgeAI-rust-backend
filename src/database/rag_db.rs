use crate::database::db_config::DBConfig;
use uuid::Uuid;
use chrono::Utc; // For getting the current UTC timestamp
use serde_json::{json, Value};
use rand::Rng;
use rusqlite::params;
use std::error::Error;
use log::info;
impl DBConfig{

    pub fn generate_rowid() -> u64 {
        let mut rng = rand::thread_rng();
        rng.gen_range(1_000_000_000_000_000..=9_999_999_999_999_999)
    }

    // Function to store a new chat record with embeddings, timestamp, and compressed prompt
    pub fn store_parent_context(&self, user_id: &str, session_id: &str, parent_path: &str, filetype: &str, category: &str) {
            
            // Lock the mutex to access the connection
        let connection = self.connection.lock().unwrap();
        let uuid = Uuid::new_v4().to_string();

        // Get the current UTC timestamp
        let timestamp = Utc::now().to_rfc3339();
        connection.execute(
            "INSERT INTO context_parent (id, user_id, session_id, parent_path, filetype, category, timestamp)
                VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                uuid,
                user_id,
                session_id,
                parent_path,
                filetype, 
                category,            
                timestamp.as_str(),
            ],
        ).unwrap();

    }
    
    pub fn delete_parent_context(&self, parent_path: &str) -> Result<(), rusqlite::Error> {
        // Lock the mutex to access the connection
        let connection = self.connection.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
        
        // Execute the DELETE query and return the result
        connection.execute(
            "DELETE FROM context_parent WHERE parent_path = ?",
            params![parent_path],
        )?;
        
        Ok(())
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
        vec_row_id: u64
    ) {
        // Lock the mutex to access the connection
        let connection = self.connection.lock().unwrap();
        
        // Generate UUIDs for the child and the vector embedding
        let uuid = Uuid::new_v4().to_string();
    
        // Get the current UTC timestamp
        let timestamp = Utc::now().to_rfc3339();
    
        // Insert into context_children
        info!("Storing {} in the context_children with user_id {} parent_path {}", vec_row_id, user_id, parent_path);
        connection.execute(
            "INSERT INTO context_children (
                id, user_id, session_id, parent_path, chunk_type, content, compressed_content,
                end_line, file_path, start_line, vec_row_id, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
    
        // // Insert into context_embeddings
        // connection.execute(
        //     "INSERT INTO context_embeddings (rowid, embeddings) VALUES (?, ?)",
        //     params![
        //         vec_row_id,
        //         embeddings.as_bytes(),  // You can pass the float array directly in rusqlite
        //     ],
        // ).unwrap();
    }


    pub fn delete_children_context_by_parent_path(
        &self,
        user_id: &str,
        session_id: &str,
        parent_path: &str,
    ) -> Result<Vec<u64>, rusqlite::Error> {
        // Lock the mutex to access the connection
        let mut connection = self.connection.lock().unwrap();
    
        // Begin a transaction to ensure both deletions are atomic
        let tx = connection.transaction()?;

        // Collect `vec_row_id` within its own scope to avoid borrowing `tx` for too long
        let vec_row_ids: Vec<u64> = {
            let mut stmt = tx.prepare(
                "SELECT vec_row_id FROM context_children 
                WHERE user_id = ? AND session_id = ? AND parent_path = ?"
            )?;
        
            let ids = stmt.query_map(
                params![user_id, session_id, parent_path],
                |row| row.get(0),
            )?
            .filter_map(Result::ok)
            .collect();
            ids
        };
        // Delete from `context_children` table where parent_path matches
        info!("vec_row_ids to be deleted are {:?}", vec_row_ids.len());
        tx.execute(
            "DELETE FROM context_children 
             WHERE user_id = ? AND session_id = ? AND parent_path = ?",
            params![user_id, session_id, parent_path],
        )?;
    
        // // Delete from `context_embeddings` for the corresponding vec_row_ids
        // for row_id in &vec_row_ids {
        //     tx.execute(
        //         "DELETE FROM context_embeddings WHERE vec_row_id = ?",
        //         params![row_id],
        //     )?;
        // }
    
        // Commit the transaction to apply the changes
        tx.commit()?;
    
        Ok(vec_row_ids)
    }
    

    pub fn delete_children_context_by_file_path(
        &self,
        user_id: &str,
        session_id: &str,
        file_path: &str,
    ) -> Result<Vec<u64>, rusqlite::Error> {
        // Lock the mutex to access the connection
        let mut connection = self.connection.lock().unwrap();
    
        // Begin a transaction to ensure both deletions are atomic
        let tx = connection.transaction()?;

        // Collect `vec_row_id` within its own scope to avoid borrowing `tx` for too long
        let vec_row_ids: Vec<u64> = {
            let mut stmt = tx.prepare(
                "SELECT vec_row_id FROM context_children 
                WHERE user_id = ? AND session_id = ? AND file_path = ?"
            )?;
        
            let ids = stmt.query_map(
                params![user_id, session_id, file_path],
                |row| row.get(0),
            )?
            .filter_map(Result::ok)
            .collect();
            ids
        };
        // Delete from `context_children` table where parent_path matches
        info!("vec_row_ids to be deleted are {:?}", vec_row_ids.len());
        tx.execute(
            "DELETE FROM context_children 
             WHERE user_id = ? AND session_id = ? AND file_path = ?",
            params![user_id, session_id, file_path],
        )?;
    
        // // Delete from `context_embeddings` for the corresponding vec_row_ids
        // for row_id in &vec_row_ids {
        //     tx.execute(
        //         "DELETE FROM context_embeddings WHERE vec_row_id = ?",
        //         params![row_id],
        //     )?;
        // }
    
        // Commit the transaction to apply the changes
        tx.commit()?;
    
        Ok(vec_row_ids)
    }
    



    pub fn fetch_session_context_files(&self, user_id: &str, session_id: &str) -> Vec<Value> {
        // Lock the mutex to access the connection
        let connection = self.connection.lock().unwrap();
    
        // Prepare a SQL query to fetch all the chats for a specific session_id and user_id, sorted by timestamp
        let mut stmt = connection
            .prepare(
                "SELECT user_id, session_id, parent_path, filetype, category, timestamp 
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
                    "filetype": row.get::<_, String>(3)?,  // prompt
                    "category": row.get::<_, String>(4)?,  // prompt
                    "timestamp": row.get::<_, String>(5)?,  // timestamp
                }))
            })
            .unwrap();
    
        // Collect all rows into the `chats` vector
        for chat in context_iter {
            context_files.push(chat.unwrap());
        }
    
        context_files
    }

    //fetch a filepath for a paritcular session if present
    pub fn fetch_path_session(&self, user_id: &str, session_id: &str, parent_path: &str) -> Option<Value> {
        // Lock the mutex to access the connection
        let connection = self.connection.lock().unwrap();
    
        // Prepare a SQL query to fetch all the chats for a specific session_id and user_id, sorted by timestamp
        let mut stmt = connection
            .prepare(
                "SELECT user_id, session_id, parent_path, filetype, category, timestamp 
                 FROM context_parent 
                 WHERE user_id = ? and session_id = ? and parent_path = ?
                 ORDER BY timestamp ASC",
            )
            .unwrap();
    
    
        // Execute the query and iterate over the rows, collecting them into the vector
        let result = stmt
            .query_row([user_id, session_id, parent_path], |row| {
                Ok(json!({
                    "user_id": row.get::<_, String>(0)?,  // user_id
                    "session_id": row.get::<_, String>(1)?,  // session_id
                    "path": row.get::<_, String>(2)?,  // parent_path
                    "filetype": row.get::<_, String>(3)?,  // filetype
                    "category": row.get::<_, String>(4)?,  // category
                    "timestamp": row.get::<_, String>(5)?,  // timestamp
                }))
            });
    
        // Return the result as an Option, with None if no row is found
        result.ok()
    }


    pub fn update_session_context_timestamp(&self, user_id: &str, session_id: &str, parent_path: &str) -> Result<(), rusqlite::Error> {
        // Lock the mutex to access the connection
        let connection = self.connection.lock().unwrap();
        let timestamp = Utc::now().to_rfc3339();

        // Prepare the SQL query to update the timestamp for the specified entry
        let result = connection.execute(
            "UPDATE context_parent 
             SET timestamp = ? 
             WHERE user_id = ? AND session_id = ? AND parent_path = ?",
            params![timestamp.as_str(), user_id, session_id, parent_path],
        );

        // Return the number of rows affected or an error
        Ok(())
    }

    pub fn get_row_ids(&self, row_ids: Vec<u64>) ->  Result<Vec<(String, String, String, String)>, Box<dyn Error>> {
        // Collect context data for each nearest embedding
        let mut chunks: Vec<(String, String, String, String)> = Vec::new();
        let connection = self.connection.lock().map_err(|e| {
            format!("Failed to acquire lock: {}", e)
        })?;
        for rowid in row_ids {
            let mut stmt = connection.prepare(
                r#"
                SELECT
                    file_path,
                    chunk_type,
                    content,
                    session_id
                FROM context_children
                WHERE vec_row_id = ?
                "#,
            )?;

            let context_iter = stmt.query_map(params![rowid], |row| {
                let file_path: String = row.get(0)?;
                let chunk_type: String = row.get(1)?;
                let content: String = row.get(2)?;
                let session_id: String = row.get(3)?;

                Ok((file_path, chunk_type, content, session_id))
            })?;

            for context in context_iter {
                chunks.push(context?);
            }
        }

        Ok(chunks)
    }


}