use std::fs;
use log::{ info, error };
use lazy_static::lazy_static;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use sqlite_vec::sqlite3_vec_init;
use rusqlite::{ ffi::sqlite3_auto_extension, Connection };
use rusqlite_migration::{ Migrations, M };

pub struct DBConfig {
    pub connection: Mutex<Connection>, // Wrapping the connection in Mutex for thread-safe access
    pub pair_programmer_connection: Mutex<Connection>,
    pub common_connection: Mutex<Connection>,
}

lazy_static! {
    static ref MIGRATIONS: Migrations<'static> = Migrations::new(
        vec![M::up("ALTER TABLE chats ADD COLUMN vec_row_id TEXT NOT NULL;")]
    );
    // TODO can add MIGRATIONS_PAIR_PROGRAMMING etc for respective dbs and move migrations to separate folder
}

impl DBConfig {
    // Function to create a new database connection (or open existing one)
    pub fn new() -> Self {
        let home_directory = dirs::home_dir().unwrap();
        let root_pyano_dir = home_directory.join(".pyano");
        let pyano_data_dir = root_pyano_dir.join("database");
        if !pyano_data_dir.exists() {
            fs::create_dir_all(&pyano_data_dir).unwrap();
        }
        let pyano_db_file = pyano_data_dir.join("chats.db");
        let pair_programmer_db_file = pyano_data_dir.join("pair_programmer.db");
        let common_db_file = pyano_data_dir.join("common.db");

        // Register the sqlite-vec extension to support vector operations
        unsafe {
            sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
        }
        let connection: Connection = Connection::open(&pyano_db_file).unwrap();
        let pair_programmer_connection: Connection = Connection::open(
            &pair_programmer_db_file
        ).unwrap();
        let common_connection = Connection::open(&common_db_file).unwrap();

        let db_config = DBConfig {
            connection: Mutex::new(connection), // Wrapping the connection
            pair_programmer_connection: Mutex::new(pair_programmer_connection),
            common_connection: Mutex::new(common_connection),
        };

        db_config.create_chat_table();
        db_config.create_chat_embeddings();
        db_config.create_parent_context_table();
        db_config.create_children_context_table();
        db_config.create_context_embeddings();
        db_config.create_pair_programmer_steps_table();
        db_config.create_pair_programmer_table();
        db_config.create_config_table();

        if let Err(e) = db_config.run_migrations() {
            error!("Failed to run migrations: {:?}", e);
            // Handle the error appropriately, maybe exit the program or take alternative action
        } else {
            info!("Migrations completed successfully");
        }

        db_config
    }

    pub fn run_migrations(&self) -> Result<(), rusqlite_migration::Error> {
        let mut connection = self.connection.lock().unwrap();
        info!("Migration for connection");
        match MIGRATIONS.to_latest(&mut connection) {
            Ok(_) => {
                // info!("Migrations completed successfully");
                Ok(())
            }
            Err(e) => {
                // error!("Migration failed: {:?}", e);
                Err(e)
            }
        }
    }

    pub fn create_config_table(&self) {
        let connection = self.common_connection.lock().unwrap();
        info!("Checking for <config> Table in common connection");
        connection
            .execute(
                "
            CREATE TABLE IF NOT EXISTS config (
                id TEXT PRIMARY KEY,  -- UUID as primary key
                model_name TEXT NOT NULL,
                model_url TEXT NOT NULL,
                model_size INTEGER,
                ctx_size INTEGER,
                gpu_layers_offloading INTEGER,
                batch_size INTEGER,
                mlock INTEGER,
                nmap INTEGER,
                system_prompt TEXT
            );
            ",
                [] // Empty array for parameters since none are needed
            )
            .unwrap();
    }

    pub fn create_chat_table(&self) {
        let connection = self.connection.lock().unwrap();
        info!("Checking for <chats> Table");
        connection
            .execute(
                "
            CREATE TABLE IF NOT EXISTS chats (
                id TEXT PRIMARY KEY,  -- UUID as primary key
                user_id TEXT NOT NULL,
                session_id TEXT NOT NULL,
                prompt TEXT,
                compressed_prompt_response TEXT,
                response TEXT,
                timestamp TEXT,
                request_type TEXT
            );
            ",
                [] // Empty array for parameters since none are needed
            )
            .unwrap();
    }

