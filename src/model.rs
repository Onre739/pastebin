use uuid::Uuid;
use serde::Serialize;
use serde::Deserialize;
use axum::response::{IntoResponse, Response};

#[derive(Deserialize)]
pub struct CreatePasteDto {
    pub content: String,
    pub mimetype: MimeKind,
}

#[derive(Serialize, Debug, Clone)]
pub struct Paste {
    pub id: Uuid,
    pub content: Vec<u8>,          // binární i textový obsah
    pub mimetype: MimeKind,        // enum: PlainText, Html, Markdown, OctetStream
    pub hits: u32,                 // počítadlo pro LRU
    pub last_seen_tick: u64,       // "generace" naposledy zobrazeno, pro nalezení kandidáta k evikci
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum MimeKind {
    PlainText,
    Html,
    Markdown,
    OctetStream,
}

pub enum AppError {
    NotFound,
    InvalidMimeType,
    PayloadTooLarge,
    LockPoisoned,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::NotFound => {
                (axum::http::StatusCode::NOT_FOUND, "Paste not found").into_response()
            }
            AppError::InvalidMimeType => {
                (axum::http::StatusCode::BAD_REQUEST, "Invalid mimetype").into_response()
            }
            AppError::PayloadTooLarge => {
                (axum::http::StatusCode::PAYLOAD_TOO_LARGE, "Payload too large").into_response()
            }
            AppError::LockPoisoned => {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
        }
    }
}