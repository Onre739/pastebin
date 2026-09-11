# 0008 — Base64 pro binární obsah v MCP (`create_binary_paste`) místo syrových bajtů

## Status

Accepted

## Kontext

Po zavedení `POST /paste/binary` (viz [ADR 0007](./0007-dedicated-binary-endpoint.md)) zůstal `create_paste` v `mcp.rs` bez obdoby — `CreatePasteArgs.content` je `String`, takže přes MCP šlo vytvořit `OctetStream` paste jen z bajtů, které náhodou tvoří platný UTF-8 text.

Na rozdíl od HTTP, kde `Bytes` extraktor vezme tělo requestu syrové, bez ohledu na formát, MCP nemá obdobný "syrový" kanál. Volání nástroje (`tools/call`) je vždy JSON-RPC zpráva a jeho argumenty (`arguments`) jsou vždy JSON objekt — a JSON string musí být validní Unicode text, nedá se do něj vložit syrová bajtová sekvence. To platí bez ohledu na transport (stdio, SSE, Streamable HTTP) i bez ohledu na to, jaký klient MCP volá — je to vlastnost samotného protokolu, ne naší implementace nebo konkrétního transportu.

Ověření: i sama MCP specifikace, když potřebuje přenést binární obsah (typy `image`/`audio` v návratových content blocích nástrojů), kóduje ho jako base64 string + `mimeType` vedle něj — žádný jiný mechanismus pro syrová binární data v protokolu neexistuje.

## Rozhodnutí

Přidán nový nástroj `create_binary_paste(content_base64: String)`. Vstupní string se dekóduje přes `base64` crate (`base64::engine::general_purpose::STANDARD`) na `Vec<u8>` a předá `PasteStore::insert` s `MimeKind::OctetStream`, stejně jako `POST /paste/binary` na HTTP straně. Neplatný base64 vrátí `isError: true` s popisnou chybou (`"Invalid base64 content: ..."`), ne pád.

`create_paste` zůstal beze změny (`content: String`, mimetype `PlainText`/`Html`/`Markdown`) — jeho popisek teď jen odkazuje na `create_binary_paste` pro binární obsah, technicky ale pořád nic nebrání zavolat ho i s `mimetype: OctetStream` (stejná mezera, jaká už existovala u `POST /paste/json`, viz ADR 0007 — nesjednocovalo se to ani tam).

Zároveň se popisky všech MCP nástrojů/parametrů v `mcp.rs` přepsaly z češtiny do angličtiny — konzistentně s tím, že jde o veřejné rozhraní pro MCP klienty (typicky LLM agenty), kde je angličtina očekávaný jazyk schémat/popisů nástrojů.

## Důsledky

**Pozitivní:**
- `OctetStream` paste s libovolným binárním obsahem lze teď vytvořit i přes MCP, ne jen přes HTTP `POST /paste/binary` — ověřeno testem s bajty mimo platný UTF-8 (`\x00\x9f\x92\x96\xff`), roundtrip přes `GET /paste/{uuid}` vrátil bajt-po-bajtu identický obsah.
- Řešení kopíruje vzor, který používá samotná MCP specifikace pro binární content bloky (base64 + mimetype) — není to izolovaná improvizace.

**Negativní:**
- Base64 přidává ~33 % overhead velikosti přenášených dat — nevyhnutelné, dokud MCP jako protokol nenabídne jiný kanál pro syrová binární data (aktuálně nenabízí).
- Vytvoření pastu má teď přes MCP dva nástroje (`create_paste`, `create_binary_paste`) místo jednoho, obdoba stejného rozštěpení na HTTP straně (`/paste/json`, `/paste/form`, `/paste/binary` — viz ADR 0005, ADR 0007). Klient musí vědět, který nástroj zavolat podle typu obsahu.
- Čtení binárního obsahu zpátky přes MCP zůstává nevyřešené — `get_paste` u `OctetStream` obsahu vrací jen počet bajtů, ne data samotná (base64 ani jinak). Zdokumentováno jako otevřená otázka v `requirements.md`.
