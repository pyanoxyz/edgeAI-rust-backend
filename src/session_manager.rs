use std::sync::{ Arc, Mutex };
use uuid::Uuid;
use once_cell::sync::Lazy;

// Define a Session structure with a unique ID
#[derive(Clone, Debug)]
struct Session {
    id: String,
}

impl Session {
    fn new() -> Self {
        Session {
            id: Uuid::new_v4().to_string(),
        }
    }
}

// Define the SessionManager structure (Singleton)
struct SessionManager {
    session: Arc<Mutex<Option<Session>>>,
}

// Singleton instance using Lazy static initialization
static INSTANCE: Lazy<Arc<SessionManager>> = Lazy::new(|| {
    Arc::new(SessionManager {
        session: Arc::new(Mutex::new(None)),
    })
});

impl SessionManager {
    // Get the Singleton instance of SessionManager
    fn get_instance() -> Arc<SessionManager> {
        Arc::clone(&INSTANCE)
    }

    // Get the current session, create a new one if none exists    // Create a new session explicitly
    fn create_new_session(&self) -> Session {
        let new_session = Session::new();
        let mut session_guard = self.session.lock().unwrap();
        *session_guard = Some(new_session.clone());
        new_session
    }


}
// Function to check session ID

pub fn check_session(
    session_id: Option<String>
) -> Result<String, actix_web::Error> {
    let session_manager = SessionManager::get_instance();

    match session_id {
        Some(id) if id.is_empty() => {
            // Check for empty string
            let new_session = session_manager.create_new_session();
            Ok(new_session.id)
        }
        Some(id) => Ok(id),
        None => {
            // Attempt to create a new session, return an error if it fails
            let new_session = session_manager.create_new_session();
            Ok(new_session.id)
        }
    }
}
