# 0001 — Zaznamenávat architektonická rozhodnutí formou ADR

## Status

Accepted

## Kontext

Projekt prošel v průběhu vývoje několika netriviálními architektonickými změnami (např. přechod z pozičního řazení ve `Vec` na evikci řízenou globálním tickem, zrušení samostatné `repository.rs` vrstvy). Bez zápisu důvodů se tato rozhodnutí těžko zpětně obhajují nebo revidují — chybí kontext, proč byla zvolena právě daná cesta a ne alternativa.

## Rozhodnutí

Každé netriviální architektonické rozhodnutí se zaznamená jako samostatný soubor `docs/adr/NNNN-nazev-rozhodnuti.md` se strukturou Status / Kontext / Rozhodnutí / Důsledky. Čísla souborů jsou sekvenční a nemění se (staré ADR se neupravuje zpětně — pokud je rozhodnutí zvráceno, přidá se nový ADR, který na starý odkazuje a mění jeho status na "Superseded by NNNN").

## Důsledky

**Pozitivní:**
- Rozhodnutí jsou dohledatelná v gitu spolu s kódem, prochází review jako kód.
- Nový přispěvatel (nebo autor sám po čase) rychle pochopí, proč je něco navrženo tak, jak je, bez nutnosti rekonstruovat kontext z historie commitů.

**Negativní:**
- Udržování ADR je dodatečná režie — pokud se ADR nezakládá důsledně u každé netriviální změny, dokumentace se rozejde s realitou stejně jako jakákoli jiná dokumentace mimo kód.
