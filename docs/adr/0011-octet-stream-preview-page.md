# 0011 — Náhledová stránka pro `OctetStream` pasty a vyhrazený `/raw` endpoint

## Status

Accepted

## Kontext

I po [ADR 0009](./0009-original-file-name-preservation.md) (zachování názvu souboru) platilo, že `GET /paste/{uuid}` u `OctetStream` vždy vrátí syrové bajty s `Content-Disposition: attachment` — otevření odkazu tedy vždy jen spustí stažení, bez jakékoli viditelné stránky. Frontend to po vytvoření pastu obcházel vlastním `success-panel` hackem (nenavigovat na `/paste/{uuid}`, zobrazit ID přímo na `GET /`), ale to řešilo jen tok "vytvoř přes formulář" — přímý odkaz sdílený jinou cestou (zkopírovaný, poslaný kamarádovi) pořád skončil jen vynuceným stažením bez kontextu (zdokumentováno jako otevřená otázka č. 11 v `requirements.md`).

## Rozhodnutí

**Rozdělení na dvě URL:**
- `GET /paste/{uuid}` u `OctetStream` teď vrací HTML náhledovou stránku (`render::render_octet_stream_page`, šablona `templates/octet_stream.html` — stejné CSS proměnné/tmavý vzhled jako `home.html`/`markdown.html`) — jméno souboru, velikost, a pokud přípona v `file_name` odpovídá obrázku (`model::is_image_file_name` — `png/jpg/jpeg/gif/webp/bmp/svg/ico`), inline `<img>` náhled; jinak jen text "No preview available". Obojí doplněné tlačítkem "Download".
- `GET /paste/{uuid}/raw` (nová route) vrací to, co dřív vracelo přímo `GET /paste/{uuid}` — syrové bajty, `Content-Type: application/octet-stream`, `Content-Disposition: attachment; filename="..."` (`render::render_octet_stream_raw`). Tahle URL je zároveň `src` pro `<img>` náhled i cíl tlačítka Download — funguje pro obojí, protože `Content-Disposition: attachment` prohlížeče respektují jen při navigaci (kliknutí/adresní řádek), ne při fetchi subresource jako `<img src>`.

**`PasteStore` dostal read-only `get(uuid)`** (`store.rs`), oddělené od mutujícího `record_view`. `/raw` používá `get` — jinak by automatický fetch obrázku prohlížečem (vyvolaný `<img>` tagem při načtení náhledové stránky) tiše připočítal druhé "zobrazení" navíc k tomu, co se už započítalo při načtení samotné stránky.

**`file_name` se teď poprvé vkládá do HTML** (viditelný text + `<img alt>`), což je nový vektor pro stored XSS, pokud by se nedělalo nic navíc — soubor nazvaný např. `<script>alert(1)</script>.png` by jinak spustil JS v prohlížeči diváka. `render.rs` proto escapuje (`html_escape`: `& < > " '`) před vložením do šablony. Detekce obrázku (`is_image_file_name`) běží na neescapovaném názvu (přípona se escapováním nezmění), ale do samotné stránky se vkládá jen escapovaná verze.

**`render_paste_page` zůstala jediná funkce s matchem přes všechny 4 `MimeKind`** (ne rozdělená na `render_paste_page`/`render_paste_raw`, jak vznikla v mezikroku) — `OctetStream` arm dělá `return render_octet_stream_page(paste)`. Samostatná `render_octet_stream_raw(paste) -> Response` (bez `StyleStore`, bez `Result` — nemůže selhat) existuje jen pro `/raw`, protože je to jediný mimetype, který potřebuje své bajty servírovat i mimo hlavní stránku.

**Rozsah náhledu — jen obrázky pro první verzi.** Video/audio/PDF byly zvažované (viz konzultace před implementací), ale zůstávají mimo scope — video/audio přes `<video>`/`<audio>` by fungovalo stejně jako `<img>` (subresource, `attachment` hlavička nevadí), PDF přes `<iframe>` by ale bylo problematické, protože iframe navigace `Content-Disposition: attachment` typicky respektuje a vynutila by stažení/prázdný frame místo náhledu.

## Důsledky

**Pozitivní:**
- Přímý odkaz na `OctetStream` paste teď zobrazí užitečnou stránku (jméno, velikost, případně náhled) namísto bezkontextového vynuceného stažení — řeší otevřenou otázku č. 11 obecně, ne jen tok "vytvoř přes formulář". `success-panel` hack z ADR 0009 mohl být z `home.html` odstraněn, frontend je zase jednodušší (jednotný redirect na `/paste/{uuid}` pro všechny mimetypy).
- Detekce obrázku i escapování jsou pokryté testy (`render.rs`: `octet_stream_page_shows_image_preview`, `octet_stream_page_falls_back_to_no_preview`, `octet_stream_page_escapes_file_name`; `store.rs`: `get_does_not_increment_hits_or_tick`; `tests/routes_test.rs`: náhledová stránka, `/raw` bez vlivu na `hits`, `/raw` 404).

**Negativní:**
- **Breaking change** oproti dřívějšímu kontraktu `GET /paste/{uuid}` pro `OctetStream` (dřív rovnou bajty) — `AC-11a`, `AC-11c`, `AC-14` a MCP popisek `get_paste` musely být přepsané na nový `/raw` endpoint.
- Náhled je jen pro obrázky — video/audio/PDF zůstávají budoucí rozšíření (nová otevřená otázka).
- Detekce "je to obrázek" je čistě podle přípony v `file_name`, ne skutečný sniff bajtů — přejmenovaný nesouborový obsah na `.png` se pokusí zobrazit jako `<img>`, ale žádnou chybu to nezpůsobí: pokud bajty nejsou platný obrázek, prohlížeč jen nic nevykreslí (rozbitá ikona), server ani klient nepadá.
