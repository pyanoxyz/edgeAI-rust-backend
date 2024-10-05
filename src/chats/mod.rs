pub mod chat_infill;
pub mod chat_plain;
pub mod chat_explain;
pub mod chat_refactor;
pub mod chat_testcases;
pub mod chat_findbugs;
pub mod chat_docstring;
// pub use chat_infill::register_routes as chat_fill_routes;
pub use chat_plain::register_routes as chat_plain_routes;
// pub use chat_explain::register_routes as chat_explain_routes;
// pub use chat_refactor::register_routes as chat_refactor_routes;
// pub use chat_testcases::register_routes as chat_testcases_routes;
// pub use chat_findbugs::register_routes as chat_findbugs_routes;
// pub use chat_docstring::register_routes as chat_docstring_routes;
