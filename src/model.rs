use mermaid_svg::RenderError;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use axum::{response::{IntoResponse, Response}, http::StatusCode};
use schemars::JsonSchema;
#[derive(Serialize, Debug, Clone)]
pub struct Paste {
    pub id: Uuid,
    pub content: Vec<u8>,
    pub mimetype: MimeKind,
    pub hits: u32,
    pub last_seen_tick: u64,
}

impl Paste {
    pub fn new (content: Vec<u8>, mimetype: MimeKind, last_seen_tick: u64, max_content_size: usize) -> Result<Self, AppError> {
        if content.len() > max_content_size {
            return Err(AppError::PayloadTooLarge);
        }

        if content.is_empty() {
            return Err(AppError::BadRequest(String::from("Content cannot be empty")));
        }

        Ok(Self {
            id: Uuid::new_v4(),
            content,
            mimetype,
            hits: 0,
            last_seen_tick
        })
    }

    pub fn increment_hits ( &mut self ) {
        self.hits += 1;
    }

    pub fn decrement_hits ( &mut self) {
        if self.hits > 0 {
            self.hits -= 1;
        }
    }

    pub fn update ( &mut self, tick: u64) {
        self.last_seen_tick = tick;
        self.increment_hits();
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub enum MimeKind {
    PlainText,
    Html,
    Markdown,
    OctetStream,
}

#[derive(Debug)]
pub enum AppError {
    NotFound,
    PayloadTooLarge,
    BadRequest (String),
    MarkdownParserFailed,
    MermaidRenderError(RenderError),
    FullCapacityEvictionFailed,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::NotFound => {
                (StatusCode::NOT_FOUND, "Paste not found").into_response()
            }
            AppError::PayloadTooLarge => {
                (StatusCode::PAYLOAD_TOO_LARGE, "Payload too large").into_response()
            }
            AppError::BadRequest(e) => {
                (StatusCode::BAD_REQUEST, format!("{}", e)).into_response()
            }
            AppError::MarkdownParserFailed => {
                (StatusCode::BAD_REQUEST, "Markdown parser failed").into_response()
            }
            AppError::MermaidRenderError(e) => {
                (StatusCode::BAD_REQUEST, format!("Mermaid render failed: {}", e)).into_response()
            }
            AppError::FullCapacityEvictionFailed => {
                (StatusCode::BAD_REQUEST, "Full capacity eviction failed").into_response()
            }
        }
    }
}