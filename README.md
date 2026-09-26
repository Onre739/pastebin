# Pastebin by Onre

In-memory pastebin server napsaný v Rustu (axum + tokio). Pasty se ukládají výhradně v paměti procesu (žádná databáze, žádná perzistence na disk) a při dosažení kapacity se nejstarší nepoužívané pasty automaticky uvolňují vlastním countdown LRU mechanismem nad `Vec`.

Podrobný popis požadavků a architektury:
- [`docs/requirements.md`](docs/requirements.md) — doménový slovník, funkční a nefunkční požadavky, akceptační kritéria
- [`docs/architecture.md`](docs/architecture.md) — C4 diagramy, vlastnictví stavu, aplikace návrhových vzorů
- [`docs/adr/`](docs/adr/) — jednotlivá architektonická rozhodnutí

## Funkce

- Vytvoření pastu přes JSON (`POST /paste/json`) nebo form-urlencoded (`POST /paste/form`)
- Nahrání skutečně binárního souboru beze změny přes `POST /paste/binary` (syrové tělo requestu, žádná JSON/base64 obálka) — homepage pro `Octet Stream` mimetype nabízí místo textového pole výběr souboru kliknutím nebo přetažením (drag & drop kdekoli na stránce)
- Zachování původního názvu souboru u binárních pastů — `Content-Disposition` při stažení použije skutečný název (např. `photo.png`), ne generický `{uuid}.bin` (viz [ADR 0009](docs/adr/0009-original-file-name-preservation.md))
- Náhledová stránka pro binární pasty (`GET /paste/{uuid}`) — jméno souboru, velikost, a pro rozpoznané obrázkové přípony inline náhled; syrová data (stažení) jsou na `GET /paste/{uuid}/raw`, které se nepočítá jako zobrazení (viz [ADR 0011](docs/adr/0011-octet-stream-preview-page.md))
- Zobrazení pastu podle UUID (`GET /paste/{uuid}`), 404 pro neexistující/evikovaný paste
- Renderování podle typu obsahu (`PlainText`, `Html`, `Markdown`, `OctetStream`) — `PlainText` i `Markdown`/`OctetStream` mají vlastní tmavou HTML šablonu; jen `Html` se vrací beze změny (viz Omezení). Obsah/metadata vkládaná do těchto šablon (text pastu, název souboru) jsou escapovaná, aby paste nemohl spustit vlastní JS v prohlížeči diváka (viz [ADR 0011](docs/adr/0011-octet-stream-preview-page.md), [ADR 0012](docs/adr/0012-plain-text-html-page.md))
- Markdown → HTML (`pulldown-cmark`), zvýraznění syntaxe v blocích kódu (`syntect`, tmavé téma), bloky ```mermaid``` renderované na SVG (`mermaid-svg`) — výstup je obalený do stejné tmavé šablony jako homepage
- Countdown LRU evikce při naplnění kapacity
- Validace vstupu (prázdný obsah, příliš velký payload, neplatný mimetype) — textové pasty a binární soubory mají oddělený limit velikosti
- MCP server na `POST /mcp` (Streamable HTTP transport, `rmcp`) — stejná doména vystavená i přes Model Context Protocol pro MCP klienty (např. LLM agenty)

## MCP rozhraní

Server na `POST /mcp` vystavuje Model Context Protocol server (Streamable HTTP transport — jeden endpoint pro JSON-RPC i server-sent stream, bez samostatného SSE endpointu). Implementace je v `src/mcp.rs`, sdílí stejný `AppState`/`PasteStore` jako HTTP vrstva — paste vytvořený přes MCP je hned viditelný i přes `GET /paste/{uuid}` a naopak.

Dostupné nástroje:
- `create_paste(content, mimetype)` — vytvoří nový textový paste (`PlainText`/`Html`/`Markdown`), vrátí jeho UUID (stejná validace jako `POST /paste/json`)
- `create_binary_paste(content_base64, file_name?)` — vytvoří nový paste typu `OctetStream` z base64 zakódovaného obsahu; MCP obdoba `POST /paste/binary` — JSON-RPC neumí přenést syrové bajty, takže base64 je jediná cesta, jak binární data protlačit přes MCP tool call (stejný vzor používá i MCP specifikace pro binární content bloky). Nepovinný `file_name` se uloží a použije v `Content-Disposition` při stažení pastu.
- `get_paste(id)` — zobrazí paste podle UUID; má stejný vedlejší efekt jako běžné zobrazení (`hits`/`last_seen_tick` se aktualizují). Textový obsah se vrací surový (u `Markdown` tedy zdrojový text, ne vyrenderované HTML), binární (`OctetStream`) obsah se jen popíše velikostí s odkazem na `GET /paste/{uuid}/raw` — čtení binárních dat zpátky přes MCP zatím není řešeno (viz `docs/requirements.md`, otevřené otázky)

