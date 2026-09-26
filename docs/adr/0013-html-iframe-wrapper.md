# 0013 — `Html` dostal iframe wrapper, `/raw` se zobecnil, sjednocený dispatch v `render.rs`

## Status

Accepted

## Kontext

Po [ADR 0011](./0011-octet-stream-preview-page.md) a [ADR 0012](./0012-plain-text-html-page.md) měly `OctetStream`, `PlainText` i `Markdown` vlastní stránkovou šablonu — `Html` zůstal jediný typ, který `GET /paste/{uuid}` vracel úplně beze změny (`NFR-7`), bez jakéhokoli vizuálního kontextu nebo cesty zpět na domovskou stránku.

Zvažované možnosti (viz konzultace před implementací):
- **Nechat beze změny** — nejbezpečnější, ale `Html` by zůstal jediný typ bez tlačítka zpět.
- **Obalit uživatelův vstup do vlastní šablony přímo** — zamítnuto: uživatelův paste je často celý dokument (`<html>`, `<head>`, vlastní `<style>`); obalením by vzniklo nevalidní/zdvojené HTML a kolidující CSS (naše `:root` proměnné, univerzální selektory), navíc by to porušilo `NFR-7` doslova.
- **`<iframe>` split** — zvoleno. Zachovává `NFR-7` (obsah zůstává byte-přesně beze změny, jen se přesunul na `/raw`), bez rizika CSS/HTML kolize.

## Rozhodnutí

**Nová šablona `templates/html_page.html`** — tlačítko `← Back to home` + `<iframe src="/paste/{uuid}/raw">` s **pevnou výškou** (`80vh`, scrollbar uvnitř, ne JS auto-resize — vědomě zvolená jednodušší varianta) a bílým pozadím (obsah pastu může být cokoliv, nemusí počítat s tmavým motivem aplikace).

**`render.rs`**:
- `render_html_page(paste)` — postaví wrapper stránku, do HTML nejde nic z `paste.content` přímo (jen server-generovaná `/raw` URL), takže tu není potřeba escapování jako u `file_name`/`PlainText` (ADR 0011/0012).
- `render_html_raw(paste)` — vrátí `paste.content` beze změny, `Content-Type: text/html`, **bez** `Content-Disposition` (na rozdíl od `OctetStream`, tady jde o zdroj pro `<iframe>`, ne o stažení).
- `render_raw_content` (dispatcher pro `GET /paste/{uuid}/raw`, zavedený už v ADR 0011 jen pro `OctetStream`) se rozšířil o `Html` → `render_html_raw`; `PlainText`/`Markdown` → `AppError::NotFound` (404) — nemají žádnou "raw" formu odlišnou od své hlavní stránky, takže `/raw` u nich nemá co vrátit (dřív, před touto změnou, `/raw` byl fakticky jen pro `OctetStream` a jiné typy nebyly ošetřené vůbec).
- **`render_paste_page` zjednodušen na čistý dispatch** — `match paste.mimetype { ... }` s jedním řádkem na typ, každý volá vlastní pomocnou funkci (`render_plain_text_page`, `render_html_page`, `render_markdown_page`, `render_octet_stream_page`). `Markdown` dřív byl jediný typ renderovaný inline přímo v matchi (zbytek `response`/`Ok(response)` obalu) — teď má vlastní `render_markdown_page(paste, style_store)` stejně jako ostatní tři, žádná výjimka.

**Tlačítko zpět doplněno i do zbylých tří šablon** (`markdown.html`, `octet_stream.html`, `plain_text.html`) — stejná `.back-link`/`.back-link:hover` třída a umístění jako v `html_page.html`. Všechny čtyři typy pastů teď mají konzistentní navigaci zpět na `GET /`.

**Vědomě žádná bezpečnostní izolace** — `<iframe>` je stejná origin jako zbytek aplikace, takže JS uvnitř pastu se pořád může dostat na `window.top`/`parent` a manipulovat celou stránku. To není regrese (dřív měl `Html` paste plnou kontrolu nad stránkou úplně stejně, jen bez iframu), ale taky to neznamená žádný nový bezpečnostní přínos — je to čistě vizuální/layoutové oddělení.

## Důsledky

**Pozitivní:**
- `Html` je teď konzistentní se zbylými třemi typy (vlastní stránka, tlačítko zpět), `NFR-7` ("beze změny") zůstal doslova pravdivý — jen se přesunul z `GET /paste/{uuid}` na `GET /paste/{uuid}/raw`.
- `render_paste_page` je čitelnější a snáz rozšiřitelný — jednotný vzor "jeden mimetype, jedna pomocná funkce", žádná speciální inline výjimka pro `Markdown`.
- Pokryto testy (`render.rs`: `html_page_embeds_iframe_to_raw`, `html_raw_is_unchanged`, `raw_content_not_available_for_plain_text_or_markdown`; `tests/routes_test.rs`: `test_get_paste_html_shows_iframe_wrapper`, `test_get_paste_raw_html_is_unchanged`, `test_get_paste_raw_not_available_for_plain_text`).

**Negativní:**
- **Breaking change** — `GET /paste/{uuid}` u `Html` už nevrací uživatelův obsah přímo (`test_get_paste_html` muselo být přejmenováno/přepsáno na kontrolu `<iframe>` wrapperu); syrový obsah je teď na `GET /paste/{uuid}/raw`.
- Pevná výška `iframe`u (`80vh`) znamená, že velmi krátký `Html` paste nechá pod obsahem prázdné místo a velmi dlouhý bude scrollovat uvnitř malého okna — bez JS auto-resize se výška nepřizpůsobí obsahu.
- `<iframe>` nedává žádnou skutečnou izolaci (stejná origin) — zdokumentováno jako otevřená otázka, ne jako vyřešené bezpečnostní opatření.
