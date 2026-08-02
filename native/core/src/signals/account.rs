use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserListItem {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub role: String,
}
