use rinf::{DartSignal, RustSignal};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, DartSignal)]
pub struct SetTawaiHome {
    pub id: String,
    pub path: String,
    pub device_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct SetTawaiHomeResponse {
    pub id: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Deserialize, DartSignal)]
pub struct StartServer {
    pub port: u16,
}

#[derive(Serialize, RustSignal)]
pub struct LogSignal {
    pub level: String,
    pub message: String,
}

#[derive(Deserialize, DartSignal)]
pub struct InitDatabase {
    pub id: String,
}

#[derive(Deserialize, DartSignal)]
pub struct InitConfig {
    pub id: String,
}

#[derive(Serialize, RustSignal)]
pub struct InitConfigResponse {
    pub id: String,
    pub success: bool,
    pub error: Option<String>,
    pub settings_json: Option<String>,
    pub is_first_run: bool,
}

#[derive(Deserialize, DartSignal)]
pub struct SaveConfigRequest {
    pub id: String,
    pub settings_json: String,
}

#[derive(Serialize, RustSignal)]
pub struct SaveConfigResponse {
    pub id: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Serialize, RustSignal)]
pub struct InitDatabaseResponse {
    pub id: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Deserialize, DartSignal)]
pub struct CreateTaggingRequest {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Serialize, RustSignal)]
pub struct CreateTaggingResponse {
    pub id: String,
    pub success: String,
}

#[derive(Deserialize, DartSignal)]
pub struct PutTaggingRequest {
    pub id: String,
    pub name: String,
    pub tag: String,
}

#[derive(Serialize, RustSignal)]
pub struct PutTaggingResponse {
    pub id: String,
    pub success: String,
}
