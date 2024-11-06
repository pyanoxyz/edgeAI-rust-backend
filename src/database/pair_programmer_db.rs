

use chrono::Utc; // For getting the current UTC timestamp
use rusqlite::params;
use serde_json::{json, Value};
use crate::database::db_config::DBConfig;
use crate::pair_programmer::types::{StepChat, PairProgrammerStep, PairProgrammerStepRaw};
use log::info;
use std::collections::HashMap;
use std::error::Error;

impl DBConfig{
    pub fn fetch_single_step(
        &self,
        pair_programmer_id: &str,
        step_number: &str,
    ) -> Result<PairProgrammerStep, Box<dyn Error>> {
        // Lock the mutex to access the connection
         // Lock the mutex to access the connection
         let connection = self.pair_programmer_connection.lock()
         .map_err(|_| "Failed to acquire lock for connection")?;

        // Construct the step_id from pair_programmer_id and step_number
        let step_id = format!("{}_{}", pair_programmer_id, step_number);
    
        // Query to fetch a single step
        let mut stmt = connection.prepare(
            "SELECT heading, action, details, executed, response, chat
             FROM pp_steps
             WHERE id = ?",
        ).map_err(|e| format!("Failed to prepare statement: {}", e))?;
    
        // Execute the query and map the result
        let step = stmt.query_row(rusqlite::params![step_id], |row| {
            let heading: String = row.get(0)?;
            let action: String = row.get(1)?;
            let details: String = row.get(2)?;
            let executed: bool = row.get(3)?;
            let response: String = row.get(4)?;
            let chat: String = row.get(5)?;
    
        let chats: Vec<StepChat> = serde_json::from_str(&chat).map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?;

        let step_details: HashMap<String, String> = serde_json::from_str(&details).map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?;

        
        // Create PairProgrammerStep using the constructor
        Ok(PairProgrammerStep::new(
            step_number.to_string(),
            heading,
            action,
            step_details,
            response,
            executed,
            chats,
            ))
        }).map_err(|e| format!("Failed to fetch pair_programmer_step record: {}", e))?;
    
        Ok(step)
    }

    // pub fn generate_pair_programmer_id() -> u64 {
    //     let mut rng = rand::thread_rng();
    //     rng.gen_range(1_000_000_000_000_000..=9_999_999_999_999_999)
    // }

    // Function to store a new chat record with embeddings, timestamp, and compressed prompt
    pub fn store_new_pair_programming_session(
        &self, 
        user_id: &str, 
        session_id: &str, 
        pair_programmer_id: &str, 
        task: &str, 
        steps: &Vec<PairProgrammerStepRaw>
    ) -> Result<(), Box<dyn Error>> {
        
        // Lock the mutex to access the connection
        let connection = self.pair_programmer_connection.lock()
            .map_err(|_| "Failed to acquire lock for connection")?;
        
        let serialized_steps = serde_json::to_string(&steps)
            .map_err(|_| "Failed to serialize steps")?;
        
        // Get the current UTC timestamp
        let timestamp = Utc::now().to_rfc3339();
        
        connection.execute(
            "INSERT INTO pair_programmer (id, user_id, session_id, task, steps, timestamp)
                VALUES (?, ?, ?, ?, ?, ?)",
            params![
                pair_programmer_id,
                user_id,
                session_id,
                task,
                serialized_steps,
                timestamp.as_str(),
            ],
        ).map_err(|e| format!("Failed to insert pair_programmer record: {}", e))?;
        
        for (index, step) in steps.iter().enumerate() {
            let step_id = format!("{}_{}", pair_programmer_id, index + 1);
            let serialized_chat = serde_json::to_string(&Vec::<StepChat>::new())
                .map_err(|_| "Failed to serialize chat")?;
            
            let serialized_details = serde_json::to_string(&step.details)
                .map_err(|_| "Failed to serialize details")?;
            connection.execute(
                "INSERT INTO pp_steps (id, pair_programmer_id, user_id, session_id, heading, action, details, executed, response, timestamp, chat)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    step_id, 
                    pair_programmer_id,
                    user_id,
                    session_id,
                    step.heading,
                    step.action,
                    serialized_details,
                    0,
                    "",
                    timestamp.as_str(),
                    serialized_chat,
                ],
            ).map_err(|e| format!("Failed to insert pp_steps record: {}", e))?;
            
            info!("Inserting step {:?}", step);
        }
        
