# 0009 — Zachování původního názvu souboru u `OctetStream` pastů

## Status

Accepted

## Kontext

`Content-Disposition` hlavička u `GET /paste/{uuid}` pro `MimeKind::OctetStream` byla natvrdo `attachment; filename="{uuid}.bin"` — bez ohledu na to, jaký soubor uživatel skutečně nahrál. V praxi to znamenalo, že třeba nahraný `.png` obrázek se stáhl jako `<uuid>.bin` a operační systém/prohlížeč ho bez přípony neuměl rovnou otevřít správnou aplikací, i když šlo o naprosto validní obsah — jediný chybějící kus informace byl původní název souboru, který se nikde neukládal.

Druhý, související problém: po vytvoření `OctetStream` pastu přes formulář na `GET /` frontend přesměroval prohlížeč rovnou na `/paste/{uuid}`. Protože `Content-Disposition: attachment` donutí prohlížeč soubor rovnou stáhnout, na obrazovce se po přesměrování nic nezobrazilo — uživatel neviděl UUID svého nového pastu vůbec, jen viděl, že se něco stáhlo do Stažených souborů.

## Rozhodnutí

**Doména:** `Paste` (`model.rs`) dostal nové pole `file_name: Option<String>`, `Paste::new` ho bere jako parametr a `PasteStore::insert` ho posílá dál. `None` znamená "název souboru neznámý/nezadaný" — platí pro všechny textové mimetypy a pro `OctetStream` pasty vytvořené bez názvu.

**Sanitizace:** `model::sanitize_file_name(&str) -> Option<String>` odstraní řídicí znaky a uvozovky (`"`, ty by rozbily `Content-Disposition: attachment; filename="..."` syntaxi), ořízne název na 255 znaků a vrátí `None`, pokud po očištění nic nezbyde. Používá se na obou vstupních cestách (HTTP i MCP), aby se do hlavičky nikdy nedostal nevalidní/nebezpečný název.

**HTTP (`POST /paste/binary`):** Frontend (`templates/home.html`) posílá původní název souboru v hlavičce `X-File-Name-B64` — base64 zakódovaný, protože HTTP hlavičky povolují jen ASCII a název souboru může obsahovat libovolný Unicode (`obrázek.png`). Base64 se zvolilo záměrně místo percent-encode/`encodeURIComponent`: `base64` crate je v projektu už závislostí kvůli MCP (viz [ADR 0008](./0008-base64-mcp-binary-tool.md)), takže se tím nepřidává nová závislost a dekódování na backendu (`BASE64.decode(...)`) je jednodušší a bezpečnější než ruční percent-decode (žádné riziko panicnutí na neplatném indexu uprostřed UTF-8 znaku). `post_paste_binary` handler hlavičku čte volitelně (`headers.get(...)`) — chybějící nebo nevalidní hlavička (špatný base64, prázdný/jen-oddělovače název po sanitizaci) tiše vede k `file_name: None`, ne k chybě požadavku.

**MCP (`create_binary_paste`):** Nový volitelný argument `file_name: Option<String>` u `CreateBinaryPasteArgs` — žádné kódování navíc není potřeba, protože JSON-RPC argumenty jsou už samy o sobě Unicode text.

**Rendering:** `render_paste_page` u `OctetStream` použije `paste.file_name.clone().unwrap_or_else(|| format!("{}.bin", paste.id))` jako `filename` v `Content-Disposition`. Fallback na `{uuid}.bin` zůstává beze změny pro pasty bez známého názvu (staré i nově vytvořené bez hlavičky/argumentu).

**Frontend UX po vytvoření (volba B z několika zvažovaných):** Protože `GET /paste/{uuid}` pro `OctetStream` vždy spustí stažení a nezobrazí žádnou stránku, `home.html` po úspěšném `POST /paste/binary` nenaviguje pryč, ale zobrazí panel přímo pod formulářem s ID pastu a odkazem na stažení (`/paste/{uuid}`). Ostatní mimetypy (`PlainText`/`Html`/`Markdown`) se chovají beze změny — po vytvoření se přesměrují na `/paste/{uuid}`, protože tam se obsah normálně vyrenderuje a je vidět. Zvažované alternativy:
- **Samostatná info stránka + dedikovaný download endpoint** (`GET /paste/{uuid}` by vracel HTML info o pastu, skutečné stažení by šlo přes nový `GET /paste/{uuid}/download`) — řešilo by problém obecně, i pro odkaz sdílený mimo formulář, ale vyžadovalo by novou route, šablonu a rozhodnutí o tom, kdy přesně počítat `hits`/`last_seen_tick`.
- **Content negotiation** přes query parametr / `Accept` hlavičku na stejné routě — jedna route, ale méně objevitelné pro API klienty.

Zvolena byla nejjednodušší varianta (B), protože řeší bezprostřední problém (uživatel po vytvoření vidí ID) s minimální změnou; obecnější případ (přímý odkaz sdílený mimo formulář pořád jen stáhne soubor beze stránky) zůstává neřešený — viz Důsledky a otevřená otázka níže.

## Důsledky

**Pozitivní:**
- Binární pasty (obrázky, archivy, ...) se dají po stažení rovnou otevřít správnou aplikací — ověřeno živě s reálným `.png` a Unicode názvem (`obrázek.png`), `Content-Disposition` header nesl správný název beze změny.
- Sanitizace je sdílená mezi HTTP a MCP cestou (jedna funkce, `model::sanitize_file_name`), takže obě rozhraní mají stejné záruky (žádné uvozovky, žádné řídicí znaky, max. 255 znaků).
- Chybějící/nevalidní `X-File-Name-B64` hlavička nebo `file_name` MCP argument nikdy nezpůsobí chybu vytvoření pastu — jen tichý fallback na `{uuid}.bin`, stejně jako předtím.

**Negativní:**
- Uživatel, kterému někdo pošle přímý odkaz `/paste/{uuid}` na `OctetStream` paste mimo formulář na `GET /`, pořád jen dostane vynucené stažení bez jakékoli stránky/kontextu — varianta B tenhle obecnější případ neřeší, jen tok "vytvoř přes formulář → uvidíš ID".
- Dvě různá místa, kde se název souboru čte/kóduje (HTTP hlavička jako base64, MCP argument jako prostý string) — nekonzistence stejného druhu, jaká už existuje mezi `POST /paste/binary` (syrové bajty) a `create_binary_paste` (base64 obsah), viz [ADR 0008](./0008-base64-mcp-binary-tool.md).
