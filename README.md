# Disegno CAD

Programma di disegno tecnico 2D che funziona nel browser, pensato per gli alunni della
**Scuola Secondaria di I grado R. Montecuccoli, Pavullo nel Frignano**.

Si apre da una pagina web, senza installare nulla. L'alunno accede con l'account Google della scuola,
disegna, e **consegna** il lavoro: il disegno viene salvato nel suo Google Drive, nella cartella
*Disegni*, e può copiare il link da incollare nel compito di Google Classroom oppure inviarlo per
email a un docente della scuola.

## Cosa c'è

- **Modalità Semplice**: pochi strumenti (linea, polilinea, cerchio, rettangolo, arco, poligono,
  tratteggio, sposta, copia, ruota, specchio, taglia, quote di base), griglia a quadretti da 0,5,
  aggancio alla griglia e ai punti (estremi, punto medio, centro, intersezione).
- **Modalità Normale**: l'interfaccia completa.
- **Accesso obbligatorio** con l'account Google della scuola prima di disegnare, e avviso se si
  chiude la pagina con un disegno non salvato.
- Solo 2D: non c'è la modellazione 3D.

## Crediti e licenza

Disegno CAD è una **versione modificata di [CADCraft](https://getartcraft.com/apps/cadcraft)**, il
programma di disegno libero scritto in Rust dal team ArtCraft e dai suoi collaboratori. Le modifiche
sono dei docenti della scuola. Non è un prodotto del team ArtCraft.

Il codice è distribuito con la stessa licenza di CADCraft: **MIT oppure Apache-2.0**, a scelta
(`LICENSE-MIT`, `LICENSE-APACHE`). Gli avvisi di copyright originali sono conservati in `NOTICE`;
l'elenco dei materiali e delle loro licenze è in `ATTRIBUTION.md`. I marchi ArtCraft sono stati
rimossi da questa versione, come richiede `docs/brand/LICENSE-brand.txt`.

## Per chi mantiene il sito

Il sito si compila e si pubblica da solo su GitHub Pages a ogni modifica del ramo `main`
(`.github/workflows/pages.yml`). Le istruzioni sono in `ISTRUZIONI-GITHUB-PAGES.md`. L'accesso a Google
usa un ID client OAuth pubblico (in `apps/cadcraft-web/src/drive.rs`), valido solo per gli indirizzi
autorizzati nella console Google Cloud della scuola. Con `?nodrive` nell'indirizzo il sito funziona
senza accesso e scarica i file sul computer (utile per prove).