        Ok(())
    }

    pub fn fetch_task_from_pair_programmer(
        &self, 
        pair_programmer_id: &str
    ) -> Result<String, Box<dyn Error>> {
        
        // Lock the mutex to access the connection
        let connection = self.pair_programmer_connection.lock()
            .map_err(|_| "Failed to acquire lock for connection")?;
        
        // Query to fetch only the task from pair_programmer table
        let mut stmt = connection.prepare(
            "SELECT task FROM pair_programmer WHERE id = ?"
        )?;
        
        // Execute the query and fetch the result
        let task: String = stmt.query_row([pair_programmer_id], |row| row.get(0))
            .map_err(|e| format!("Failed to fetch task for session_id {}: {}", pair_programmer_id, e))?;
        
        Ok(task)
    }

    pub fn fetch_steps(&self, pair_programmer_id: &str) -> Vec<Value> {
        // Lock the mutex to access the connection
        let connection = self.pair_programmer_connection.lock().unwrap();
        
        // Prepare a SQL query to fetch all the steps for a specific pair_programming_id
        let mut stmt = connection
            .prepare(
                "SELECT id, user_id, session_id, heading, action, details, executed, response, chat, timestamp 
                 FROM pp_steps 
                 WHERE pair_programmer_id = ?",
            )
            .unwrap();
        
        // Create a vector to hold the steps in JSON format
        let mut steps: Vec<Value> = Vec::new();
        
        // Execute the query and iterate over the rows, collecting them into the vector
        let step_iter = stmt
            .query_map([pair_programmer_id], |row| {
                let details_str: String = row.get(5)?;
                let details: HashMap<String, String> = serde_json::from_str(&details_str).unwrap_or_default();
                Ok(json!({
                    "step_id": row.get::<_, String>(0)?,         // step_id
                    "user_id": row.get::<_, String>(1)?,         // user_id
                    "session_id": row.get::<_, String>(2)?,      // session_id
                    "heading": row.get::<_, String>(3)?,         // heading
                    "action": row.get::<_, String>(4)?,   // function_call
                    "details": details,   // function_call
                    "executed": row.get::<_, bool>(6)?,          // executed (boolean)
                    "response": row.get::<_, String>(7)?,        // response
                    "chat": row.get::<_, String>(8)?,            // chat (assuming it's serialized as JSON or a string)
                    "timestamp": row.get::<_, String>(9)?,       // timestamp
                }))
            })
            .unwrap();
        
        // Collect all rows into the `steps` vector
        for step in step_iter {
            steps.push(step.unwrap());
        }
        
        steps
    }

    pub fn update_step_execution(&self, pair_programmer_id: &str, step_number: &str, response: &str) ->Result<(), rusqlite::Error>  {
            
        // Lock the mutex to access the connection
        let connection = self.pair_programmer_connection.lock().unwrap();
        let step_id = format!("{}_{}", pair_programmer_id, step_number);

        let sql = "UPDATE pp_steps SET response = ?1, executed = 1 WHERE id = ?2";
        connection.execute(sql, params![response, step_id])?;
        Ok(())

    }
    

    pub fn update_step_chat(&self, pair_programmer_id: &str, step_number: &str, prompt: &str, response: &str) ->Result<(), rusqlite::Error>  {
            
        // Lock the mutex to access the connection
        let connection = self.pair_programmer_connection.lock().unwrap();
        let step_id = format!("{}_{}", pair_programmer_id, step_number);

        // Fetch the current chat from the step
        let mut stmt = connection.prepare("SELECT chat FROM pp_steps WHERE id = ?1")?;
        let chat_json: String = stmt.query_row(params![step_id], |row| row.get(0))?;

        let mut chat_history: Vec<StepChat> = serde_json::from_str(&chat_json).unwrap_or_else(|_| Vec::new());

        // Append the new chat message
        chat_history.push(StepChat{prompt: prompt.to_string(), response: response.to_string()});

        // Serialize the updated chat array
        let updated_chat_json = serde_json::to_string(&chat_history).unwrap();

        // Update the step with the new response and chat history
        let sql = "UPDATE pp_steps SET chat = ?1 WHERE id = ?2";
        connection.execute(sql, params![updated_chat_json, step_id])?;

        Ok(())

    }

    pub fn step_chat_string(&self, pair_programmer_id: &str, step_number: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Lock the mutex to access the connection
        let connection = self.pair_programmer_connection.lock().unwrap();
        let step_id = format!("{}_{}", pair_programmer_id, step_number);
    
        // Fetch the current chat from the step
        let mut stmt = connection.prepare("SELECT chat FROM pp_steps WHERE id = ?1")?;
        let chat_json: String = stmt.query_row(params![step_id], |row| row.get(0))?;
    
        // Deserialize the chat history from the JSON string
        let chat_history: Vec<StepChat> = serde_json::from_str(&chat_json).unwrap_or_else(|_| Vec::new());
    
        // Convert the StepChat into a formatted string with prompts and responses
        let formatted_string = chat_history
            .into_iter()
            .map(|chat| {
                format!("Prompt: {}\nResponse: {}\n", chat.prompt, chat.response)
            })
            .collect::<Vec<String>>()
            .join("\n"); // Join the formatted strings with newlines
    
        // Return the whole formatted string
        Ok(formatted_string)
    }

    pub fn update_step_heading(
        &self,
        pair_programmer_id: &str, step_number: &str,
        prompt: &str,
        response: &str
    ) -> Result<(), Box<dyn Error>> {
        
        // Lock the mutex to access the connection
        let connection = self.pair_programmer_connection.lock()
            .map_err(|_| "Failed to acquire lock for connection")?;
        let step_id = format!("{}_{}", pair_programmer_id, step_number);
    
        // Fetch the existing chat history for the step
        let mut stmt = connection.prepare("SELECT heading, chat FROM pp_steps WHERE id = ?")?;
        let (existing_heading, chat_json): (String, String) = stmt.query_row([step_id.clone()], |row| {
            Ok((row.get(0)?, row.get(1)?))
        }).map_err(|_| "Failed to retrieve heading and chat for the given step_id")?;
        
        // Deserialize the chat history into a Vec<StepChat>
        let mut chat_history: Vec<StepChat> = serde_json::from_str(&chat_json)
            .map_err(|_| "Failed to deserialize chat history")?;
        
        // Add the new chat entry
        let new_chat_entry = StepChat {
            prompt: prompt.to_string(),
            response: response.to_string(),
        };
        chat_history.push(new_chat_entry);
        
        // Serialize the updated chat history
        let updated_chat_json = serde_json::to_string(&chat_history)
            .map_err(|_| "Failed to serialize updated chat history")?;
        
        // Update the step record with the new task heading and chat
        connection.execute(
            "UPDATE pp_steps
                SET heading = ?, chat = ?
                WHERE id = ?",
            params![
                response,
                updated_chat_json,
                step_id,
            ],
        ).map_err(|e| format!("Failed to update pp_steps record: {}", e))?;
        
        info!("Updated step_id {} with new heading and chat entry", step_id);
        
        Ok(())
    }


    pub fn update_step_response(
        &self,
        pair_programmer_id: &str, step_number: &str,
        prompt: &str,
        response: &str
    ) -> Result<(), Box<dyn Error>> {
        
        // Lock the mutex to access the connection
        let connection = self.pair_programmer_connection.lock()
            .map_err(|_| "Failed to acquire lock for connection")?;
        let step_id = format!("{}_{}", pair_programmer_id, step_number);
    
        // Fetch the existing chat history for the step
        let mut stmt = connection.prepare("SELECT response, chat FROM pp_steps WHERE id = ?")?;
        let (existing_heading, chat_json): (String, String) = stmt.query_row([step_id.clone()], |row| {
            Ok((row.get(0)?, row.get(1)?))
        }).map_err(|_| "Failed to retrieve heading and chat for the given step_id")?;
        
        // Deserialize the chat history into a Vec<StepChat>
        let mut chat_history: Vec<StepChat> = serde_json::from_str(&chat_json)
            .map_err(|_| "Failed to deserialize chat history")?;
        
        // Add the new chat entry
        let new_chat_entry = StepChat {
            prompt: prompt.to_string(),
            response: response.to_string(),
        };
        chat_history.push(new_chat_entry);
        
        // Serialize the updated chat history
        let updated_chat_json = serde_json::to_string(&chat_history)
            .map_err(|_| "Failed to serialize updated chat history")?;
        
        // Update the step record with the new task heading and chat
        connection.execute(
            "UPDATE pp_steps
                SET response = ?, chat = ?
                WHERE id = ?",
            params![
                response,
                updated_chat_json,
                step_id,
            ],
        ).map_err(|e| format!("Failed to update pp_steps record: {}", e))?;
        
        info!("Updated step_id {} with new heading and chat entry", step_id);
        
        Ok(())
    }

    // pub fn get_step_chat(&self, pair_programmer_id: &str, step_number: &str) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    //     // Lock the mutex to access the connection
    //     let connection = self.pair_programmer_connection.lock().unwrap();
    //     let step_id = format!("{}_{}", pair_programmer_id, step_number);
    
    //     // Fetch the current chat from the step
    //     let mut stmt = connection.prepare("SELECT chat FROM pp_steps WHERE id = ?1")?;
    //     let chat_json: String = stmt.query_row(params![step_id], |row| row.get(0))?;
    
    //     // Deserialize the chat history from the JSON string
    //     let chat_history: Vec<StepChat> = serde_json::from_str(&chat_json).unwrap_or_else(|_| Vec::new());
    
    //     // Create a vector of JSON values where each element contains the prompt and response
    //     let chat_vector: Vec<Value> = chat_history
    //         .into_iter()
    //         .map(|chat| {
    //             json!({
    //                 "prompt": chat.prompt,
    //                 "response": chat.response
    //             })
    //         })
    //         .collect();
    
    //     // Return the vector of chat history
    //     Ok(chat_vector)
    // }

}