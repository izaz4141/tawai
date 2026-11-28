use std::env;
use std::sync::OnceLock;

static TAWAI_HOME: OnceLock<String> = OnceLock::new();

pub fn tawai_home() -> &'static String {
    TAWAI_HOME.get_or_init(|| {
        env::var("TAWAI_HOME")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "/home/tawai".to_string())
    })
}

/// Sets the process-wide Tawai home once. A repeated identical value is a
/// no-op; a different value is rejected so every path keeps one root.
pub fn set_tawai_home(path: &str) -> Result<(), String> {
    let env_home = env::var("TAWAI_HOME").ok();
    let home = resolve_home(path, env_home.as_deref())?;
    match TAWAI_HOME.set(home.clone()) {
        Ok(()) => Ok(()),
        Err(existing) if existing == home => Ok(()),
        Err(existing) => Err(format!(
            "tawai home is already set to `{existing}`, cannot change it to `{home}`"
        )),
    }
}

fn resolve_home(requested: &str, env_home: Option<&str>) -> Result<String, String> {
    if let Some(home) = env_home.filter(|value| !value.trim().is_empty()) {
        return Ok(home.to_string());
    }
    let requested = requested.trim();
    if requested.is_empty() {
        return Err("tawai home path is empty".to_string());
    }
    Ok(requested.to_string())
}

pub fn master_key_path() -> String {
    format!("{}/config/master.key", tawai_home())
}

pub fn config_path() -> String {
    format!("{}/config/config.toml", tawai_home())
}

pub fn data_dir() -> String {
    format!("{}/data", tawai_home())
}

pub fn downloads_dir() -> String {
    format!("{}/downloads", tawai_home())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_home_wins_over_the_argument() {
        assert_eq!(
            resolve_home("/requested", Some("/from-env")),
            Ok("/from-env".to_string())
        );
    }

    #[test]
    fn argument_is_used_when_the_environment_is_empty() {
        assert_eq!(
            resolve_home("/requested", Some("   ")),
            Ok("/requested".to_string())
        );
        assert_eq!(
            resolve_home("/requested", None),
            Ok("/requested".to_string())
        );
    }

    #[test]
    fn empty_home_is_rejected() {
        assert_eq!(
            resolve_home("   ", Some("")),
            Err("tawai home path is empty".to_string())
        );
    }
}
