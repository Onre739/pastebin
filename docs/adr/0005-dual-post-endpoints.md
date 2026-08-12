# 0005 — Dva oddělené POST endpointy místo jednoho s content-negotiation

## Status

Accepted, revidovatelné

## Kontext

Zadání popisuje jediný endpoint `POST /paste`, který má podle hlavičky `Content-Type` requestu přijmout buď `application/json`, nebo `application/x-www-form-urlencoded` tělo se stejnou strukturou (`content`, `mimetype`). Axum nabízí extraktory `Json<T>` a `Form<T>`, každý vázaný na jinou routu/handler signaturu — sjednocení do jednoho handleru, který sám za běhu rozhoduje podle `Content-Type`, jaký extraktor použít, by vyžadovalo buď vlastní extraktor, nebo ruční čtení těla a rozhodování v handleru.

## Rozhodnutí

Místo jednoho endpointu s dynamickým rozlišením existují dva oddělené: `POST /paste/json` (extraktor `Json<CreatePasteDto>`) a `POST /paste/form` (extraktor `Form<CreatePasteDto>`). Oba volají stejnou doménovou operaci (`PasteStore::insert`) se stejnou logikou, liší se jen ve způsobu extrakce těla requestu.

## Důsledky

**Pozitivní:**
- Využívá standardní, dobře otestované axum extraktory beze změny — žádný vlastní `FromRequest` kód.
- Chybová hlášení pro špatně formátované tělo jsou ta, která axum/serde vrací nativně pro daný formát, bez nutnosti je sjednocovat ručně.

**Negativní:**
- Neodpovídá doslovně zadání, které počítá s jedním `POST /paste`. Veřejné API má tak dvě URL pro fakticky jednu operaci ("vytvořit paste"), což je nekonzistentní s `GET /paste/{uuid}`, který je jen jeden.
- Klient musí předem vědět, na kterou z URL adres poslat request, podle formátu, který zvolil — s jedním endpointem by o tom rozhodovala jen hlavička `Content-Type`, kterou stejně posílá.
- Sjednocení do jednoho `POST /paste` je možné bez zásahu do domény (`PasteStore::insert` by se nezměnilo) — jde čistě o změnu HTTP vrstvy (`routes.rs`), ale je to veřejné rozhraní, takže případná změna je breaking change pro existující klienty.