    pub fn create_pair_programmer_table(&self) {
        info!("Checking for <pair_programmer> Table");

        let connection = self.pair_programmer_connection.lock().unwrap();
        connection
            .execute(
                "
            CREATE TABLE IF NOT EXISTS pair_programmer (
                id TEXT PRIMARY KEY,  -- UUID as primary key,
                user_id TEXT NOT NULL,
                session_id TEXT NOT NULL,
                task TEXT NOT NULL,
                steps TEXT,
                timestamp TEXT
            );
            ",
                [] // Empty array for parameters since none are needed
            )
            .unwrap();
    }

    pub fn create_pair_programmer_steps_table(&self) {
        info!("Checking for <pair_programmer_steps> Table");

        let connection = self.pair_programmer_connection.lock().unwrap();
        connection
            .execute(
                "
            CREATE TABLE IF NOT EXISTS pair_programmer_steps (
                id TEXT PRIMARY KEY,  -- UUID as primary key,
                pair_programmer_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                session_id TEXT NOT NULL,
                heading TEXT NOT NULL,
                function_call TEXT NOT NULL,
                executed INTEGER NOT NULL,
                response TEXT,
                timestamp TEXT,
                chat TEXT
            );
            ",
                [] // Empty array for parameters since none are needed
            )
            .unwrap();
    }

    //Saves the individual chunks in the table
    pub fn create_parent_context_table(&self) {
        info!("Checking for <context_parent> Table");

        let connection = self.connection.lock().unwrap();
        connection
            .execute(
                "
            CREATE TABLE IF NOT EXISTS context_parent (
                id TEXT PRIMARY KEY,  -- UUID as primary key
                session_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                parent_path TEXT NOT NULL,
                filetype TEXT, 
                category TEXT,
                timestamp TEXT
            );
            ",
                [] // Empty array for parameters since none are needed
            )
            .unwrap();
    }

    //Saves the individual chunks in the table
    pub fn create_children_context_table(&self) {
        info!("Checking for <context_children> Table");

        let connection = self.connection.lock().unwrap();
        connection
            .execute(
                "
            CREATE TABLE IF NOT EXISTS context_children (
                id TEXT PRIMARY KEY,  -- UUID as primary key
                user_id TEXT NOT NULL,
                session_id TEXT NOT NULL,
                parent_path TEXT,
                chunk_type TEXT,
                content TEXT,
                compressed_content TEXT,
                end_line INTEGER,
                file_path TEXT,
                start_line INTEGER,
                vec_row_id TEXT NOT NULL,  -- This links to the rowid in the vec table
                timestamp TEXT            
                );
            ",
                [] // Empty array for parameters since none are needed
            )
            .unwrap();
    }

    pub fn create_context_embeddings(&self) {
        info!("Checking for <context_embeddings> Table");

        let connection = self.connection.lock().unwrap();
        let table_exists: bool =
            connection
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='context_embeddings';",
                    [],
                    |row| row.get(0)
                )
                .unwrap_or(0) > 0;

        // Create the 'chat_embeddings' table only if it doesn't exist
        if !table_exists {
            connection
                .execute(
                    "
            CREATE VIRTUAL TABLE context_embeddings USING vec0 (embeddings float[384]);
            ",
                    []
                )
                .unwrap();
        }
    }

    pub fn create_chat_embeddings(&self) {
        info!("Checking for <chat_embeddings> Table");

        let connection = self.connection.lock().unwrap();
        let table_exists: bool =
            connection
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='chat_embeddings';",
                    [],
                    |row| row.get(0)
                )
                .unwrap_or(0) > 0;

        // Create the 'chat_embeddings' table only if it doesn't exist
        if !table_exists {
            connection
                .execute(
                    "
            CREATE VIRTUAL TABLE chat_embeddings USING vec0 (embeddings float[384]);
            ",
                    []
                )
                .unwrap();
        }
    }
}
// Create a singleton instance of the database connection

pub static DB_INSTANCE: Lazy<DBConfig> = Lazy::new(|| { DBConfig::new() });
