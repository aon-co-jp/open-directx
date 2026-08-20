# Filosofia progettuale e politica di sviluppo e regole dell'ambiente di sviluppo (open-directx) — condensato

> **Nota**: questa è una traduzione condensata dello stato attuale. Il log
> HANDOFF storico completo (decine di voci dal 2026-07-25) rimane
> disponibile solo in giapponese in CLAUDE.md — consultare quello per il
> dettaglio sessione per sessione.

Unità di lavoro: `F:\runo`. Questa sezione segue la prassi di trattare il
`CLAUDE.md` di [`open-raid-z`](https://github.com/aon-co-jp/open-raid-z)
come fonte di verità e di copiarlo in ogni progetto. Repository GitHub:
[aon-co-jp/open-directx](https://github.com/aon-co-jp/open-directx).

**Data di inizio sviluppo: 2026-07-25** (il repository GitHub vuoto in sé
era stato creato il 2026-07-01).

## Attività in sospeso (aggiunta 2026-08-06, non ancora iniziata)

Esiste un piano, su istruzione dell'utente, di indagare l'integrazione
della tecnologia Simulated Bifurcation Machine di Toshiba (pseudo-computer
quantistico) e delle tecniche di DeepSeek (MLA, DeepSeekMoE, addestramento
a precisione mista FP8, studiate da paper/blog di implementazione, non
solo da notizie) in 8 repository incluso `open-directx`. Non è ancora
stato identificato alcun obiettivo di ottimizzazione concreto per questo
repository — l'indagine è rinviata a una sessione futura.

## Ruolo di questo progetto

Un livello di compatibilità DirectX (D3D9/10/11/12) multipiattaforma che
mira a far girare app/giochi esistenti scritti per l'API DirectX
esclusiva di Windows su Linux, Android, e in futuro macOS e la famiglia
PlayStation.

Il 2026-07-25, per esplicita scelta dell'utente, il progetto si è
impegnato a perseguire un **vero livello di compatibilità in direzione
inversa** (far girare binari/shader DirectX Windows senza modifiche su
altri sistemi operativi) piuttosto che l'alternativa precedente di
"esporre un'API simile a DirectX sopra Vulkan come base comune".

## Correzione del posizionamento tecnico (2026-07-25, importante)

Una valutazione precedente (2026-07-23) giudicava che DXVK/vkd3d-proton/
MoltenVK traducessero solo in una direzione (DirectX → Vulkan/Metal) senza
un vero esempio della direzione inversa, e fossero quindi tecnicamente
poco adatti. Questo era **una confusione tra due assi diversi** ed è stato
corretto:

- DXVK/vkd3d-proton (la tecnologia dietro Proton di Valve / compatibilità
  DirectX di giochi Linux Steam) e CrossOver/Whisky basati su MoltenVK
  (macOS) sono esempi reali e funzionanti esattamente della compatibilità
  inversa che l'utente desidera: far girare binari/giochi DirectX
  (un'API esclusiva Windows) esistenti senza modifiche su Linux/macOS.
- La **direzione della traduzione** ("le chiamate DirectX tradotte in
  chiamate Vulkan") e la **direzione dell'esperienza utente finale** ("se
  un'app DirectX pensata per Windows gira effettivamente su Linux/macOS")
  sono assi separati — il fatto che il primo sia orientato a Vulkan non
  impedisce al secondo di raggiungere correttamente "DirectX in esecuzione
  su un altro sistema operativo".
- Pertanto, usare Vulkan come backend di esecuzione interno non è in
  conflitto con il perseguire un vero livello di compatibilità in
  direzione inversa — DXVK ecc. sono esattamente questo precedente.
  Questo progetto adotta lo stesso approccio: intercettare le chiamate
  API D3D + tradurre a runtime il bytecode shader DXBC/DXIL → eseguire
  via Vulkan.

## Scope e roadmap onesta

**Fase 0 (attuale, fase di design/ricerca)**:
- Ricerca della struttura di DXBC/DXIL (formati di bytecode shader
  DirectX).
- Studio dell'architettura di implementazioni OSS esistenti (DXVK,
  vkd3d-proton, [dxil-spirv](https://github.com/HansKristian-Work/dxil-spirv)
  — il vero strumento di conversione DXIL→SPIR-V usato da vkd3d-proton —
  SPIRV-Cross, naga) per evitare di reinventare la ruota e prendere in
  prestito decisioni progettuali collaudate.
- Ritagliare uno scope MVP realistico: **una pipeline grafica completa
  (rasterizzatore, texture sampler, blend state, ecc.) è fuori scope per
  ora.** Si inizia con un vertical slice che copre solo il dispatch del
  D3D11 Compute Shader (DirectCompute) — un semplice compute shader
  effettivamente tradotto da DXBC/DXIL a SPIR-V, inviato via
  `opencuda-vulkan` di `open-cuda`, e verificato corrispondere
  numericamente a un'implementazione di riferimento su CPU. Il lavoro
  sulla pipeline grafica inizia come fase successiva solo dopo che questo
  vertical slice sarà dimostrato.

**Fase 1 in avanti (non ancora iniziata)**:
1. Vertical slice D3D11 Compute Shader (traduzione DXBC/DXIL→SPIR-V +
   dispatch Vulkan).
2. Pipeline grafica minima D3D11 (vertex/pixel shader + rasterizzazione
   di base).
3. Supporto D3D12 (command list, descriptor heap, root signature).
4. Supporto Android (Vulkan stesso è nativo su Android, quindi la
   maggior parte degli asset della versione Linux dovrebbe essere
   riutilizzabile — ma probabilmente servirà un livello di emulazione
   Win32/COM equivalente a Wine; in tal caso si valuterà la
   collaborazione/il riutilizzo del progetto Wine stesso).
5. Supporto macOS / iPhone / iPad (via MoltenVK, stesso approccio di
   CrossOver/Whisky; iPhone/iPad richiede l'Apple Developer Program per
   la distribuzione ufficiale — l'esecuzione nativa su hardware non
   ufficiale è impossibile, lo stesso vincolo identificato nella ricerca
   di `dream-os`).
6. Vari sistemi della famiglia UNIX (BSD, ecc.) — probabilmente in grado
   di riutilizzare gran parte del percorso Linux a seconda del supporto
   Vulkan (non ancora indagato).

**Sul supporto PlayStation 4/5/6/7 (divulgazione onesta, al 2026-07-25)**:
incluso nella visione originale dell'utente, ma **esistono preoccupazioni
legali/sui termini di servizio indipendenti dalla difficoltà tecnica** —
gli SDK di sviluppo PlayStation sono privati e coperti da NDA, e il
reverse engineering non ufficiale rischia di violare vari termini di
servizio e leggi (es. DMCA). Questo progetto mantiene il supporto PS4-7
in roadmap solo come **"ambizione futura"** e non lo include attualmente
nello scope di design/implementazione. Avviarlo richiederebbe una
valutazione separata del rischio legale e una nuova conferma dell'utente.

**Sul supporto Nintendo Switch 2/3 (aggiunto 2026-08-17, divulgazione
onesta)**: registrato analogamente solo come "ambizione futura". Switch 2
richiede hardware di sviluppo ufficiale/NDA di Nintendo (la stessa
preoccupazione legale di PS4-7). **Switch 3 non è stata ufficialmente
annunciata da Nintendo al 2026-08-17 — la sua inclusione qui è solo un
segnaposto per se/quando verrà annunciata, non basata su informazioni
reali sul prodotto** (esplicitato per evitare di esagerare).

## Progetti di base (su istruzione dell'utente, 2026-07-25)

- **[open-cuda](https://github.com/aon-co-jp/open-cuda)**: usa
  `opencuda-vulkan` (backend di esecuzione Vulkan Compute, verificato su
  hardware reale NVIDIA GT 730) come backend di esecuzione shader. Si
  prevede di riutilizzare l'astrazione `opencuda-core::GpuDevice`
  (alloc/memcpy/launch_kernel) così com'è, passando i kernel tradotti
  DXBC/DXIL→SPIR-V come `KernelSource::SpirV` (dettagli API esatti da
  confermare contro `opencuda-core`). Distinto da `opencuda-directx` (un
  backend D3D12 esclusivo Windows, Fase 1&2 già implementate) — quello
  esegue DirectX nativamente *su* Windows, la direzione opposta rispetto
  a questo progetto (che esegue DirectX *su altri sistemi operativi*).
- **[aruaru-llm](https://github.com/aon-co-jp/aruaru-llm)**: nessuna
  dipendenza tecnica diretta al momento (aruaru-llm è un servizio di
  inferenza LLM; questo progetto è un livello di compatibilità API
  grafica). L'esatta intenzione dietro il fatto che l'utente elenchi
  aruaru-llm come "base" non è confermata — possibilmente significa
  seguire lo stesso pattern condiviso di servizio "bunshin-no-jutsu"
  (clone/tenant) (es. applicare il pattern API di gestione in stile
  `TenantRegistry` a qualche superficie di gestione di questo progetto,
  come un server di cache di traduzione). Da aggiornare una volta
  identificati punti di integrazione concreti.

## Politica di sviluppo (riassunto valido per tutto l'ecosistema)

- Implementazione basata su Rust. Usa il crate `windows` (API Windows) e
  i binding Vulkan (`ash`, ecc., in linea con ciò che usa
  `opencuda-vulkan`).
- "Mai riportare un successo basandosi solo su type-check/build riusciti"
  — segnalare qualcosa come funzionante solo dopo aver effettivamente
  tradotto bytecode DXBC/DXIL reale, inviato via Vulkan reale, e
  confermato la corrispondenza numerica contro un'implementazione di
  riferimento su CPU su hardware reale (disciplina valida per tutto
  l'ecosistema).
- Le funzionalità non implementate/stub non devono mai segnalare
  falsamente "supportato" (seguendo il pattern `supports_dxil()` di
  `opencuda-directx`).
- Confermare con l'utente prima di qualsiasi decisione che coinvolga
  nuovi file, nuovi crate, nuovi repository, o giudizi su
  nomenclatura/posizionamento (lezione dal 2026-07-23, vedere il
  CLAUDE.md di `open-raid-z`).

## HANDOFF (solo le voci più recenti — vedere CLAUDE.md per il log completo)

- **2026-08-20: implementata e verificata su hardware reale una fetta
  verticale di compute shader per GEMM (2x2 fisso)**, primo passo verso
  l'accelerazione hardware per l'inferenza LLM. Nuova funzione
  `translate_gemm2x2_shader` in `directx-shader-translate`: uno shader
  HLSL GEMM 2x2x2 è stato compilato realmente con `fxc.exe`, il suo
  DXBC decodificato, tradotto in SPIR-V ed eseguito su Vulkan reale
  (NVIDIA GT 730) riutilizzando il contratto kernel di
  `opencuda-vulkan`. L'output della GPU corrisponde esattamente al
  riferimento CPU. Limite dichiarato: solo dimensione fissa 2x2 — GEMM
  generale a dimensione variabile (con cicli), operazioni di Attention
  e integrazione con `aruaru-llm` restano fuori ambito.
- **2026-08-19: valutata la fattibilità di un meccanismo di
  aggiornamento automatico (come il self-update via GitHub Releases di
  `open-english`) — rimandato.** L'unico binario `fn main` nel
  workspace è `directx-graphics-window` (demo stile Breakout per
  ispezione visiva), senza servizio persistente né endpoint
  `/healthz`; il pattern non è applicabile.
- **2026-08-19: analizzata l'obiezione secondo cui "l'assenza di un
  servizio persistente è un difetto" — conclusione: non lo è.** Il
  vero DirectX Microsoft viene distribuito come DLL runtime collegate
  dinamicamente dalle app, non come servizio in background
  indipendente; il design attuale (solo libreria + binario demo)
  corrisponde all'architettura del DirectX originale.
- **2026-08-20: registrato un promemoria di idea `open-cg-cad` (non
  iniziato).** Concetto di un futuro strumento di modellazione 3D con
  motion capture, modifiche di specifiche in linguaggio naturale
  guidate da chat IA e supporto DirectX/OpenGL/WebGL/WebGPU.
- **2026-08-20 (continuazione): registrato il concetto "immobiliare IA
  × edilizia IA"**, che combina `open-cg-cad` con `aruaru-llm` per
  generare automaticamente modelli 3D (case, edifici, ponti, gallerie,
  ecc.) da dati sul terreno; esteso poi a treni maglev e modelli CAD di
  semiconduttori (CPU/GPU/NPU), con un'onesta nota sulla difficoltà di
  coprire architettura, ingegneria civile, materiale rotabile e
  progettazione di semiconduttori in un unico sistema. Solo
  promemoria, non iniziato.

- **2026-08-08 (continuazione 12): implementato il caricamento di
  texture da file PNG reali, verificato su hardware reale Windows +
  Linux.** Il nuovo modulo `png_loader.rs` (usando il crate `png`, serie
  0.17.x) implementa `load_png_rgba8`, normalizzando immagini
  RGB/RGBA/scala di grigi/scala di grigi+alpha/palette a RGBA8
  (l'espansione della palette e la normalizzazione 16 bit→8 bit sono
  entrambe delegate alle trasformazioni proprie del crate `png`, non
  scritte a mano). È stato generato e committato un asset di test reale
  (`assets/sample_sprite.png`, una scacchiera 2x2 con un quadrante
  semi-trasparente). Verificato su hardware reale sia Windows (NVIDIA GT
  730) che Linux (WSL2 Ubuntu/Mesa llvmpipe): i quadranti opachi
  corrispondono esattamente, e il quadrante semi-trasparente produce
  esattamente il colore composito con alpha-blend previsto dalla
  formula standard "over". Intero workspace: `cargo build`/`clippy`
  puliti (0 warning); `cargo test --workspace --release` supera tutti i
  33 test su hardware reale + 56 unit test, nessuna regressione.
  Divulgazione onesta: i PNG interlacciati e i PNG a 16 bit per canale
  non sono effettivamente testati (solo affidati alla gestione
  automatica del crate `png`); `directx-graphics-window` (la demo con
  finestra reale) non chiama ancora questo loader (ancora una texture a
  colore solido 1x1).

- **2026-08-08 (continuazione 11): implementato il blending alpha
  (sprite semi-trasparenti), verificato su hardware reale Windows +
  Linux.** Blending alpha standard "over" abilitato nella costruzione
  della pipeline di `render_sprites_and_read_back`
  (`SRC_ALPHA`/`ONE_MINUS_SRC_ALPHA`/`ADD`). Poiché `src.a=1.0` è
  numericamente equivalente a `blend_enable=false`, tutti i test
  esistenti con sprite opachi sono stati confermati produrre risultati
  identici (cambiamento non distruttivo e additivo). Un nuovo test ha
  verificato il risultato della formula standard "over" su hardware
  reale, con corrispondenza esatta su entrambi i sistemi operativi.
  Divulgazione onesta: questo blending è abilitato solo nel percorso
  offscreen `render_sprites_and_read_back` — `directx-graphics-window`
  (la demo con finestra reale) non è stata ancora aggiornata di
  conseguenza; è supportato solo il blending "over" (nessuna modalità
  additiva/moltiplicativa); l'interazione tra depth-test e ordine di
  blend non è testata (questo passaggio non ha depth buffer, solo
  sprite 2D).

Per la cronologia completa sessione per sessione (incluso il traguardo
precedente della pipeline grafica D3D11, il vertical slice DXIL, il game
loop con finestra reale + swap-chain + input da tastiera, e i numerosi
incrementi di lunghezza della catena DXBC/DXIL con controllo dei limiti),
vedere [CLAUDE.md](CLAUDE.md) (giapponese, autorevole).
