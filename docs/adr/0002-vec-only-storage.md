# 0002 — Úložiště pastů výhradně přes `Vec`

## Status

Accepted

## Kontext

Zadání explicitně zakazuje pro úložiště pastů asociativní/indexované kolekce (`HashMap`, `BTreeMap`, `VecDeque`, `HashSet`, prioritní frontu) — vyhledání pastu podle `Uuid` má být záměrně lineární. Cílem je procvičit implementaci vlastního evikčního mechanismu nad nejjednodušší možnou datovou strukturou, ne spoléhat na standardní knihovní kolekce, které by evikci (a hledání) zjednodušily na `O(1)`/`O(log n)`.

## Rozhodnutí

`PasteStore.pastes` je `Vec<Paste>`. Veškeré vyhledání probíhá přes `pastes.iter().position(|p| p.id == uuid)` (`O(n)`). Evikční algoritmus (`find_last_seen_paste`, `process_full_capacity`) pracuje výhradně s `Vec` i pro pomocné bookkeeping struktury — konkrétně `skip_mask: Vec<bool>` (indexovaná přímo podle pozice v `pastes`), ne `HashSet<usize>`, přestože by množina indexů byla přirozenější volba pro "které indexy už byly vyzkoušeny v tomto kole evikce". Toto omezení bylo záměrně dodrženo i pro tuto pomocnou strukturu, ne jen pro hlavní úložiště.

## Důsledky

**Pozitivní:**
- Splňuje explicitní požadavek zadání.
- Žádné dodatečné závislosti/kolekce, jednoduchý mentální model (jedno pole, žádný sekundární index k udržování v konzistenci).

**Negativní:**
- Vyhledání pastu (`record_view`) je `O(n)` místo `O(1)`.
- `find_last_seen_paste` je `O(n)` na jedno volání (lineární sken + `O(1)` kontrola přeskočení přes `skip_mask`); `process_full_capacity` ho může v nejhorším případě zavolat opakovaně, čímž se evikce jedné položky dostane až na `O(n²)`.
- Při řádovém nárůstu `MAX_PASTES` (desítky tisíc a více) by tohle znatelně zpomalilo jak čtení, tak evikci — viz NFR-6 v `requirements.md`.
