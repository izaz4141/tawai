use std::env;
use std::sync::OnceLock;

static TAWAI_HOME: OnceLock<String> = OnceLock::new();

pub fn tawai_home() -> &'static String {
    TAWAI_HOME.get_or_init(|| env::var("TAWAI_HOME").unwrap_or_else(|_| "/home/tawai".to_string()))
}

pub fn config_path() -> String {
    format!("{}/config/config.json", tawai_home())
}

pub fn data_dir() -> String {
    format!("{}/data", tawai_home())
}

pub fn downloads_dir() -> String {
    format!("{}/downloads", tawai_home())
}
