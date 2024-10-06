

use crate::database::db_config::DBConfig;
use rusqlite::params;
use crate::model_state::state::ConfigSection;


impl DBConfig{
    pub fn update_model_config(&self, config: &ConfigSection) {
        let connection = self.common_connection.lock().unwrap();
        connection.execute(
            "
            INSERT INTO config (id, model_name, model_url, model_size, ctx_size, gpu_layers_offloading, batch_size, mlock, nmap, system_prompt)
            VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(id) DO UPDATE SET
                model_name = excluded.model_name,
                model_url = excluded.model_url,
                model_size = excluded.model_size,
                ctx_size = excluded.ctx_size,
                gpu_layers_offloading = excluded.gpu_layers_offloading,
                batch_size = excluded.batch_size,
                mlock = excluded.mlock,
                nmap = excluded.nmap,
                system_prompt = excluded.system_prompt;
            ",
            params![
                config.model_name,
                config.model_url,
                config.model_size,
                config.ctx_size,
                config.gpu_layers_offloading,
                config.batch_size,
                config.mlock as i32,
                config.mmap as i32,
                config.system_prompt,
            ],
        ).unwrap();
    }
    
     
    pub fn get_model_config(&self) ->  Result<ConfigSection, rusqlite::Error> {
        let connection = self.common_connection.lock().unwrap();
        let mut stmt = connection.prepare(
            "
            SELECT
                model_name,
                model_url,
                model_size,
                ctx_size,
                gpu_layers_offloading,
                batch_size,
                mlock,
                nmap,
                system_prompt
            FROM config WHERE id = 1;
            ",
        )?;

        let config = stmt.query_row([], |row| {
            Ok(ConfigSection {
                model_name: row.get(0)?,
                model_url: row.get(1)?,
                model_size: row.get(2)?,
                ctx_size: row.get(3)?,
                gpu_layers_offloading: row.get(4)?,
                batch_size: row.get(5)?,
                mlock: row.get::<_, i32>(6)? != 0,
                mmap: row.get::<_, i32>(7)? != 0,
                system_prompt: row.get(8)?,
            })
        });

        config
    }
    
    pub fn get_system_prompt(&self) -> Result<String, rusqlite::Error> {
        // Lock the common database connection to ensure safe access across threads
        let connection = self.common_connection.lock().unwrap();
    
        // Prepare the SQL statement to select the system_prompt from the config table
        let mut stmt = connection.prepare(
            "
            SELECT
                system_prompt
            FROM config WHERE id = 1;
            ",
        )?;
    
        // Query the database and return the system_prompt as a String
        let system_prompt = stmt.query_row([], |row| {
            // Extract and return the system_prompt value from the row
            row.get(0)
        });
    
        // Return the system_prompt or propagate any error encountered
        system_prompt
    }

}