# PORTING.md — cosa è riutilizzabile, da chi e come (condensato)

> **Nota**: questa è una traduzione condensata. La guida tecnica completa
> con dettagli di codice e insidie rimane disponibile solo nel
> [PORTING.md](PORTING.md) originale — consultarlo prima di adottare
> effettivamente un pattern.

Riepilogo dei pattern di implementazione riutilizzabili di questo
progetto, per chiunque voglia portarli in un altro progetto:

1. **`crates/directx-shader-translate`**: un parser di contenitore/chunk
   DXBC (bytecode shader D3D9/10/11, SM<=5.1) (`parse_dxbc`), più un
   traduttore DXBC(SM5.0)→SPIR-V con scope ristretto (`translate_shader`)
   esattamente per le forme di opcode effettivamente emesse da `fxc.exe`
   per gli shader propri di questo progetto: operazioni singole
   (add/mul/sub-via-add-negato/div), catene di operazioni binarie
   sequenziali di lunghezza arbitraria (basate su albero di espressione
   di registri, non hardcoded per lunghezza — verificato fino a 7
   termini con *zero* modifiche al codice di produzione necessarie per
   ogni nuovo conteggio di termini), e varianti con controllo dei limiti
   (`if (id.x < N)`) delle stesse, generando SPIR-V reale
   `OpSelectionMerge`/`OpBranchConditional`. Qualsiasi altra forma di
   opcode viene onestamente rifiutata tramite
   `SpirvGenError::UnsupportedShader` invece di essere tradotta in modo
   silenziosamente errato — questo **non** è un decoder SM5.0 generale.
2. **Supporto DXIL (SM6+, D3D12)** (`src/dxil.rs`): `parse_dxil_
   container` percorre il bitstream LLVM grezzo (tramite il crate
   generico `llvm-bitcode`) per risolvere la tabella dei tipi e
   decodificare le istruzioni delle funzioni; le funzioni della famiglia
   `resolve_vector_add_dxil_calls` disambiguano le chiamate intrinseche
   `dx.op.*` di DXIL (CreateHandle/ThreadId/BufferLoad/BufferStore)
   contro i numeri di opcode documentati nel DXIL.rst di Microsoft.
   `translate_dxil_..._to_spirv` condivide un unico emettitore SPIR-V
   (`emit_spirv_for_kernel`) con il backend DXBC, così entrambi i
   formati di contenitore producono SPIR-V attraverso un unico percorso
   di codice. La dimensione del workgroup (`numthreads`) viene
   genuinamente estratta dal `METADATA_BLOCK` di DXIL
   (`dx.entryPoints` → `ShaderProperties`), non hardcoded. La
   generalizzazione della catena con controllo dei limiti (vedere sopra)
   è stata verificata end-to-end sia su DXBC che su DXIL per catene fino
   a 7 termini.
3. **Pipeline grafica D3D11**: `translate_vertex_shader`/
   `translate_pixel_shader` emettono SPIR-V grafico reale per una coppia
   fissa di VS/PS passthrough, validata in due modi indipendenti
   (rianalisi con rspirv + il vero `spirv-val.exe` dell'SDK Vulkan,
   entrambi codice di uscita 0). Il nuovo crate
   `crates/directx-graphics-vulkan` aggiunge `ash` come dipendenza
   **diretta** propria di questo progetto (non stratificata su
   `opencuda-vulkan`, che è stato confermato tramite audit del sorgente
   essere solo compute-dispatch) e implementa un vero render pass,
   framebuffer e `VkGraphicsPipelineCreateInfo`. Disegna e rilegge
   triangoli e scene con texture/sprite multipli con blending alpha
   standard "over" e caricamento di texture da file PNG reali (crate
   `png`), tutto verificato su hardware reale NVIDIA GT 730 (Windows)
   *e* WSL2 Ubuntu/Mesa llvmpipe (Linux), con risultati corrispondenti
   su entrambi i sistemi operativi.
4. **`crates/directx-graphics-window`**: una finestra reale + swapchain
   Vulkan reale + input da tastiera reale (winit + ash-window), con una
   propria istanza/dispositivo Vulkan indipendente separata dal
   contesto offscreen di `directx-graphics-vulkan` — mantenere i due
   sincronizzati se si usano entrambi (es. il blending alpha è
   attualmente abilitato solo nel percorso offscreen). Ha prodotto una
   demo interattiva in stile Breakout, confermata funzionante dagli
   occhi dell'utente stesso (movimento della paletta + rimbalzo della
   palla).
5. **Convenzione di dipendenza per percorso**: questo crate dipende da
   `opencuda-core`/`opencuda-vulkan` di `open-cuda` tramite dipendenze di
   percorso relativo, un livello di directory più in profondità rispetto
   alla convenzione di repository fratelli sotto `F:\runo` usata altrove
   in questo ecosistema (es. `aruaru-llm`, `aruaru-db`) — queste sono
   **solo dev-dependency**, la libreria pubblicata in sé non ha alcuna
   dipendenza da `open-cuda`.
6. **Nota di scope sull'anti-cheat a livello kernel**: per chiunque
   porti questo progetto verso un obiettivo di "far girare giochi
   Windows reali su Linux" — l'anti-cheat a livello kernel (Riot
   Vanguard, BattlEye in modalità kernel, ecc.) blocca per progettazione
   ambienti in stile Linux/Proton, indipendentemente da quanto completo
   diventi questo livello di traduzione shader. Questo non è un difetto
   da correggere; i titoli che usano tale anti-cheat sono fuori portata
   per questo progetto indipendentemente dalla completezza della
   traduzione.

## Cosa NON è ancora riutilizzabile (lacune oneste)

- Nessun decoder di istruzioni SM5.0 o SM6.0 generale — sono gestite
  solo le classi specifiche di opcode/forma descritte sopra; qualsiasi
  altra cosa viene onestamente rifiutata, non tradotta in modo errato.
- I livelli superiori di D3D12 (command list, descriptor heap, root
  signature) sono interamente non implementati.
- Nessun depth buffer, nessun controllo di interpolazione tra vertici di
  colori diversi oltre al caso del triangolo a gradiente, e l'hardware
  GPU AMD/Intel e l'esecuzione nativa desktop macOS/Linux rimangono non
  verificati (esistono percorsi di codice per il rilevamento del PCI
  vendor-ID AMD/Intel ma non sono mai stati eseguiti contro hardware
  AMD/Intel reale in questo ambiente di sviluppo).

---

Per il dettaglio tecnico completo (sequenze di opcode esatte, tracce a
livello di byte del bitstream DXIL, snippet di codice, ed esempi completi
di Cargo.toml per le dipendenze di percorso), vedere il
[PORTING.md](PORTING.md) originale (inglese, autorevole).
