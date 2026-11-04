//! Only mounted inside the existing admin-key authentication boundary.
use super::middleware::AdminState;
use crate::client_keys::{ClientKeyStore, KeyError};
use axum::{
    Json,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRequest {
    name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisabledRequest {
    disabled: bool,
}

fn store(state: &AdminState) -> Result<&ClientKeyStore, KeyError> {
    state.client_keys.as_deref().ok_or(KeyError::Persistence)
}
fn error(error: KeyError) -> Response {
    let (status, message) = match error {
        KeyError::InvalidName => (
            StatusCode::BAD_REQUEST,
            "密钥名称必须为 1–80 个字符，且不能包含控制字符",
        ),
        KeyError::NotFound => (StatusCode::NOT_FOUND, "客户端密钥不存在"),
        KeyError::LimitReached => (
            StatusCode::CONFLICT,
            "最多保存 1000 把客户端密钥，请先删除不再使用的密钥",
        ),
        KeyError::Persistence => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "保存客户端密钥失败，请检查配置目录的写入权限后重试",
        ),
    };
    (
        status,
        Json(json!({"error":{"type":"client_key_error","message":message}})),
    )
        .into_response()
}
fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    response
}
pub async fn list(State(state): State<AdminState>) -> Response {
    no_store(match store(&state) {
        Ok(store) => {
            Json(json!({"keys":store.list(),"legacyKeyEnabled":state.legacy_client_key_enabled}))
                .into_response()
        }
        Err(e) => error(e),
    })
}
pub async fn create(State(state): State<AdminState>, Json(body): Json<CreateRequest>) -> Response {
    no_store(
        match store(&state).and_then(|store| store.create(&body.name)) {
            Ok(created) => (StatusCode::CREATED, Json(created)).into_response(),
            Err(e) => error(e),
        },
    )
}
pub async fn set_disabled(
    State(state): State<AdminState>,
    Path(id): Path<Uuid>,
    Json(body): Json<DisabledRequest>,
) -> Response {
    no_store(
        match store(&state).and_then(|store| store.set_disabled(id, body.disabled)) {
            Ok(item) => Json(item).into_response(),
            Err(e) => error(e),
        },
    )
}
pub async fn delete(State(state): State<AdminState>, Path(id): Path<Uuid>) -> Response {
    no_store(match store(&state).and_then(|store| store.delete(id)) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => error(e),
    })
}
