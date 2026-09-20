# 0010 — CI/CD přes GitHub Actions a publikace Docker image do GHCR

## Status

Accepted

## Kontext

Do teď se `cargo build`/`cargo test` spouštěly jen manuálně, lokálně — nic nezaručovalo, že push do `main` skutečně projde buildem/testy, a neexistoval žádný verzovaný, znovupoužitelný artefakt pro nasazení (jen zdrojový kód, který si každý musí sám zkompilovat).

## Rozhodnutí

**Pipeline (`.github/workflows/ci.yml`), dva joby:**
- `test` — na každý `push`/`pull_request` do `main`: `cargo build` + `cargo test` na `ubuntu-latest` runneru (`dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2` pro cache závislostí mezi běhy).
- `docker` — `needs: test`, spustí se **jen** při `push` do `main` (ne na PR) — sestaví image podle `Dockerfile` a pushne ho do `ghcr.io/<owner>/<repo>` s tagy `latest` a zkráceným SHA (`docker/metadata-action`). Přihlášení do GHCR běží přes vestavěný `GITHUB_TOKEN` (`docker/login-action`), takže nejsou potřeba žádné extra secrets, na rozdíl od Docker Hub. Build cache je sdílená mezi běhy přes `cache-from/cache-to: type=gha`.

**Dockerfile — multi-stage build:**
- Build stage `rust:1-slim-bookworm` + `build-essential`/`pkg-config` — potřeba, protože `syntect` (viz `Cargo.toml`) táhne transitivní C závislost `onig` (Oniguruma), kterou si `onig-sys` samo zkompiluje ze zdroje při `cargo build`.
- Runtime stage `debian:bookworm-slim`, do kterého se zkopíruje jen výsledná binárka — žádný Rust toolchain, žádné zdrojové soubory. Šablony (`templates/*.html`) jsou v binárce zakompilované už při buildu přes `include_str!`, takže runtime image nemusí nic navíc kopírovat. Běží pod nerootovým uživatelem (`useradd --system`).
- `.env` je v `.dockerignore` — image ho nikdy neobsahuje. `main.rs` volá `dotenvy::dotenv()` a chybu při chybějícím souboru ignoruje (`let _ = ...`), takže konfigurace (`URL`, `PORT`, `MAX_PASTES`, `MAX_PASTE_SIZE`, `MAX_FILE_SIZE`) se do kontejneru musí předat přes `docker run -e ...` — bez nich `main.rs` hned při startu panicne (`.expect(...)`).

**Bez `docker-compose.yml`** — aplikace je jeden bezstavový proces bez databáze/cache/dalších služeb (viz NFR-3), takže `docker run` s pěti `-e` proměnnými a mapováním portu je dostatečný; compose by přidal jen kosmetické pohodlí, ne řešil reálnou potřebu orchestrace víc kontejnerů.

## Důsledky

**Pozitivní:**
- Každý push/PR do `main` je automaticky ověřený buildem a testy, bez spoléhání na to, že si to autor spustí lokálně sám.
- Každý merge do `main` automaticky produkuje verzovaný, spustitelný Docker image bez extra secrets (GHCR + `GITHUB_TOKEN`).
- Runtime image je malý a bez Rust toolchainu — obsahuje jen binárku a systémové knihovny potřebné k jejímu běhu.

**Negativní:**
- Nic v repozitáři samo o sobě nevynucuje "merge jen po zeleném CI" — to vyžaduje ručně zapnout branch protection rules v GitHub repo Settings, což není verzovaný soubor a musí se udělat mimo tento kód.
- Nově publikovaný GHCR balíček je defaultně **private** — stažení image bez přihlášení vyžaduje ručně přepnout viditelnost balíčku na public v Package settings (taky mimo verzovaný kód).
- `docker` job publikuje jen na `push` do `main`, ne podle tagů/release — každý merge přepíše `latest` a přidá jeden nový SHA tag; žádné sémantické verzování zatím není řešeno.
- Build stage je díky kompilaci `onig` ze zdroje pomalejší, než by byl s čistě rustovým regex backendem (`syntect`'s `default-fancy` feature) — přijatý kompromis, protože změna regex backendu nebyla součástí tohoto rozhodnutí.
- `docker build` podle tohoto `Dockerfile` nebylo možné doběhnout do konce na vývojovém stroji — padalo na `SSL certificate problem: unable to get local issuer certificate` už při stahování Cargo dependencies uvnitř kontejneru, což se ověřilo jako lokální síťový/certifikátový problém stroje (stejná chyba nastala i při čistém `curl` z nezměněného `rust:1-slim-bookworm` image), ne chyba `Dockerfile`. GitHub Actions runnery mají čisté připojení k internetu, takže by pipeline měla projít bez úpravy — reálný zelený běh na GitHubu ale zatím nebyl potvrzen.
