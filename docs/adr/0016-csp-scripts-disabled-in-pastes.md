# 0016 — Skripty v pastech zakázané na úrovni aplikace (CSP `sandbox`), bezpečnostní hlavičky posílá aplikace

## Status

Accepted

Mění důsledky NFR-9 ([ADR 0013](./0013-html-iframe-wrapper.md)): `Html` paste už v prohlížeči nespustí JavaScript.

## Kontext

Obsah pastu mohl spustit JavaScript pod doménou pastebinu (stored XSS) — u `Html` pastů záměrně (NFR-9), u `Markdown` nezáměrně, protože výstup `pulldown-cmark` se nesanitizuje (NFR-17). Skript v pastu by mohl číst nebo podvrhávat cookies dostupné pro doménu a přepsat stránku (phishing z důvěryhodné adresy).

V produkčním nasazení přidávala reverse proxy na `/paste/*` hlavičku `Content-Security-Policy: sandbox …` bez `allow-scripts`, takže skripty v pastech už neběžely. Ochrana ale nebyla v repozitáři, nehlídaly ji testy a neplatila při spuštění Docker image bez této proxy (issue #17).

Zvažované možnosti:
- **Nechat ochranu jen v proxy** — zamítnuto: bezpečnost aplikace by závisela na konfiguraci mimo repozitář.
- **Sanitizovat Markdown (`ammonia`) a `Html` nechat beze změny** — samo o sobě nestačí: `Html` paste by dál spouštěl skripty ve stejné origin. Sanitizace Markdownu zůstává jako volitelné vylepšení (issue #18).
- **Servírovat `/raw` z oddělené domény** — zamítnuto pro teď: vyžaduje druhou doménu a změnu nasazení. Smysl má jen tehdy, pokud by JS v `Html` pastech měl fungovat, a to není potřeba.
- **Zakázat skripty ve všech stránkách s uživatelským obsahem přímo v aplikaci** — zvoleno. Skripty v pastech na produkci stejně neběží, takže uživatelé nepřijdou o nic, co dnes funguje.

## Rozhodnutí

**Nový modul `src/security.rs`** s middlewarem `security_headers` (`axum::middleware::from_fn`), zapojeným v `routes::create_router` nad celým routerem včetně `/mcp`. Politika se vybírá podle cesty:

| Cesta | Hlavičky |
|---|---|
| `/` (domovská stránka) | `HOME_CSP` (`script-src 'self' 'unsafe-inline'` pro vlastní skript formuláře, `frame-ancestors 'none'`), `X-Frame-Options: DENY` |
| `/mcp` | žádná CSP — vrací JSON-RPC, ne dokument |
| vše ostatní (`/paste/*`, 404, neznámé cesty) | `PASTE_CSP` |
| všude | `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer` |

`PASTE_CSP` přebírá hodnotu z produkční proxy:

```
sandbox allow-same-origin allow-downloads; default-src 'none'; style-src 'unsafe-inline';
img-src 'self' data: https:; frame-src 'self'; form-action 'none'; base-uri 'none'; frame-ancestors 'self'
```

- `sandbox` bez `allow-scripts` — prohlížeč nespustí `<script>`, inline handlery (`onerror=`) ani `javascript:` odkazy; zároveň blokuje formuláře, vyskakovací okna a navigaci nadřazeného okna.
- `allow-same-origin` — iframe na stránce `Html` pastu smí načíst `/raw` (`frame-src 'self'`). Bez povolených skriptů tím dokument nezíská přístup ke cookies.
- `allow-downloads` — funguje stažení `OctetStream` souboru.
- `style-src 'unsafe-inline'` — potřebují ho šablony a inline `style` ze `syntect`.

**Výchozí je zákaz:** sandboxovaná CSP platí pro všechno kromě `/` a `/mcp`, takže nová route s uživatelským obsahem je chráněná bez úpravy middlewaru.

HSTS a `Permissions-Policy` zůstávají v reverse proxy — HSTS patří k TLS terminaci, aplikace sama běží přes HTTP.

## Důsledky

**Pozitivní:**
- JavaScript z `Html` ani `Markdown` pastu se v prohlížeči nespustí, bez ohledu na reverse proxy (lokálně, v Dockeru i na produkci).
- Pokryto integračními testy v `tests/routes_test.rs` (domovská stránka bez sandboxu, stránka Markdown pastu, stránka a `/raw` `Html` pastu, 404 a neznámá cesta, `/mcp` bez CSP). Bez middlewaru všech pět testů selže.
- Proxy může své CSP hlavičky ponechat jako druhou vrstvu — prohlížeč vynucuje obě politiky zároveň, při stejných hodnotách se chování nemění.

**Negativní:**
- **`Html` pasty už neumí JavaScript** — jde o změnu chování oproti NFR-9 ve verzi bez proxy. Interaktivní HTML stránky (např. tlačítka s `onclick`) se zobrazí, ale nebudou reagovat. Domovská stránka na to upozorní nápovědou, která se ukáže po výběru `Html` (`#html-hint` v `templates/home.html`).
- Výstup Markdownu je dál nesanitizovaný (NFR-17): HTML vložené do Markdownu se zobrazí, jen bez skriptů — `<style>` nebo pozicované prvky můžou stránku vizuálně podvrhnout. Řeší volitelně issue #18.
- `img-src … https:` dovoluje externí obrázky, které prozradí IP adresy čtenářů. Ponecháno kvůli kompatibilitě s obrázky v Markdownu.
- Ochrana závisí na podpoře CSP `sandbox` v prohlížeči — velmi staré prohlížeče ji ignorují.
- Domovská stránka potřebuje `'unsafe-inline'` pro vlastní inline skript; uživatelský obsah na ni ale nikdy nejde.
