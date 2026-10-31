pub mod account;
pub mod cache;
pub mod config;
pub mod encryption;
pub mod fs;
pub mod helper;
pub mod identify;
pub mod logger;
pub mod playback;
pub mod romajize;
pub mod security;
pub mod settings;
pub mod tagging;
pub mod types;
pub mod url;
pub mod version;

/// User-Agent sent with outbound requests to public APIs.
pub const MB_UA: &str = "Tawai/0.0.1 ( https://github.com/izaz4141/tawai )";
