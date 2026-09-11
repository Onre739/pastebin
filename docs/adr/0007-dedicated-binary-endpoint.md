# 0007 — Vyhrazený binární endpoint (`POST /paste/binary`) a oddělené limity velikosti

## Status

Accepted

## Kontext

`CreatePasteDto.content` (a stejně tak `CreatePasteArgs` v `mcp.rs`) je typu `String`. Pro `MimeKind::OctetStream` to znamená, že do pastu lze reálně uložit jen bajty, které jsou zároveň platný UTF-8 text — JSON string i form-urlencoded pole musí být validní Unicode text, takže syrová binární data (JPEG, ZIP, spustitelný soubor apod.), která obsahují bajtové sekvence mimo UTF-8, se vůbec nedají odeslat přes `POST /paste/json` ani `POST /paste/form` — request by spadl na parsování těla, nebo by se obsah cestou poškodil.

Zároveň platilo, že jediný `MAX_PASTE_SIZE` (1 MiB) byl navržený pro textové pasty (kód, log, markdown) — pro skutečné soubory (fotky, PDF, archivy) je to málo; běžná fotka z telefonu klidně zabere několik MB.

## Rozhodnutí

Přidán nový endpoint `POST /paste/binary`, který extrahuje tělo requestu přes `axum::body::Bytes` — ten vezme tělo tak, jak přišlo, bez ohledu na deklarovaný `Content-Type` a bez jakékoli textové/UTF-8 validace. Handler vždy vytvoří paste s `MimeKind::OctetStream`, žádné další pole (`mimetype`) se u tohoto endpointu neposílá, protože je pro tenhle endpoint pevně dané.

Zároveň se `PasteStore` rozdělil na dva nezávislé limity velikosti:
- `max_paste_size` — pro `PlainText`, `Html`, `Markdown` (textové mimetypy).
- `max_file_size` — pro `OctetStream`, typicky výrazně vyšší hodnota (výchozí konfigurace: 1 MiB vs. 20 MiB).

`PasteStore::insert` interně vybírá platný limit podle `mimetype` paste, který se vkládá (`max_content_size(&mimetype)`), a předává ho `Paste::new`. `DefaultBodyLimit` na úrovni axum routeru (hrubý filtr, běží dřív, než je znám mimetype) se nastavuje na `max(max_paste_size, max_file_size)`, aby nepřiříznul legitimní request dřív, než se dostane k přesné kontrole podle mimetype.

Frontend (`templates/home.html`) při výběru `Octet Stream` v mimetype selectu skryje textarea a zobrazí `<input type="file">`; při odeslání pošle vybraný `File` objekt přímo jako tělo requestu na `/paste/binary` (fetch umí `Blob`/`File` poslat jako tělo bez jakéhokoli kódování).

## Důsledky

**Pozitivní:**
- `OctetStream` teď skutečně podporuje libovolná binární data, ne jen bajty, které náhodou tvoří platný UTF-8 text — ověřeno testem s bajty jako `\x00\x9f\x92\x96\xff`.
- U `POST /paste/binary` je tělo requestu přímo obsah, žádná JSON/form obálka — `DefaultBodyLimit` a `Paste::new` tak měří přesně to samé, což řeší nesoulad popsaný v [otevřené otázce č. 2](../requirements.md#6-otevřené-otázky) requirements.md (ten nesoulad ale pro `/paste/json`/`/paste/form` dál platí).
- Oddělené limity dovolují nastavit textu a souborům rozumné, různé hranice, aniž by jedna z kategorií musela být zbytečně přísná nebo zbytečně benevolentní.
- Frontend kód se zjednodušil — u `OctetStream` se soubor posílá přímo (`body: file`), žádné čtení přes `FileReader`/`.text()`/base64.

**Negativní:**
- Veřejné API má teď tři POST endpointy pro vytvoření pastu (`/paste/json`, `/paste/form`, `/paste/binary`) místo jednoho z `POST /paste` popsaného v zadání — viz [ADR 0005](./0005-dual-post-endpoints.md) a otevřená otázka č. 1 v `requirements.md`.
- `MCP` nástroj `create_paste` (v `mcp.rs`) touto změnou neprošel — pořád bere `content: String`, takže přes MCP se skutečně binární `OctetStream` paste vytvořit nedá, jen přes HTTP `/paste/binary`. Je to nekonzistence mezi HTTP a MCP rozhraním nad stejnou doménou.
- Dvě konfigurační hodnoty (`MAX_PASTE_SIZE`, `MAX_FILE_SIZE`) k udržování místo jedné, i když `PasteStore::insert` rozhoduje mezi nimi automaticky podle `mimetype`, takže volající kód (`routes.rs`, `mcp.rs`) o tom nemusí nic vědět.
