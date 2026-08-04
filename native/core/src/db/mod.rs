pub mod account;
mod account_pg;
mod account_sq;
pub mod database;
pub mod download;
mod download_pg;
mod download_sq;
pub mod history;
mod history_pg;
mod history_sq;
pub mod library;
mod library_pg;
pub mod library_source;
mod library_source_pg;
mod library_source_sq;
mod library_sq;
pub mod user_settings;
mod user_settings_pg;
mod user_settings_sq;

/// PostgreSQL expression rendering a `TIMESTAMPTZ` column as an RFC3339 UTC
/// string, matching the TEXT format stored by SQLite.
pub(crate) fn ts_utc(col: &str) -> String {
    let alias = col.rsplit('.').next().unwrap_or(col);
    format!("to_char({col} AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.MS\"Z\"') AS {alias}")
}
