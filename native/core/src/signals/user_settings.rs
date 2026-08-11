use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GlobalSettingsResponse {
    #[serde(flatten)]
    pub settings: Value,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GetGlobalSettingsRequest {
    #[serde(default)]
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SetUserSettingRequest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SetUserSettingResponse {
    #[serde(default)]
    pub id: String,
    pub success: bool,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GetUserSettingRequest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetUserSettingResponse {
    #[serde(default)]
    pub id: String,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GetAllUserSettingsRequest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct GetAllUserSettingsResponse {
    #[serde(default)]
    pub id: String,
    pub settings: HashMap<String, String>,
}
