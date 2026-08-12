# 0004 — `PasteStore` jako repository bez samostatné `repository.rs` vrstvy

## Status

Accepted

## Kontext

Projekt dříve obsahoval samostatný soubor `repository.rs`, který stál mezi `routes.rs` a `store.rs`/`render.rs`. Funkce v něm (`process_post`, `process_paste`, `process_homepage`) ale ve skutečnosti dělaly tři různé věci najednou: (1) volaly metody nad `PasteStore`, (2) sestavovaly HTTP odpověď podle `MimeKind` (hlavičky, `IntoResponse`), (3) obsahovaly samotnou markdown/syntax-highlighting logiku (`transform_md`), která věcně patřila do `render.rs`. Vrstva tak nebyla čistou repository abstrakcí — byla to směs orchestrace a mísení odpovědností, které se navíc dalo rozdělit mezi existující moduly beze ztráty zapouzdření, pokud `PasteStore` sám dostane dostatečně bohaté veřejné API.

## Rozhodnutí

`repository.rs` byl zrušen. Jeho odpovědnosti se rozdělily takto:
- Přístup k datům a jejich mutace (najít podle UUID, vložit, zaznamenat zobrazení, evikce) přešly na metody `PasteStore` (`insert`, `record_view`, `find_last_seen_paste`, `process_full_capacity`) v `store.rs`.
- Markdown/syntax/mermaid rendering (`transform_md`) přešel do `render.rs`.
- Sestavení HTTP odpovědi (volba `Content-Type`, `Content-Disposition`, volání správné render funkce podle výsledku z `PasteStore`) zůstalo v `routes.rs`, kde `PasteStore` a `render` volá napřímo.

`PasteStore` tím sám plní roli repository — jeho pole `pastes: Vec<Paste>` je sice v kódu `pub`, ale `routes.rs` k němu nikde nepřistupuje přímo, pouze přes výše uvedené metody.

## Důsledky

**Pozitivní:**
- O jednu vrstvu/soubor méně, žádná funkce jen "přeposílá dál" bez přidané hodnoty.
- `render.rs` už nezávisí na `PasteStore` (bere jen `&Paste`/`&StyleStore`) — jde testovat a používat nezávisle na zámku a na existenci úložiště.
- `store.rs` obsahuje veškerou LRU logiku na jednom místě, přesně podle doporučeného rozvržení modulů ze zadání.

**Negativní:**
- `PasteStore.pastes` zůstává `pub` — nic technicky nebrání budoucímu kódu sáhnout na `Vec` přímo a obejít repository rozhraní. Zapouzdření je tedy dodržené konvencí (nikdo to zatím nedělá), ne vynucené kompilátorem (`private` pole + veřejné metody by bylo přísnější varianta).
- Bez samostatného souboru je repository odpovědnost implicitní — nový čtenář kódu musí vědět, že "`store.rs` = repository", není to pojmenované stejně explicitně, jako by bylo se samostatnou vrstvou.
