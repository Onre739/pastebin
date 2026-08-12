# 0003 — Countdown LRU řízené globálním tickem, ne pozicí ve vektoru

## Status

Accepted

## Kontext

Ranná implementace evikce určovala "nejstarší" paste podle jeho **pozice ve `Vec`** — nedávno zobrazené pasty se přesouvaly na začátek (`move_to_front`), evikční sweep procházel vektor od konce. To fungovalo jen přibližně: `Paste` sice od začátku obsahoval pole `last_seen_tick`, ale nikdy se nezapisovalo, takže bylo fakticky mrtvé, a pořadí ve `Vec` sloužilo jako jediný (nepřesný) proxy pro "kdy byl paste naposledy viděn". To vedlo k několika reálným chybám: nově vytvořený paste mohl být evikován ve stejném requestu, kdy vznikl (protože se ocitl na pozici, kde sweep začínal), a algoritmus neuměl spolehlivě rozlišit mezi "starý, ale hodně navštěvovaný" a "nový, nikdy nenavštívený" paste.

## Rozhodnutí

`PasteStore` má globální monotónně rostoucí čítač `current_tick: u64`. Každá operace, která by se dala považovat za "dotek" pastu — vytvoření (`insert`) i zobrazení (`record_view`) — si vyžádá novou hodnotu přes `get_next_tick()` a zapíše ji do `Paste.last_seen_tick`. Evikční kandidát se hledá výhradně podle této hodnoty (`find_last_seen_paste` porovnává `last_seen_tick`, ne index ve vektoru), ne podle pozice ve `Vec`. Pozice pastu ve `Vec` už nemá žádný sémantický význam pro LRU — nový paste se jednoduše připojí na konec (`self.pastes.push(paste)`) a zůstává tam, dokud ho evikce nesmaže.

## Důsledky

**Pozitivní:**
- `last_seen_tick` konečně dělá to, co jeho jméno slibuje — je to přesný, jednoznačný záznam "naposledy dotčeno v generaci X", ne aproximace přes pozici v poli.
- Nový paste už nemůže být evikován jen kvůli tomu, kde se náhodou ocitl ve `Vec` — jeho tick je vždy nejvyšší (nejnovější) v okamžiku vzniku, takže se nikdy nestane nejstarším kandidátem hned po vytvoření.
- Odstraňuje potřebu `move_to_front` (žádné přesouvání prvků ve `Vec` při každém zobrazení) — evikční algoritmus je nezávislý na pořadí prvků.

**Negativní:**
- Tick se spotřebovává jak při vytvoření, tak při zobrazení — `last_seen_tick` po prvním zobrazení nově vytvořeného pastu je tedy o 1 vyšší, než by čekal někdo, kdo počítá jen "počet zobrazení" (zdokumentováno v AC-4 v `requirements.md` a testu `hits_and_ticks_increment`).
- Vyžaduje, aby `find_last_seen_paste` uměl přeskočit kandidáty, kteří v aktuálním evikčním kole už byli vyzkoušeni a nedosáhli nuly (`skip_mask`) — bez toho by evikce vždy znovu našla stejný nejstarší tick a nikdy by se neposunula na dalšího kandidáta v případech, kdy nejstarší paste má `hits > 1`.
