# 0015 — Nasazení přes GitOps (ArgoCD): CI zapisuje SHA tag image do samostatného repozitáře s manifesty

## Status

Accepted (experimentální — nasazení přes ArgoCD se zkouší a ladí)

Rozšiřuje [ADR 0010](./0010-github-actions-ci-docker-image.md): job `docker` už nekončí publikací image, ale i aktualizací GitOps repozitáře.

## Kontext

Po [ADR 0010](./0010-github-actions-ci-docker-image.md) CI na každý push do `main` publikuje image do GHCR (tagy `latest` a `sha-<7 znaků>`), samotné nasazení ale nebylo automatizované ani popsané. Aplikace se nově nasazuje do Kubernetes přes ArgoCD. ArgoCD nasazuje stav popsaný v Git repozitáři, takže každá nová verze image se musí propsat do manifestů.

Zvažované možnosti:
- **Manifesty v tomto repozitáři** — zamítnuto: commit s novým tagem by spustil další běh CI v témže repozitáři a historie aplikace by se míchala s provozními změnami (konfigurace prostředí, počet replik).
- **ArgoCD sleduje tag `latest`** — zamítnuto: změna obsahu `latest` nemění manifest, takže ArgoCD nepozná, že má co nasadit. Neměnný tag také umožňuje dohledat, která verze běží, a vrátit se zpět.
- **ArgoCD Image Updater** (sleduje registry a tag aktualizuje sám) — odloženo: další komponenta v clusteru navíc; pro učení je přímý zápis z CI průhlednější.
- **Samostatný GitOps repozitář, do kterého CI zapíše nový SHA tag** — zvoleno.

## Rozhodnutí

**Manifesty jsou v samostatném repozitáři [`Onre739/pastebin-gitops`](https://github.com/Onre739/pastebin-gitops)** (`k8s/app.yaml`). ArgoCD sleduje tento repozitář; konfigurace prostředí (`URL`, `PORT`, limity, `MCP_ALLOWED_HOSTS`) patří do manifestů tam, ne do image ani do tohoto repozitáře.

**Poslední krok jobu `docker`** (`Update image tag in GitOps repository` v `.github/workflows/ci.yml`), po úspěšném pushi image do GHCR:
1. Spočítá tag `sha-${GITHUB_SHA::7}` — stejný jako ten, který vytvoří `docker/metadata-action` přes `type=sha,format=short`. Tag se počítá přímo v kroku, ne z výstupu `steps.meta.outputs.version`: ten vrací verzi tagu s nejvyšší prioritou, tedy `latest`.
2. Naklonuje GitOps repozitář přes secret `GITOPS_PAT`.
3. `sed` nahradí v `k8s/app.yaml` každý výskyt `ghcr.io/onre739/pastebin:sha-<hex>` novým tagem.
4. Pokud se soubor změnil, commitne (`ci: update pastebin image to sha-…`) a pushne jako `github-actions[bot]`.

## Důsledky

**Pozitivní:**
- Každý merge do `main`, který projde testy, se automaticky dostane do clusteru bez ručního zásahu.
- V manifestu je vždy neměnný SHA tag — v historii GitOps repozitáře je vidět, která verze kdy běžela, a návrat zpět je `git revert` příslušného commitu.
- Provozní konfigurace je oddělená od kódu aplikace.

**Negativní:**
- **CI závisí na věcech mimo tento repozitář:** secret `GITOPS_PAT` (token s právem zápisu do `pastebin-gitops`; ideálně fine-grained, omezený jen na ten repozitář), existence `k8s/app.yaml` a formát reference na image. Pokud manifest přestane obsahovat `ghcr.io/onre739/pastebin:sha-<hex>` (např. přechod na `latest`, Kustomize `images:` nebo Helm values), `sed` nic nenahradí, krok skončí úspěšně („žádná změna“) a nasazování tiše přestane fungovat.
- **Souběh:** dva pushe do `main` těsně po sobě můžou vést k odmítnutému `git push` v GitOps repozitáři (druhý běh má zastaralý klon). Job zatím nemá `concurrency` ani `git pull --rebase` s opakováním.
- Rollback přes `git revert` v GitOps repozitáři přepíše další úspěšný běh CI — trvalý návrat ke starší verzi vyžaduje i revert v tomto repozitáři.
- Cluster musí umět stáhnout image z GHCR — balíček je po prvním publikování private (viz ADR 0010), takže buď public viditelnost, nebo `imagePullSecret` v clusteru.
