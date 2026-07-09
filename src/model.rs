struct Paste {
    id: Uuid,
    content: Vec<u8>,          // binární i textový obsah
    mimetype: MimeKind,        // enum: PlainText, Html, Markdown, OctetStream
    hits: u32,                 // počítadlo pro LRU
    last_seen_tick: u64,       // "generace" naposledy zobrazeno, pro nalezení kandidáta k evikci
}

enum MimeKind {
    PlainText,
    Html,
    Markdown,
    OctetStream,
}