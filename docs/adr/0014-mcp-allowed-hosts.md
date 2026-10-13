# 0014 — Konfigurovatelné povolené hosty pro MCP (`MCP_ALLOWED_HOSTS`), předávané parametrem, ne přes `AppState`

## Status

Accepted

## Kontext

Po nasazení na veřejnou doménu vracel `POST /mcp` na každý request `403 Forbidden: Host header is not allowed`, zatímco běžné HTTP endpointy fungovaly. Příčina je v `rmcp` 3.1.4: `StreamableHttpServerConfig::default()` má `allowed_hosts: ["localhost", "127.0.0.1", "::1"]` a každý request s jinou hlavičkou `Host` odmítne. Je to ochrana proti DNS rebinding útokům na lokálně běžící MCP servery. `mcp::mcp_service` předával `Default::default()` a aplikace neměla žádnou proměnnou prostředí, kterou by šel seznam rozšířit.

Zvažované možnosti:
- **`disable_allowed_hosts()`** — zamítnuto: povolí jakýkoli `Host` a tím úplně vypne ochranu proti DNS rebindingu (sama dokumentace `rmcp` to pro veřejná nasazení nedoporučuje).
- **Zadrátovat veřejný hostname do kódu** — zamítnuto: každé nasazení (jiná doména, jiný port) by vyžadovalo nový build.
- **Nová volitelná proměnná `MCP_ALLOWED_HOSTS`**, jejíž hodnoty se přidají k výchozím loopback adresám — zvoleno.

## Rozhodnutí

**`MCP_ALLOWED_HOSTS`** — nepovinná proměnná prostředí, seznam hostů oddělený čárkami (např. `paste.example.com,paste.example.com:8443`). `main.rs` začne s `["localhost", "127.0.0.1", "::1"]` (stejné hodnoty jako výchozí konfigurace `rmcp`, lokální vývoj tedy funguje beze změny), přidá k nim ořezané neprázdné položky z proměnné a výsledek předá dál. Na rozdíl od ostatních proměnných chybějící `MCP_ALLOWED_HOSTS` nezpůsobí pád při startu.

**Sémantika položek je převzatá z `rmcp`:** položka bez portu (`paste.example.com`) povolí daný host na jakémkoli portu, položka s portem (`paste.example.com:8080`) jen ten jeden port.

**Seznam se předává parametrem `routes::create_router(state, &mcp_hosts)` → `mcp::mcp_service(state, allowed_hosts)`, ne přes `AppState`.** `AppState` drží sdílený stav za běhu, který čtou handlery při každém requestu (`PasteStore`, `StyleStore`). Povolené hosty jsou konfigurace pro start: použijí se jednou při sestavení routeru, uloží se do `StreamableHttpServerConfig` (`with_allowed_hosts`) a žádný handler je za běhu nepotřebuje. V `AppState` by se zbytečně klonovaly do každého handleru a nebylo by vidět, kdo na nich závisí. Parametr dělá závislost explicitní a testy si můžou postavit router s libovolným seznamem.

## Důsledky

**Pozitivní:**
- MCP funguje i na veřejné doméně, ochrana proti DNS rebindingu zůstává zapnutá.
- Lokální vývoj ani stávající nasazení na loopbacku nevyžadují žádnou změnu konfigurace.
- Pokryto integračními testy v `tests/routes_test.rs` (`test_mcp_allows_loopback_host_by_default`, `test_mcp_rejects_public_host_not_in_allowed_hosts`, `test_mcp_allows_configured_public_host`, `test_mcp_allowed_host_with_port_matches_only_that_port`). Jsou to první automatizované testy MCP endpointu (dřív jen manuální ověření).

**Negativní:**
- Za reverse proxy rozhoduje hlavička `Host`, která dorazí do aplikace. Pokud ji proxy zachovává (běžné), musí být veřejný hostname v `MCP_ALLOWED_HOSTS`; pokud ji přepisuje na upstream (`127.0.0.1:3000`), projde i bez konfigurace. Chování tedy závisí na konfiguraci proxy mimo tento repozitář.
- Loopback seznam v `main.rs` opakuje výchozí hodnoty `rmcp` — kdyby se v budoucí verzi `rmcp` změnily, nerozjedou se automaticky.
- Kontrola hlavičky `Origin` (`with_allowed_origins`) zůstává vypnutá (výchozí prázdný seznam) — viz otevřené otázky v `requirements.md`.
