pub mod chat_infill;
pub mod chat_plain;
pub mod chat_explain;
pub use chat_infill::register_routes as chat_fill_routes;
pub use chat_plain::register_routes as chat_plain_routes;
pub use chat_explain::register_routes as chat_explain_routes;
