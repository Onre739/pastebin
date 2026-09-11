use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use rmcp::transport::streamable_http_server::{
    StreamableHttpService,
    session::local::LocalSessionManager,
};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use crate::model::MimeKind;
use crate::store::AppState;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreatePasteArgs {
    #[schemars(description = "paste content as plain text")]
    pub content: String,
    #[schemars(description = "content type: PlainText, Html, or Markdown. For OctetStream (binary) content use the create_binary_paste tool instead.")]
    pub mimetype: MimeKind,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateBinaryPasteArgs {
    #[schemars(description = "paste content encoded as base64 - use this for arbitrary binary data (images, archives, etc.) that isn't valid UTF-8 text")]
    pub content_base64: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetPasteArgs {
    #[schemars(description = "UUID of the paste to display")]
    pub id: String,
}

#[derive(Clone)]
pub struct PastebinMcp {
    state: AppState,
}

impl PastebinMcp {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[tool_router]
impl PastebinMcp {
    #[tool(description = "Creates a new text paste (PlainText, Html or Markdown) and returns its UUID")]
    fn create_paste(&self, Parameters(args): Parameters<CreatePasteArgs>) -> Result<String, String> {
        let mut paste_store = self.state.paste_store.lock().unwrap();
        paste_store
            .insert(args.content.into_bytes(), args.mimetype)
            .map(|id| id.to_string())
            .map_err(|e| format!("{:?}", e))
    }

    #[tool(description = "Creates a new OctetStream (binary) paste from base64-encoded content and returns its UUID")]
    fn create_binary_paste(&self, Parameters(args): Parameters<CreateBinaryPasteArgs>) -> Result<String, String> {
        let content = BASE64.decode(&args.content_base64)
            .map_err(|e| format!("Invalid base64 content: {e}"))?;

        let mut paste_store = self.state.paste_store.lock().unwrap();
        paste_store
            .insert(content, MimeKind::OctetStream)
            .map(|id| id.to_string())
            .map_err(|e| format!("{:?}", e))
    }

    #[tool(description = "Displays a paste by UUID - same as a user opening it in a browser (counts as a view, incrementing hits and last_seen_tick)")]
    fn get_paste(&self, Parameters(args): Parameters<GetPasteArgs>) -> Result<String, String> {
        let uuid = Uuid::parse_str(&args.id).map_err(|e| format!("Invalid UUID: {e}"))?;

        let mut paste_store = self.state.paste_store.lock().unwrap();
        let paste = paste_store
            .record_view(uuid)
            .map_err(|e| format!("{:?}", e))?;

        let content = match paste.mimetype {
            MimeKind::OctetStream => {
                format!("binary content, {} bytes (cannot be displayed as text; use an HTTP client against GET /paste/{{uuid}} to download it)", paste.content.len())
            }
            _ => String::from_utf8_lossy(&paste.content).into_owned(),
        };

        Ok(format!("[{:?}] {}", paste.mimetype, content))
    }
}

#[tool_handler]
impl ServerHandler for PastebinMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(
                "MCP interface over the pastebin server - lets you create and view pastes, including binary (OctetStream) content via base64.",
            )
    }
}

/// Builds the tower service exposing the MCP server on a single Streamable HTTP endpoint.
pub fn mcp_service(state: AppState) -> StreamableHttpService<PastebinMcp, LocalSessionManager> {
    StreamableHttpService::new(
        move || Ok(PastebinMcp::new(state.clone())),
        Arc::new(LocalSessionManager::default()),
        Default::default(),
    )
}