## Spuštění

```
cargo run
```

Server naslouchá na adrese a portu z `.env` (výchozí `http://127.0.0.1:3000`).

## Konfigurace

Konfigurace se čte z `.env` v kořeni projektu při startu — všechny proměnné jsou povinné, jejich chybějící hodnota nebo neplatný formát způsobí pád aplikace hned při startu s chybovou hláškou.

| Proměnná | Význam | Aktuální hodnota |
|---|---|---|
| `URL` | Adresa, na které server naslouchá | `127.0.0.1` |
| `PORT` | Port, na kterém server naslouchá | `3000` |
| `MAX_PASTES` | Maximální počet pastů držených současně; po dosažení limitu se spustí countdown LRU evikce | `100` |
| `MAX_PASTE_SIZE` | Maximální velikost textového obsahu (`PlainText`, `Html`, `Markdown`) v bajtech | `1048576` (1 MiB) |
| `MAX_FILE_SIZE` | Maximální velikost binárního souboru (`OctetStream`, nahrávaného přes `POST /paste/binary`) v bajtech | `20971520` (20 MiB) |

## Testování

```
cargo test
```

Spustí unit testy (`src/store.rs`, `src/render.rs`) i integrační testy (`tests/routes_test.rs`) — dohromady pokrývají hraniční případy LRU evikce, rendering všech typů obsahu, HTTP vrstvu včetně chybových stavů a oddělené limity velikosti pro text/binární obsah.

## CI/CD

[`.github/workflows/ci.yml`](.github/workflows/ci.yml) (GitHub Actions), dva joby:
- **test** — na každý `push`/`pull_request` do `main` spustí `cargo build` + `cargo test`.
- **docker** — jen po `push` do `main` a jen když `test` uspěje: sestaví image podle `Dockerfile` a publikuje ho do GitHub Container Registry (`ghcr.io/<owner>/<repo>`, tagy `latest` a zkrácené SHA), bez extra secrets (vestavěný `GITHUB_TOKEN`).

Repozitář sám o sobě nevynucuje "merge jen po zeleném CI" — to (a viditelnost GHCR balíčku, defaultně private) je potřeba nastavit ručně v GitHub Settings. Podrobnosti a zvažované alternativy (Docker Hub, `docker-compose.yml`) viz [ADR 0010](docs/adr/0010-github-actions-ci-docker-image.md).

## Docker

```
docker build -t pastebin .

docker run --rm -p 3000:3000 \
  -e URL=0.0.0.0 \
  -e PORT=3000 \
  -e MAX_PASTES=100 \
  -e MAX_PASTE_SIZE=1048576 \
  -e MAX_FILE_SIZE=20971520 \
  pastebin
```

Image (`Dockerfile`, multi-stage build) obsahuje jen zkompilovanou binárku, žádný Rust toolchain ani `.env` soubor — všech 5 proměnných z [Konfigurace](#konfigurace) je proto nutné předat přes `-e`/`--env-file`, jinak aplikace při startu hned panicne. `URL` musí být uvnitř kontejneru `0.0.0.0`, ne `127.0.0.1` (viz sekce Konfigurace) — jinak server naslouchá jen sám sobě a `-p` mapování portu se nepropojí.

## Struktura projektu

```
src/
  main.rs     - spuštění serveru, načtení konfigurace, sestavení stavu
  lib.rs      - deklarace veřejných modulů knihovny
  routes.rs   - HTTP handlery a axum router
  mcp.rs      - MCP server (rmcp), nástroje create_paste/get_paste na POST /mcp
  store.rs    - PasteStore (úložiště + countdown LRU), StyleStore, AppState
  render.rs   - markdown/syntax highlighting/mermaid rendering, HTML šablony
  model.rs    - Paste, MimeKind, AppError
templates/    - statické HTML šablony (zakompilované přes include_str!)
  home.html         - domovská stránka s formulářem (textové pole, výběr/drag&drop souboru)
  markdown.html     - stránka pro vyrenderovaný markdown, stejný vzhled jako home.html
  plain_text.html   - stránka pro PlainText pasty (obsah v <pre>, escapovaný)
  octet_stream.html - náhledová stránka pro binární pasty (jméno, velikost, obrázkový náhled/download)
tests/        - integrační testy nad HTTP vrstvou
docs/         - požadavky, architektura, ADR
.github/workflows/ - CI/CD pipeline (GitHub Actions)
Dockerfile    - multi-stage build (kompilace → štíhlý runtime image)
```

## Omezení

Úložiště je čistě v paměti — restart procesu znamená ztrátu všech pastů. Obsah typu `Html` se vrací beze změny, bez sanitizace. Žádná autentizace ani rate-limiting. Podrobnosti a zdůvodnění viz [`docs/requirements.md`](docs/requirements.md) a [`docs/architecture.md`](docs/architecture.md).
