pub mod json;
pub mod models;
pub mod traits;

// Re-export the storage trait and main implementation
pub use json::JsonStorage;
pub use models::*;
pub use traits::Storage;
