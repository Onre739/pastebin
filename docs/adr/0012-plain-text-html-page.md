# 0012 — `PlainText` se zobrazuje jako HTML stránka, ne `text/plain`

## Status

Accepted

## Kontext

`GET /paste/{uuid}` u `PlainText` doteď vracelo obsah beze změny s `Content-Type: text/plain; charset=utf-8` — v prohlížeči to znamenalo holý, nestylovaný text bez jakéhokoli vizuálního kontextu, na rozdíl od `Markdown` (vlastní stránková šablona od FR-9a) a nově i `OctetStream` (náhledová stránka, [ADR 0011](./0011-octet-stream-preview-page.md)). `PlainText` byl jediný zbývající textový mimetype bez vlastní šablony.

## Rozhodnutí

`render_paste_page`u `PlainText` teď vrací HTML stránku (`render::render_plain_text_page`, nová šablona `templates/plain_text.html` — stejné CSS proměnné/tmavý vzhled jako `markdown.html`/`octet_stream.html`), obsah je uvnitř `<pre>` s `white-space: pre-wrap; word-break: break-word` (zachovává bílé znaky a zalomení řádků z originálu, ale dlouhé řádky se zalamují místo vynuceného horizontálního scrollu).

Obsah pastu se do šablony vkládá až po escapování (`render::html_escape` — stejná funkce, která už chránila `file_name` u `OctetStream`, viz ADR 0011). Bez toho by `PlainText` paste s obsahem jako `<script>alert(1)</script>` spustil JS v prohlížeči diváka — dřív to bylo neškodné, protože `text/plain` se v prohlížeči nikdy neparsuje jako HTML, ale zabalením do stránky se to stává novým vektorem stejného druhu jako u `file_name`.

Na rozdíl od `OctetStream` (kde `GET /paste/{uuid}/raw` zůstává dostupné pro stažení/`<img>` zdroj) `PlainText` **žádný `/raw` endpoint nedostal** — přes HTTP tedy teď není cesta, jak dostat syrový text bez HTML obalu; jediná cesta k syrovému textu zůstává MCP nástroj `get_paste`. To je stejná asymetrie, jaká už dřív platila u `Markdown` (taky nemá HTTP raw přístup, jen MCP), takže `PlainText` se touto změnou jen sjednotil se stávajícím vzorem, ne že by nově něco rozbil oproti `Markdown`.

## Důsledky

**Pozitivní:**
- `PlainText`, `Markdown` i `OctetStream` teď mají konzistentní vizuální zacházení — každý typ obsahu má vlastní stránkovou šablonu se stejným tmavým vzhledem, jen `Html` zůstává výjimkou (viz NFR-7/NFR-9, záměrně beze změny/sanitizace).
- Pokryto testy (`render.rs`: `plain_text`, `plain_text_escapes_content`; `tests/routes_test.rs::test_get_paste_plain_text` aktualizován).

**Negativní:**
- **Breaking change** pro jakéhokoli HTTP/skriptového klienta, který spoléhal na to, že `GET /paste/{uuid}` u `PlainText` vrátí syrový text s `Content-Type: text/plain` (typicky `curl`/automatizace) — teď dostane HTML stránku s escapovaným obsahem uvnitř `<pre>`.
- Bez `/raw` endpointu (na rozdíl od `OctetStream`) nemá `PlainText` žádnou HTTP cestu k syrovému obsahu — jen MCP `get_paste`. Zdokumentováno jako otevřená otázka, zda tuhle asymetrii sjednotit (přidat `/raw` i pro `PlainText`/`Markdown`), nebo ji nechat, protože MCP už tuhle potřebu pokrývá.
