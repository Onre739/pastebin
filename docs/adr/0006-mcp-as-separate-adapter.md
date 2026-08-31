# 0006 — MCP server jako samostatný modul se Streamable HTTP transportem

## Status

Accepted

## Kontext

Bylo potřeba zpřístupnit doménovou logiku aplikace (vytváření a zobrazování pastů) i klientům komunikujícím přes Model Context Protocol (typicky LLM agentům), ne jen přes běžné HTTP endpointy. `rmcp` (oficiální Rust SDK pro MCP) nabízí dvě transportní varianty: starší HTTP+SSE (dva endpointy — jeden `GET` pro SSE stream, jeden `POST` pro JSON-RPC zprávy, spárované přes session ID) a novější Streamable HTTP (jeden endpoint, `POST` pro JSON-RPC, volitelně `GET` pro server-initiated stream na tom samém pathu).

Zároveň bylo potřeba rozhodnout, kam MCP kód umístit — jestli definice nástrojů (tools) přidat přímo do `routes.rs`, nebo je vydělit do vlastního modulu.

## Rozhodnutí

Zvolen **Streamable HTTP transport** (`rmcp` feature `transport-streamable-http-server`) — jeden endpoint `POST /mcp`, bez zvláštního SSE endpointu.

MCP kód žije v samostatném modulu `src/mcp.rs`, ne v `routes.rs`:
- `PastebinMcp` — tenká struktura držící `state: AppState` (stejné dva `Arc` ukazatele jako `routes.rs`, ne kopie ani oddělený stav).
- `#[tool_router]`/`#[tool]` makra definují nástroje `create_paste` a `get_paste` jako metody na `PastebinMcp`, volající přímo `PasteStore::insert`/`PasteStore::record_view` — stejné veřejné API, jaké používá `routes.rs`, žádná duplicitní doménová logika.
- `#[tool_handler]` na `impl ServerHandler for PastebinMcp` generuje `call_tool`/`list_tools`/`get_tool` dispatch nad `#[tool_router]`; vlastní `get_info()` jen doplňuje popis serveru.
- `mcp_service(state: AppState) -> StreamableHttpService<...>` zapouzdřuje sestavení `StreamableHttpService` (factory closure `move || Ok(PastebinMcp::new(state.clone()))` + `LocalSessionManager`) a vrací tower službu.
- `routes::create_router` MCP službu jen připojuje přes `.nest_service("/mcp", mcp_service(state.clone()))` — routing kompozice, ne prolnutí odpovědností.

`mcp.rs` záměrně nezávisí na `render.rs` — `get_paste` vrací obsah pastu jako surový text (u markdownu tedy zdrojový text, ne vyrenderované HTML), ne přes `render::render_paste_page` (ta je vázaná na `axum::Response`, což pro MCP textový výsledek nedává smysl).

## Důsledky

**Pozitivní:**
- `routes.rs` zůstává jednoúčelový (HTTP handlery), MCP je čistě druhý adaptér nad stejnou doménou — přesně tvar, který už zavedl repository pattern nad `PasteStore` (ADR 0004): dva spotřebitelé, jedno doménové jádro.
- Streamable HTTP (jeden endpoint) je jednodušší na mount do existujícího axum routeru než legacy SSE transport (dva endpointy, ruční párování session ID) a je to i aktuálně doporučovaná varianta specifikace.
- Paste vytvořený přes MCP je okamžitě viditelný přes běžné HTTP endpointy a naopak — žádný oddělený stav k synchronizaci, protože `PastebinMcp` sdílí ten samý `AppState`.
- Chybové stavy (`AppError`) se v `mcp.rs` mapují na `isError: true` s textovým popisem přes `format!("{:?}", e)`, ne na HTTP status — MCP nástroj tedy nikdy nepanikaří ani nevrací nesrozumitelnou chybu, i když je to jiná hranice pro chyby, než jakou má `routes.rs` (`IntoResponse`).

**Negativní:**
- `PasteStore`'s veřejné API má teď dva nezávislé volající (`routes.rs`, `mcp.rs`) — jakákoli budoucí změna signatury `insert`/`record_view` znamená úpravu na dvou místech.
- `get_paste` u `Markdown` vrací jiný výstup (surový zdroj) než ekvivalentní HTTP cesta (vyrenderované HTML) — je to záměr, ale je to nekonzistence chování pro "stejnou" operaci podle protokolu, kterým se zavolá.
- MCP endpoint nemá (stejně jako HTTP vrstva) žádnou autentizaci ani rate-limiting — rozšiřuje existující bezpečnostní riziko (NFR-8/NFR-9 v `requirements.md`) o další vstupní bod.
- Vlastní `get_info()` nepředává jméno/verzi projektu, takže se MCP server v `initialize` odpovědi identifikuje jako `rmcp`/verze knihovny, ne jako `pastebin` — kosmetický nedodělek.
