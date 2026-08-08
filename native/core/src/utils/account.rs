use anyhow::Result;

use crate::db::{account, database::DatabaseManager};
use crate::signals::account::{
    CreateAccountRequest, CreateAccountResponse, DeleteAccountRequest, DeleteAccountResponse,
    GetUserByIdResponse, GetUserByUsernameResponse, UpdateAccountRequest, UpdateAccountResponse,
    UserListItem,
};
use crate::utils::{encryption, helper, security};

#[derive(Debug)]
pub enum AccountError {
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    BadRequest,
    Internal(anyhow::Error),
}

impl std::fmt::Display for AccountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unauthorized => write!(f, "Invalid authorizer for account operation"),
            Self::Forbidden => write!(f, "Not authorized to edit this account"),
            Self::NotFound => write!(f, "Target user not found"),
            Self::Conflict => write!(f, "Username already taken"),
            Self::BadRequest => write!(f, "Invalid account operation"),
            Self::Internal(e) => write!(f, "Internal error: {e}"),
        }
    }
}

impl std::error::Error for AccountError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Internal(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

pub fn sanitize_role(role: Option<&str>) -> &'static str {
    match role {
        Some("admin") => "admin",
        _ => "user",
    }
}

pub fn valid_role(role: &str) -> bool {
    ["admin", "user"].contains(&role)
}

pub async fn get_user_by_id(
    db: &DatabaseManager,
    mk: &str,
    user_id: &str,
) -> Result<GetUserByIdResponse> {
    let user = account::get_user_by_id(db.pool(), user_id, mk).await?;
    Ok(GetUserByIdResponse {
        id: String::new(),
        user_id: user.id,
        username: user.username,
        display_name: user.display_name,
        role: user.role,
        found: true,
    })
}

pub async fn get_user_by_username(
    db: &DatabaseManager,
    mk: &str,
    username: &str,
) -> Result<GetUserByUsernameResponse> {
    let user = account::get_user_by_username(db.pool(), username, mk).await?;
    Ok(GetUserByUsernameResponse {
        id: String::new(),
        user_id: user.id,
        username: user.username,
        display_name: user.display_name,
        role: user.role,
        found: true,
    })
}

pub async fn list_users(
    db: &DatabaseManager,
    mk: &str,
    include_api_keys: bool,
) -> Result<Vec<UserListItem>> {
    if include_api_keys {
        account::get_all_users_with_keys(db.pool(), mk).await
    } else {
        account::get_all_users(db.pool()).await
    }
}

pub async fn verify_current_password(
    db: &DatabaseManager,
    mk: &str,
    user_id: &str,
    password: &str,
) -> bool {
    let user = account::get_user_by_id(db.pool(), user_id, mk).await;
    match user {
        Ok(u) => security::validate_password(&u.password_hash, password).unwrap_or(false),
        Err(_) => false,
    }
}

async fn is_admin(db: &DatabaseManager, mk: &str, username: &str, password: &str) -> Result<bool> {
    let user = account::get_user_by_username(db.pool(), username, mk).await;
    Ok(match user {
        Ok(u) => {
            u.role == "admin"
                && security::validate_password(&u.password_hash, password).unwrap_or(false)
        }
        Err(_) => false,
    })
}

pub async fn create_user(
    db: &DatabaseManager,
    mk: &str,
    req: &CreateAccountRequest,
) -> Result<CreateAccountResponse> {
    let username = req.username.trim();
    if username.is_empty() {
        return Err(anyhow::Error::new(AccountError::BadRequest));
    }

    if !is_admin(db, mk, &req.admin_username, &req.admin_password).await? {
        return Err(anyhow::Error::new(AccountError::Unauthorized));
    }

    if account::get_user_by_username(db.pool(), &req.username, mk)
        .await
        .is_ok()
    {
        return Err(anyhow::Error::new(AccountError::Conflict));
    }

    let api_key = uuid::Uuid::new_v4().to_string();
    let encrypted = encryption::encrypt(&api_key, mk).unwrap_or_else(|_| api_key.clone());
    let api_key_hash = helper::sha256_hex(&api_key);
    let display_name = req.display_name.clone().unwrap_or_default();
    let role = sanitize_role(req.role.as_deref());

    let user_id = account::create_user(
        db.pool(),
        &req.username,
        &display_name,
        &req.password,
        &encrypted,
        &api_key_hash,
        role,
    )
    .await
    .map_err(|e| anyhow::Error::new(AccountError::Internal(e)))?;

    Ok(CreateAccountResponse {
        id: req.id.clone(),
        success: true,
        user_id,
        username: req.username.clone(),
        display_name,
        role: role.to_string(),
        api_key,
    })
}

