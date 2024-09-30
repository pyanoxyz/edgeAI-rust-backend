

use rand::Rng;
use chrono::Utc; // For getting the current UTC timestamp
use rusqlite::params;
use serde_json::{json, Value};
use crate::database::db_config::DBConfig;
use crate::pair_programmer::pair_programmer_types::{Step, StepChat};

impl DBConfig{


    pub fn generate_pair_programmer_id() -> u64 {
        let mut rng = rand::thread_rng();
        rng.gen_range(1_000_000_000_000_000..=9_999_999_999_999_999)
    }

    // Function to store a new chat record with embeddings, timestamp, and compressed prompt
    pub fn store_new_pair_programming_session(&self, user_id: &str, session_id: &str, pair_programmer_id: &str, task: &str, steps: &Vec<Step>) {
            
        // Lock the mutex to access the connection
        let connection = self.pair_programmer_connection.lock().unwrap();
        let serialized_steps = serde_json::to_string(&steps).unwrap();
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
        ).unwrap();

        for (index, step) in steps.iter().enumerate() {
            let step_id = format!("{}_{}", pair_programmer_id, index+1);
            let serialized_chat = serde_json::to_string(&Vec::<StepChat>::new()).unwrap();

            connection.execute(
                "INSERT INTO pair_programmer_steps (id, pair_programmer_id, user_id, session_id, heading, function_call, executed, response, timestamp, chat)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    step_id, 
                    pair_programmer_id,
                    user_id,
                    session_id,
                    step.heading,
                    step.tool,
                    0,
                    "",
                    timestamp.as_str(),
                    serialized_chat,

                ],
            ).unwrap();

        }

    }
    

    pub fn fetch_steps(&self, pair_programming_id: &str) -> Vec<Value> {
        // Lock the mutex to access the connection
        let connection = self.connection.lock().unwrap();
        
        // Prepare a SQL query to fetch all the steps for a specific pair_programming_id
        let mut stmt = connection
            .prepare(
                "SELECT step_id, user_id, session_id, heading, function_call, executed, response, chat, timestamp 
                 FROM pair_programmer_steps 
                 WHERE pair_programming_id = ?",
            )
            .unwrap();
        
        // Create a vector to hold the steps in JSON format
        let mut steps: Vec<Value> = Vec::new();
        
        // Execute the query and iterate over the rows, collecting them into the vector
        let step_iter = stmt
            .query_map([pair_programming_id], |row| {
                Ok(json!({
                    "step_id": row.get::<_, String>(0)?,         // step_id
                    "user_id": row.get::<_, String>(1)?,         // user_id
                    "session_id": row.get::<_, String>(2)?,      // session_id
                    "heading": row.get::<_, String>(3)?,         // heading
                    "tool": row.get::<_, String>(4)?,   // function_call
                    "executed": row.get::<_, bool>(5)?,          // executed (boolean)
                    "response": row.get::<_, String>(6)?,        // response
                    "chat": row.get::<_, String>(7)?,            // chat (assuming it's serialized as JSON or a string)
                    "timestamp": row.get::<_, String>(8)?,       // timestamp
                }))
            })
            .unwrap();
        
        // Collect all rows into the `steps` vector
        for step in step_iter {
            steps.push(step.unwrap());
        }
        
        steps
    }

}