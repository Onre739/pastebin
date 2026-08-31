use std::sync::Arc;

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
    #[schemars(description = "obsah pastu jako text")]
    pub content: String,
    #[schemars(description = "typ obsahu: PlainText, Html, Markdown nebo OctetStream")]
    pub mimetype: MimeKind,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetPasteArgs {
    #[schemars(description = "UUID pastu, který se má zobrazit")]
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
    #[tool(description = "Vytvoří nový paste a vrátí jeho UUID")]
    fn create_paste(&self, Parameters(args): Parameters<CreatePasteArgs>,
    ) -> Result<String, String> {
        let mut paste_store = self.state.paste_store.lock().unwrap();
        paste_store
            .insert(args.content.into_bytes(), args.mimetype)
            .map(|id| id.to_string())
            .map_err(|e| format!("{:?}", e))
    }

    #[tool(description = "Zobrazí paste podle UUID - stejně jako by ho uživatel otevřel v prohlížeči (zápočítá se zobrazení, zvýší se hits a last_seen_tick)")]
    fn get_paste(&self, Parameters(args): Parameters<GetPasteArgs>) -> Result<String, String> {
        let uuid = Uuid::parse_str(&args.id).map_err(|e| format!("Neplatné UUID: {e}"))?;

        let mut paste_store = self.state.paste_store.lock().unwrap();
        let paste = paste_store
            .record_view(uuid)
            .map_err(|e| format!("{:?}", e))?;

        let content = match paste.mimetype {
            MimeKind::OctetStream => {
                format!("binární obsah, {} bajtů (nelze zobrazit jako text)", paste.content.len())
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
                "MCP rozhraní nad pastebin serverem - umožňuje pasty vytvářet a zobrazit.",
            )
    }
}

pub fn mcp_service(state: AppState) -> StreamableHttpService<PastebinMcp, LocalSessionManager> {
    StreamableHttpService::new(
        move || Ok(PastebinMcp::new(state.clone())),
        Arc::new(LocalSessionManager::default()),
        Default::default(),
    )
}