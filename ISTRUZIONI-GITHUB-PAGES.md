# Pubblicare la versione web su GitHub Pages (gratis)

Il file `.github/workflows/pages.yml` compila `apps/cadcraft-web` su GitHub e la pubblica.
Non serve installare nulla sul tuo PC.

## 1. Crea l'account e il repository
1. Vai su <https://github.com> e crea un account (puoi usare `disegno@smpavullo.com`).
2. Clicca **New repository**. Nome, per esempio: `disegno-cad`. Scegli **Public**
   (le GitHub Actions e Pages sono gratis per i repository pubblici).
3. Non aggiungere README, .gitignore o licenza: il repository deve restare vuoto.

## 2. Carica il progetto
Da PowerShell, nella cartella del progetto:

```powershell
cd C:\Users\profa\Downloads\cadcraft-main
git init -b main
git add .
git commit -m "Prima versione"
git remote add origin https://github.com/TUO-UTENTE/disegno-cad.git
git push -u origin main
```

Sostituisci `TUO-UTENTE` con il nome del tuo account GitHub.
Al primo `git push` si apre il login di GitHub nel browser.

Alternativa senza comandi: nella pagina del repository clicca **uploading an existing file**
e trascina dentro il contenuto della cartella. La cartella nascosta `.github` va caricata
anch'essa; se Windows la nasconde, usa i comandi qui sopra.

## 3. Attiva Pages
Nel repository: **Settings → Pages → Build and deployment → Source: GitHub Actions**.

## 4. Controlla la compilazione
Scheda **Actions** → workflow **Pages**. La prima volta richiede 15-30 minuti
(poi circa 5, grazie alla cache). Quando diventa verde, l'indirizzo dell'app e:

`https://TUO-UTENTE.github.io/disegno-cad/`

## 5. Workflow vecchi che falliranno
Gli altri file in `.github/workflows/` (ci, freebsd, nix, release, sonarcloud...) usano
risorse dell'organizzazione originale e nel tuo repository daranno errore.
Non bloccano Pages. Per non ricevere email di errore, cancellali e fai un nuovo commit:

```powershell
cd C:\Users\profa\Downloads\cadcraft-main\.github\workflows
Remove-Item ci.yml, freebsd.yml, nix.yml, packaging-lint.yml, release.yml, sonarcloud.yml, windows-arm64.yml
cd ..\..
git add -A
git commit -m "Tengo solo il workflow Pages"
git push
```

Sposta anche `.github/dependabot.yml` o cancellalo, se non lo vuoi.

## 6. Licenza
Il progetto e MIT/Apache: lascia nel repository `LICENSE-MIT`, `LICENSE-APACHE`,
`NOTICE` e `ATTRIBUTION.md`.

## Nota
Se la pagina si apre ma resta su "Loading CADCraft..." serve un browser con WebGPU o WebGL2
(Chrome o Edge recenti, anche su Chromebook). Dimmelo e controlliamo la console del browser.
