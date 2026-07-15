use uuid::Uuid;
use serde::Serialize;
use serde::Deserialize;

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