pub async fn delete_user(
    db: &DatabaseManager,
    mk: &str,
    req: &DeleteAccountRequest,
) -> Result<DeleteAccountResponse> {
    if !is_admin(db, mk, &req.admin_username, &req.admin_password).await? {
        return Err(anyhow::Error::new(AccountError::Unauthorized));
    }

    let target = account::get_user_by_username(db.pool(), &req.target_username, mk)
        .await
        .map_err(|_| anyhow::Error::new(AccountError::NotFound))?;

    if target.role == "admin" {
        let users = account::get_all_users(db.pool()).await.unwrap_or_default();
        let admin_count = users.iter().filter(|u| u.role == "admin").count();
        if admin_count <= 1 {
            return Err(anyhow::Error::new(AccountError::BadRequest));
        }
    }

    let deleted = account::delete_user(db.pool(), &target.id)
        .await
        .map_err(|e| anyhow::Error::new(AccountError::Internal(e)))?;

    if !deleted {
        return Err(anyhow::Error::new(AccountError::NotFound));
    }

    Ok(DeleteAccountResponse {
        id: req.id.clone(),
        success: true,
        username: req.target_username.clone(),
    })
}

pub async fn update_user(
    db: &DatabaseManager,
    mk: &str,
    req: &UpdateAccountRequest,
) -> Result<UpdateAccountResponse> {
    let authorizer = account::get_user_by_id(db.pool(), &req.operator_user_id, mk)
        .await
        .map_err(|_| anyhow::Error::new(AccountError::Unauthorized))?;

    if !security::validate_password(&authorizer.password_hash, &req.operator_password)
        .unwrap_or(false)
    {
        return Err(anyhow::Error::new(AccountError::Unauthorized));
    }

    if authorizer.role != "admin" && req.operator_user_id != req.target_user_id {
        return Err(anyhow::Error::new(AccountError::Forbidden));
    }

    let target = account::get_user_by_id(db.pool(), &req.target_user_id, mk)
        .await
        .map_err(|_| anyhow::Error::new(AccountError::NotFound))?;

    if let Some(new_username) = &req.new_username {
        if !new_username.is_empty() {
            account::change_username(db.pool(), &target.id, new_username)
                .await
                .map_err(|e| anyhow::Error::new(AccountError::Internal(e)))?;
        }
    }

    if let Some(new_password) = &req.new_password {
        if !new_password.is_empty() {
            account::change_password(db.pool(), &target.id, new_password)
                .await
                .map_err(|e| anyhow::Error::new(AccountError::Internal(e)))?;
        }
    }

    if let Some(display_name) = &req.display_name {
        if !display_name.is_empty() {
            account::change_display_name(db.pool(), &target.id, display_name)
                .await
                .map_err(|e| anyhow::Error::new(AccountError::Internal(e)))?;
        }
    }

    if let Some(role) = &req.role {
        if !role.is_empty() && valid_role(role) {
            if role == "admin" && authorizer.role != "admin" {
                return Err(anyhow::Error::new(AccountError::Forbidden));
            }
            account::change_role(db.pool(), &target.id, role)
                .await
                .map_err(|e| anyhow::Error::new(AccountError::Internal(e)))?;
        }
    }

    let updated = account::get_user_by_id(db.pool(), &req.target_user_id, mk)
        .await
        .map_err(|e| anyhow::Error::new(AccountError::Internal(e)))?;

    Ok(UpdateAccountResponse {
        id: req.id.clone(),
        success: true,
        user_id: updated.id,
        username: updated.username,
        display_name: updated.display_name,
        role: updated.role,
        api_key: updated.api_key,
    })
}
