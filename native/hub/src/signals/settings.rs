use rinf::{DartSignal, RustSignal};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, RustSignal)]
pub struct GlobalSettingsResponse {
    pub id: String,
    pub settings_json: Option<String>,
}

#[derive(Deserialize, DartSignal)]
pub struct GetGlobalSettingsRequest {
    pub id: String,
}

#[derive(Deserialize, DartSignal)]
pub struct SetUserSettingRequest {
    pub id: String,
    pub user_id: String,
    pub key: String,
    pub value: String,
}

#[derive(Serialize, RustSignal)]
pub struct SetUserSettingResponse {
    pub id: String,
    pub success: bool,
}

#[derive(Deserialize, DartSignal)]
pub struct GetUserSettingRequest {
    pub id: String,
    pub user_id: String,
    pub key: String,
}

#[derive(Serialize, RustSignal)]
pub struct GetUserSettingResponse {
    pub id: String,
    pub key: String,
    pub value: String,
}

#[derive(Deserialize, DartSignal)]
pub struct GetAllUserSettingsRequest {
    pub id: String,
    pub user_id: String,
}

#[derive(Serialize, RustSignal)]
pub struct GetAllUserSettingsResponse {
    pub id: String,
    pub settings: HashMap<String, String>,
}
