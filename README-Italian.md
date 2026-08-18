# open-directx

> 📌 **Aggiornamento recente (2026-08-08, prototipo di rendering di sprite 2D)**:
> In seguito alla proposta dell'utente di sviluppare prototipi di
> gioco/mining/LLM per la GT 730 tramite open-directx su dream-os/Linux, si è
> iniziato in modo mirato con un prototipo di rendering di sprite 2D: prima
> il supporto al campionamento delle texture, poi il supporto a più
> sprite/sprite sheet, un game loop (aggiornamento della posizione + fisica
> di rimbalzo), una **finestra reale + swapchain Vulkan reale + input da
> tastiera reale** (nuovo crate `directx-graphics-window` — l'utente
> stesso lo ha eseguito confermando "la paletta si è mossa e ha respinto la
> palla"), il blending alpha e il caricamento di texture da file PNG reali.
> Tutto verificato su hardware Windows reale (NVIDIA GT 730), in parte anche
> su hardware Linux reale (WSL2 Ubuntu). Prossimi candidati: più sprite in
> movimento con rilevamento delle collisioni, supporto al ridimensionamento
> della finestra. Per i dettagli vedere [CLAUDE.md](CLAUDE.md).

> 📌 Attività in sospeso (2026-08-06): esiste un'idea di integrazione delle
> tecnologie Toshiba SBM e DeepSeek (per 8 repository incluso dream-os).
> Per i dettagli vedere [CLAUDE.md](CLAUDE.md).

> 📌 **Aggiornamento recente (2026-08-08)**: risolta l'asimmetria per cui la
> catena a 7 termini con controllo dei limiti esisteva solo lato DXBC e non
> lato DXIL — è stato aggiunto e compilato realmente con `dxc.exe` un nuovo
> `vector_add_mul_div_sub_add_mul_div_chain7_bounded_dxil.hlsl`, verificato su
> hardware reale NVIDIA GT730 con corrispondenza numerica rispetto
> all'implementazione di riferimento su CPU e conferma del comportamento del
> controllo dei limiti (l'intero workspace: 50 unit test + 22 test su
> hardware reale, tutti verdi, zero warning). Per i dettagli vedere
> [CLAUDE.md](CLAUDE.md).

> 📌 **Aggiornamento recente (2026-08-07)**: estesa a 6 termini la catena
> DXBC/DXIL con controllo dei limiti, verificata su hardware reale NVIDIA
> GT730. È stata valutata un'integrazione più profonda con dream-os/
> open-cuda/aruaru-llm (trapianto SBM/DeepSeek), ma si è deciso di non
> estendere per congettura la logica di generazione delle catene DXBC/DXIL
> senza una comprensione approfondita del dominio — nessuna modifica al
> codice in quell'area, i risultati sono stati onestamente registrati in
> [CLAUDE.md](CLAUDE.md).

> **Aggiornato 2026-07-25**: il titolo del file di policy di sviluppo
> (`CLAUDE.md`) è stato rinominato da "Development Policy & Dev Environment
> Rules" a "Design Philosophy & Development Policy & Dev Environment Rules",
> per separare più chiaramente la filosofia progettuale del progetto (i
> valori), la politica di sviluppo (il modo di lavorare) e le regole
> dell'ambiente di sviluppo (convenzioni operative concrete). Vedere
> `CLAUDE.md` per i dettagli.


Un livello di compatibilità DirectX (D3D9/10/11/12) multipiattaforma — nello
spirito di DXVK / vkd3d-proton — che mira a far girare senza modifiche
applicazioni DirectX Windows su Linux (e in futuro Android/macOS)
traducendo il bytecode degli shader DXBC/DXIL in SPIR-V e inviandolo
tramite un backend Vulkan compute già esistente
([open-cuda](https://github.com/aon-co-jp/open-cuda)'s `opencuda-vulkan`).

Vedere [`CLAUDE.md`](CLAUDE.md) per la motivazione progettuale completa, lo
scope/roadmap onesto e il log HANDOFF delle sessioni — questo README si
limita a riassumere lo stato attuale, verificato.

### Matrice di supporto per piattaforme e vendor (aggiunta 2026-07-27, divulgazione onesta)

DirectX in sé è un'API esclusiva per Windows/Xbox — "multipiattaforma" qui
significa che il bytecode DXBC/DXIL viene tradotto in SPIR-V e inviato
tramite Vulkan, che è ciò che effettivamente raggiunge le piattaforme
non-Windows. Nel codice proprio di questo repository non esiste alcun
`cfg(windows)` o altro gate specifico per piattaforma (il parser DXBC, la
generazione di codice SPIR-V e `directx-graphics-vulkan` sono tutti Rust +
`ash` puri e indipendenti dalla piattaforma), quindi la portabilità di
build/test segue la portata dello stesso Vulkan:

| Piattaforma | Percorso | Stato |
|---|---|---|
| Windows | Vulkan nativo | **Verificato su hardware reale** (la macchina di sviluppo di questo repository, NVIDIA GeForce GT 730) |
| Linux | Vulkan nativo | Dovrebbe compilare/funzionare senza modifiche (non esiste codice specifico per Windows che lo blocchi) — **non ancora testato su una macchina Linux reale in questo ambiente** |
| Android | Vulkan nativo | `open-cuda` ha verificato che la cross-compilation `aarch64-linux-android` ha successo (secondo il suo CLAUDE.md); l'esecuzione su dispositivo reale (`vkCreateInstance` su un telefono reale) è ancora in sospeso |
| macOS | Vulkan via [MoltenVK](https://github.com/KhronosGroup/MoltenVK) (traduce verso Metal) | Non ancora tentato — MoltenVK è un livello di traduzione, non Vulkan nativo, quindi è una garanzia più debole rispetto a Linux/Android |
| iOS / iPadOS (aggiunto 2026-08-17) | Vulkan via MoltenVK (traduce verso Metal) | Non ancora tentato. **Vale la stessa avvertenza MoltenVK di macOS** — Vulkan non gira nativamente su iOS/iPadOS, solo tramite questo livello di traduzione, quindi la parità con il percorso Windows/Vulkan-nativo non è garantita finché non si prova effettivamente su un dispositivo. Richiede inoltre l'Apple Developer Program per la distribuzione ufficiale. |
| Varie UNIX/BSD (aggiunto 2026-08-17) | Vulkan nativo, probabilmente | Non ricercato — il supporto Vulkan varia per distribuzione/driver; ci si aspetta di poter riutilizzare gran parte del percorso Linux una volta indagato |
| Sony PlayStation 4/5/6/7 | n/d | Esplicitamente fuori scope per ora — vedere la nota "obiettivi della famiglia PlayStation" più sotto e `CLAUDE.md` |
| Nintendo Switch 2 / 3 (aggiunto 2026-08-17) | n/d | Stesso stato "ambizione futura, rinviata in attesa di SDK/NDA ufficiale" di PlayStation. **Switch 3 non è stata ufficialmente annunciata da Nintendo al 2026-08-17 — la sua inclusione qui è solo un segnaposto per se/quando verrà annunciata, non basata su informazioni reali sul prodotto.** |

Copertura dei vendor GPU (corrispondenza per PCI vendor ID, coerente tra
questo repository e `open-cuda`: NVIDIA `0x10DE`, AMD `0x1002`/`0x1022`,
Intel `0x8086`):

| Vendor | Stato |
|---|---|
| NVIDIA | **Verificato su hardware reale** (GeForce GT 730) |
| AMD | Il codice di corrispondenza vendor-ID esiste e compila, ma **non è mai stato eseguito contro hardware AMD reale** in questo ambiente — considerarlo non verificato |
| Intel | Come AMD: il codice esiste, **mai verificato su hardware GPU Intel reale** |

Non serve alcuna correzione per rendere questi tre vendor ID *rilevabili*
— il codice è già corretto e identico tra `open-directx`/`opencuda-vulkan`/
`opencuda-directx`. Ciò che manca è hardware AMD/Intel reale per esercitare
effettivamente quel percorso di codice, che questo ambiente di sviluppo non
possiede.

## Stato attuale (2026-07-27, ultimo: interpolazione del gradiente, diagnostica vendor GPU, catena sub/div)

Tre incrementi si sono aggiunti sopra alla pipeline grafica minima D3D11 e
al lavoro sulla classe di catene DXBC sotto, tutti verificati sulla GT 730
NVIDIA reale di questa macchina: (1) `render_gradient_triangle_and_read_back`
— la pipeline grafica ora può assegnare un colore distinto per vertice (non
più solo il caso degenere a colore uniforme), verificato tramite un
controllo di invariante di partizione dell'unità sui pixel letti
dall'hardware reale. (2) `enumerate_graphics_devices()` — colma un divario
di parità diagnostica in cui il percorso Compute di `open-cuda` aveva il
rilevamento del vendor-ID mentre il percorso Graphics qui non ne aveva
nessuno; autonomo, senza nuova dipendenza da `opencuda-vulkan`. (3)
`decode_chain_shape` ora supporta `sub`/`div` (in precedenza esplicitamente
rifiutati come non verificabili) — un nuovo shader
(`vector_sub_div_chain.hlsl`) è stato effettivamente compilato con `fxc.exe`
e il suo dump SHEX usato per confermare l'esatto ordine degli operandi, poi
verificato end-to-end contro un riferimento CPU su hardware reale. Vedere
l'HANDOFF di `CLAUDE.md` (voci del 2026-07-27) per il resoconto completo.

## Stato attuale (2026-07-25, ultimo: vertical slice DXIL completo su hardware reale)

Il vertical slice compute shader D3D12/DXIL raggiunge ora piena parità con
quello D3D11/DXBC: `vector_add.dxil` (output reale di `dxc.exe -T cs_6_0`)
viene decodificato end-to-end (contenitore -> bitstream LLVM -> tabella dei
tipi -> istruzioni -> tutti i 7 record `Call` disambiguati al loro
significato reale `dx.op.*`) e tradotto in SPIR-V reale
(`directx_shader_translate::translate_dxil_vector_add_to_spirv`), che
`tests/vector_add_dxil_real_vulkan.rs` invia sulla GT 730 NVIDIA reale di
questa macchina e verifica numericamente corrispondere al riferimento CPU
`a[i]+b[i]`. Rimane comunque una singola forma di shader nota, non un
decoder SM6.0 generale — vedere "Non implementato (scope onesto)" più
sotto per il confine preciso. La dimensione del workgroup SPIR-V viene ora
genuinamente estratta dal `METADATA_BLOCK` di DXIL
(`dx.entryPoints` -> `ShaderProperties` -> `NumThreads`), non più
hardcoded — vedere la voce HANDOFF "continued 9" del 2026-07-25 in
`CLAUDE.md` per il resoconto completo, e "continued 7" per il traguardo
originale del vertical slice che questo ha colmato.

## Stato attuale (2026-07-25, continuazione: parsing DXIL a livello di bitstream + parsing DXBC di VS/PS D3D11)

Due nuovi pezzi di lavoro si sono aggiunti sopra al vertical slice compute
shader Fase 1 più sotto:

- **DXIL (D3D12/SM6+) — byte reali analizzati, solo a livello di
  contenitore/bitstream.** `crates/directx-shader-translate/src/dxil.rs`
  (`parse_dxil_container`) analizza un contenitore DXBC reale compilato con
  `dxc.exe -T cs_6_0` (`shaders/vector_add_dxil.hlsl` ->
  `shaders/vector_add.dxil`, prodotto da
  `tools/compile-dxbc-shaders.ps1`): estrae `DxilProgramHeader`/
  `DxilBitcodeHeader` (tipo di shader, SM6.0, versione DXIL) del chunk
  `DXIL` tramite il crate `dxbc` esistente, poi passa il payload bitcode
  LLVM grezzo al crate `llvm-bitcode` (nuova dipendenza aggiunta, un lettore
  generico di bitstream LLVM senza conoscenza specifica di DXIL) per
  decodificare effettivamente l'albero di blocchi/record. Confermato contro
  i byte reali: il magic wrapper LLVM `BC\xC0\xDE`, un singolo
  `MODULE_BLOCK` (id 8) di livello superiore, e i sub-blocchi LLVM standard
  al suo interno — `TYPE_BLOCK_ID_NEW`(17), `PARAMATTR_GROUP_BLOCK`(10),
  `PARAMATTR_BLOCK`(9), `CONSTANTS_BLOCK`(11), `FUNCTION_BLOCK`(12, x5 — uno
  per basic block di `main`), `VALUE_SYMTAB_BLOCK`(14), `METADATA_BLOCK`(15,
  x2). **Aggiornamento (2026-07-25, continuazione, traccia D3D12)**: da
  allora sono stati aggiunti la risoluzione della tabella dei tipi e la
  decodifica approssimativa delle istruzioni
  (`resolve_type_table`/`decode_function_instructions` nello stesso file),
  applicando le tabelle di record `TYPE_BLOCK`/`FUNC_CODE` documentate da
  LLVM ai byte reali di `vector_add.dxil` — confermata una tabella di tipi
  a 22 voci che include `Float` e
  `StructNamed{"class.RWStructuredBuffer<float>"}`, e una sequenza di
  istruzioni reale (`DeclareBlocks -> Call*5 -> ExtractValue -> Call ->
  ExtractValue -> BinOp -> Call -> Ret`). **Aggiornamento (2026-07-25,
  continuazione 6)**: tutti e 7 i record `Call` sono ora disambiguati.
  `resolve_vector_add_dxil_calls` risolve i nomi delle funzioni del
  `VALUE_SYMTAB_BLOCK` (trovati tramite `Record::take_payload()`, non
  `fields()` — un vero limite nella comprensione del crate della voce
  precedente) e decodifica manualmente la codifica degli operandi a valore
  relativo di LLVM (verificata a mano contro i byte reali), ottenendo
  `[CreateHandle{range_id:2}, CreateHandle{range_id:1},
  CreateHandle{range_id:0}, ThreadId, BufferLoad{handle_range_id:0},
  BufferLoad{handle_range_id:1}, BufferStore{handle_range_id:2}]`. I numeri
  degli opcode DXIL (`CreateHandle`=57, `BufferLoad`=68, `BufferStore`=69,
  `ThreadId`=93) sono stati confermati tramite ricerca web contro il
  `DirectXShaderCompiler/docs/DXIL.rst` di Microsoft, non assunti a memoria,
  e corrispondono esattamente alle costanti reali decodificate. **Manca
  ancora la traduzione DXIL-to-SPIR-V** — è il prossimo incremento. Vedere
  "Non implementato" più sotto.
- **Pipeline grafica D3D11 — generazione SPIR-V reale per VS/PS raggiunta e
  validata, ancora senza rasterizzatore/draw.** `shaders/triangle_vs.hlsl`/
  `shaders/triangle_ps.hlsl` (coppia minima di vertex+pixel shader
  passthrough, `POSITION`/`COLOR` in ingresso, `SV_POSITION`/`SV_TARGET` in
  uscita) compilati con `fxc.exe /T vs_5_0`/`/T ps_5_0` reali. `parse_dxbc`
  analizza entrambi senza modifiche. `spirv_gen::translate_vertex_shader`/
  `translate_pixel_shader` (nuovo) decodificano la sequenza reale e fissa
  di opcode SHEX (`dcl_input`x2/`dcl_output_siv`/`dcl_output`/`mov`x3/`ret`
  per VS; `dcl_input_ps`(linear)/`dcl_output`/`mov`/`ret` per PS) ed emettono
  SPIR-V grafico reale: `OpEntryPoint Vertex`/`Fragment` (non `GLCompute`),
  variabili di storage class `Input`/`Output` con decorazioni `Location`,
  `BuiltIn Position` sull'uscita `SV_POSITION` del vertex shader, e
  `OpExecutionMode ... OriginUpperLeft` sul fragment shader. Validato in due
  modi: (1) il loader di `rspirv` stesso rianalizza i byte emessi senza
  errori, (2) il vero `spirv-val.exe` dell'SDK Vulkan
  (`C:\VulkanSDK\1.4.350.0\Bin\spirv-val.exe`) è stato eseguito contro
  entrambi i moduli emessi e ha restituito codice di uscita 0 senza
  diagnostiche per entrambi. `translate_shader`/`translate_chain_shader`
  (solo compute) rifiutano ancora correttamente entrambi gli shader.
  **Non esiste alcun rasterizzatore, framebuffer o comando di draw Vulkan
  reale** — è stato confermato (leggendone il codice sorgente reale) che
  `opencuda-vulkan` non ha alcun codice
  `VkGraphicsPipelineCreateInfo`/render-pass/framebuffer, solo compute
  dispatch, quindi un pixel effettivamente renderizzato è fuori scope per
  questo passaggio. Vedere l'HANDOFF di `CLAUDE.md` per il confine onesto
  del traguardo.

## Stato attuale (2026-07-26, traguardo raggiunto: pipeline grafica minima D3D11)

Il nuovo crate `crates/directx-graphics-vulkan` aggiunge `ash` come
dipendenza **diretta** di questo workspace (non stratificata sopra
`opencuda-vulkan`, che è stato confermato tramite audit del sorgente essere
solo compute dispatch). Implementa un vero render pass, framebuffer e
`VkGraphicsPipelineCreateInfo`, riutilizzando lo SPIR-V già generato e già
passato da `spirv-val` da `translate_vertex_shader`/`translate_pixel_shader`
sopra (nessuna traduzione shader viene reimplementata).
`render_uniform_triangle_and_read_back` disegna un "triangolo grande" a
tutto viewport con un singolo colore vertice uniforme, rilegge l'immagine
renderizzata tramite un buffer di staging visibile all'host, e il test su
hardware reale
(`crates/directx-graphics-vulkan/tests/triangle_real_vulkan.rs`) verifica
che ogni pixel riletto corrisponda al colore vertice passthrough sulla
GT 730 NVIDIA reale presente su questa macchina (`cargo test -p
directx-graphics-vulkan --test triangle_real_vulkan -- --nocapture`: 1
passato). Lo scope è intenzionalmente ristretto: una coppia di shader
fissa, un solo comando di draw, nessun depth buffer/texture/swapchain/
controllo di interpolazione multi-triangolo. Vedere l'HANDOFF di
`CLAUDE.md` (continuazione 2026-07-26) per la divulgazione onesta completa.

## Stato attuale (2026-07-25, vertical slice Fase 1 generalizzato a 3 shader noti)

`crates/directx-shader-translate` esegue ora il vertical slice completo per
**tre shader specifici e noti** (`vector_add.hlsl`, `vector_mul.hlsl`,
`vector_sub_bounded.hlsl`): parsing DXBC -> decodifica di un sottoinsieme
ristretto di opcode SM5.0 -> generazione di codice SPIR-V (via `rspirv`) ->
dispatch Vulkan reale (`opencuda-vulkan` di `open-cuda`) -> corrispondenza
numerica con implementazione di riferimento CPU, verificato sulla GT 730
NVIDIA reale di questa macchina. **Non è ancora un decoder SM5.0-to-SPIR-V
generale** — vedere "Non implementato" più sotto.

- `parse_dxbc` (Fase 0): introspezione del contenitore/chunk DXBC
  (presenza di RDEF/ISGN/OSGN/SHEX), invariato rispetto al front-end
  originale.
- `spirv_gen::translate_shader` (Fase 1, generalizzato 2026-07-25):
  riconosce 3 forme di opcode effettivamente emesse da `fxc.exe`, che
  condividono tutte uno scheletro comune (`dcl_globalFlags` -> opzionale
  `dcl_constantbuffer` -> 3x `dcl_uav_structured` -> `dcl_input` ->
  `dcl_temps` -> `dcl_thread_group` -> opzionale `ult`+`if` -> 2x
  `ld_structured` -> `add`/`mul` -> `store_structured` -> opzionale
  `endif` -> `ret`):
  - `vector_add.hlsl`: `add`, senza controllo dei limiti.
  - `vector_mul.hlsl`: `mul` invece di `add`.
  - `vector_sub_bounded.hlsl`: `add` con un flag `negate` sul primo
    operando sorgente (confermato dumpando l'output reale di `fxc.exe` —
    `fxc` ottimizza `a - b` in `add dest, -b, a` invece di emettere un
    opcode `sub` dedicato), più un vero controllo dei limiti
    `if (id.x < N)` (`ult` contro un constant buffer + `if`/`endif`), che
    lo SPIR-V emesso implementa con un vero `OpSelectionMerge`/
    `OpBranchConditional`, usando la push-constant `n` per il confronto.
  Qualsiasi altro opcode/forma viene rifiutato tramite
  `SpirvGenError::UnsupportedShader` invece di essere tradotto in modo
  silenziosamente errato. I bind point UAV, la dimensione del thread group,
  l'operatore e la presenza del controllo dei limiti sono tutti estratti
  dal DXBC realmente analizzato, non hardcoded. `translate_vector_add_shader`
  viene mantenuto come sottile alias retrocompatibile per `translate_shader`.
- `tests/vector_add_real_vulkan.rs`, `tests/vector_mul_real_vulkan.rs`,
  `tests/vector_sub_bounded_real_vulkan.rs`: ciascuno invia il proprio
  SPIR-V tradotto tramite il vero `opencuda-vulkan::VulkanDevice` di
  `open-cuda` (`ash`, feature `real-vulkan`) e controlla l'output della GPU
  contro un riferimento CPU per 256 elementi (epsilon 1e-3/1e-2). Il test
  del controllo dei limiti dispatcha inoltre 320 thread con un conteggio
  logico di elementi di 256 e verifica che gli elementi 256..320 non
  vengano mai scritti (rimangano a un valore sentinella), dimostrando che
  il branch `if (id.x < N)` nello SPIR-V generato effettivamente controlla
  l'esecuzione anziché limitarsi a compilare.
- `examples/dump_shex.rs`: un piccolo strumento standalone
  (`cargo run -p directx-shader-translate --example dump_shex --
  <path.dxbc>`) usato durante questa sessione per ispezionare i flussi di
  opcode SHEX reali prima di scrivere il supporto del decoder per essi;
  mantenuto per il futuro lavoro di generalizzazione opcode per opcode.

**Da quando è stato scritto il titolo di questa sezione**, è stato
aggiunto un 4° shader a operazione singola (`vector_div.hlsl`, `div`
semplice) a `translate_shader` seguendo esattamente lo stesso schema, e —
più recentemente — una classe di pattern genuinamente diversa,
`spirv_gen::translate_chain_shader`, è stata aggiunta accanto ad esso (non
sostituendolo): decodifica un vero albero di espressione di registri di
operazioni binarie sequenziali (add/mul, senza controllo di flusso) invece
di una singola operazione fissa, verificato contro uno shader
appositamente compilato il cui SHEX reale si è rivelato riutilizzare i
componenti di un singolo registro temporaneo tramite il CSE di fxc invece
di dichiarare temp aggiuntivi. Vedere la voce HANDOFF "continued 9" del
2026-07-25 in `CLAUDE.md` per il resoconto completo e attuale (questa
sezione è lasciata come originariamente scritta per accuratezza storica
sullo stato di metà giornata del 2026-07-25).

## Build & test

```powershell
cargo build --workspace
cargo test --workspace -- --nocapture
```

### Vederlo effettivamente disegnare qualcosa (aggiunto 2026-07-27)

Questo repository è un insieme di librerie senza una propria `fn main`,
quindi il modo più rapido per *vedere* funzionare la pipeline grafica sulla
propria GPU — piuttosto che leggere il codice sorgente dei test — è:

```bash
cargo run -p directx-graphics-vulkan --example render_triangle
```

Questo riutilizza gli stessi shader tradotti da DXBC compilato con
fxc.exe reale → SPIR-V di `tests/triangle_real_vulkan.rs`, disegna un
triangolo a gradiente (rosso/verde/blu) su hardware Vulkan reale, rilegge
il framebuffer e lo scrive in `render_triangle.ppm` (PPM semplice, senza
bisogno di dipendenze extra da crate immagine — convertirlo con es.
`magick render_triangle.ppm render_triangle.png` o aprirlo direttamente
nella maggior parte dei visualizzatori di immagini). Se non è presente
alcun dispositivo/driver Vulkan utilizzabile, stampa un errore onesto ed
esce con codice non-zero invece di simulare un successo.

Output effettivamente osservato (2026-07-25, questa macchina, NVIDIA
GeForce GT 730):

```
running 7 tests
test spirv_gen::tests::rejects_garbage_bytes_honestly_instead_of_pretending_to_translate ... ok
test tests::rejects_garbage_bytes_that_are_not_a_dxbc_container ... ok
test tests::rejects_truncated_dxbc_header ... ok
test tests::parses_real_fxc_compiled_vector_add_dxbc_container ... ok
test spirv_gen::tests::translates_real_fxc_compiled_vector_mul_dxbc_to_valid_spirv ... ok
test spirv_gen::tests::translates_real_fxc_compiled_vector_add_dxbc_to_valid_spirv ... ok
test spirv_gen::tests::translates_real_fxc_compiled_vector_sub_bounded_dxbc_to_valid_spirv ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 1 test
device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
OK: DXBC(fxc.exe実コンパイル)->SPIR-V(自前生成)->実Vulkan(NVIDIA GT 730)経路が、CPU参照実装(a[i]+b[i])と256要素すべてで数値一致した
c[0]=128, c[255]=255.5
test dxbc_vector_add_matches_cpu_reference_on_real_vulkan_hardware ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s

running 1 test
device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
OK: DXBC(fxc.exe実コンパイル, mul)->SPIR-V(自前生成)->実Vulkan経路が、CPU参照実装(a[i]*b[i])と256要素すべてで数値一致した
c[0]=64, c[255]=6.625
test dxbc_vector_mul_matches_cpu_reference_on_real_vulkan_hardware ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.59s

running 1 test
device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
OK: DXBC(fxc.exe実コンパイル, sub+境界チェック)->SPIR-V(自前生成)->実Vulkan経路が、CPU参照実装(a[i]-b[i])と有効範囲256要素すべてで数値一致し、境界外の64要素はセンチネル値のまま(書き込まれなかった)ことを確認した
c[0]=10, c[255]=137.5, c[319]=-1
test dxbc_vector_sub_bounded_matches_cpu_reference_and_respects_bounds_on_real_vulkan_hardware ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
```

`cargo clippy --workspace --all-targets`: 0 warning.

Dopo il lavoro di decodifica della tabella dei tipi/istruzioni DXIL
(2026-07-25, continuazione, traccia D3D12), `cargo test --workspace`
esegue 23 test in totale (19 unit + 4 test di integrazione Vulkan reali),
tutti superati, inclusi 3 nuovi oltre ai 20 precedenti: `dxil::tests::
resolves_real_dxil_type_table_and_finds_float_and_resource_struct`,
`dxil::tests::decodes_real_dxil_function_block_into_matching_vector_add_
shape`, e `dxil::tests::shape_matcher_honestly_rejects_unexpected_
instruction_orderings`.

Per rigenerare le fixture DXBC dall'HLSL (richiede `fxc.exe` del Windows
SDK — si noti che `dxc.exe` mira solo a DXIL/SM6+ e non può produrre DXBC):

```powershell
pwsh tools/compile-dxbc-shaders.ps1
```

## Non implementato (scope onesto)

- **Decodifica generale delle istruzioni SM5.0.** Sono gestite solo le 3
  forme di opcode sopra; qualsiasi altro compute shader D3D11 (layout di
  risorse diverso, altro controllo di flusso, altri intrinseci, più di un
  controllo dei limiti, `div`/`sub` come vero opcode invece di `add`
  negato, ecc.) viene rifiutato, non tradotto in modo errato. Costruire un
  vero decoder generale (o adottare/portare uno esistente, es. studiando
  più da vicino l'approccio di `dxbc-spirv`/`dxil-spirv`) rimane il vero
  prossimo traguardo.
- **DXIL (Shader Model 6+, D3D12): il vertical slice di `vector_add.dxil`
  è ora completo end-to-end, su hardware reale — ma ancora solo per
  questa singola forma di shader nota, non per l'SM6.0 generale.**
  `resolve_type_table`/`decode_function_instructions`/
  `resolve_vector_add_dxil_calls` in `dxil.rs` decodificano i record reali
  `TYPE_BLOCK`/`FUNCTION_BLOCK`/`VALUE_SYMTAB_BLOCK` contro i codici
  documentati da LLVM e disambiguano tutti e 7 i record `Call` al loro
  reale significato `dx.op.*` (`CreateHandle`/`ThreadId`/`BufferLoad`/
  `BufferStore`, con bind point UAV). `translate_dxil_vector_add_to_spirv`
  (nuovo) alimenta quell'output risolto in `emit_spirv_for_kernel`,
  condiviso da `spirv_gen.rs` (estratto dall'`emit_spirv` del percorso
  DXBC in modo che entrambi i backend emettano da un unico percorso di
  codice) per produrre SPIR-V reale, che `tests/vector_add_dxil_real_
  vulkan.rs` invia sulla GT 730 NVIDIA reale di questa macchina via
  `opencuda-vulkan` e verifica corrispondere al riferimento CPU
  `a[i]+b[i]` per tutti i 256 elementi — lo stesso rigore del test DXBC
  `vector_add`. **La dimensione del workgroup viene ora effettivamente
  estratta, non hardcoded**: `extract_numthreads_from_metadata`
  (`dxil.rs`) percorre il reale `METADATA_BLOCK` seguendo il percorso
  `dx.entryPoints` -> tupla per entry-point -> `ShaderProperties` ->
  `kDxilNumThreadsTag` (=4, confermato contro i sorgenti
  `DxilMetadataHelper.h`/`.cpp` del `DirectXShaderCompiler` di Microsoft)
  e risolve il nodo `{x,y,z}` contro l'elenco di valori reale del modulo,
  ottenendo `(64,1,1)` dai byte reali di `vector_add.dxil` — l'hardcode
  noto della voce precedente è chiuso, e un test di regressione sintetico
  dimostra che la logica di estrazione restituisce un valore *diverso*
  quando riceve metadata diversi (non solo "restituisce sempre 64,1,1").
  Qualsiasi altra forma di opcode/operando (operazione diversa, più basic
  block, controlli dei limiti) viene ancora rifiutata, non tradotta in
  modo errato. Il supporto per command list/descriptor heap/root
  signature di D3D12 (il livello sopra la traduzione degli shader) resta
  intatto.
- **Decoder DXBC generalizzato oltre le 4 forme fisse a operazione
  singola: ora gestisce catene di operazioni binarie sequenziali (senza
  controllo di flusso) tramite un vero albero di espressione di
  registri, non una 5ª forma hardcoded.** `spirv_gen::translate_chain_
  shader`/`decode_chain_shape` percorrono `ld_structured`/`add`/`mul`/
  `store_structured` e costruiscono un vero albero di espressione
  indicizzato per (registro temp, componente), gestendo così 1
  operazione, 2 operazioni o N operazioni allo stesso modo — verificato
  contro uno shader reale appena compilato (`vector_add_mul_chain.hlsl`,
  `t = A[i]+B[i]; Out[i] = t*A[i]`) il cui SHEX reale si è rivelato
  riutilizzare i componenti `.x`/`.y` di un singolo registro temp (fxc ha
  eliminato tramite CSE il caricamento ripetuto di `A[i]` invece di
  riemettere `ld_structured`) — una scoperta genuina e non prevista che
  il decoder basato su albero gestisce senza casi aggiuntivi. Inviato e
  verificato sulla GT 730 NVIDIA reale contro il riferimento CPU
  `(a[i]+b[i])*a[i]`. `sub`/`div` all'interno di una catena sono
  intenzionalmente ancora rifiutati (la loro semantica dell'ordine degli
  operandi è stata verificata solo per il caso a operazione singola). Le
  4 forme originali a operazione singola sono intatte e continuano a
  passare senza modifiche.
- **Pipeline grafica D3D11: il parsing del contenitore DXBC è confermato
  funzionante per VS/PS, ma nessuna generazione di codice SPIR-V, nessun
  rasterizzatore, nessun triangolo effettivamente disegnato a schermo.**
  L'intera pipeline (rasterizzatore, campionamento delle texture, stato
  di blend, output-merger) rimane fuori scope; così come estendere il
  decoder di forme di opcode ristretto di `spirv_gen` per capire
  `dcl_output_siv`/`dcl_input_ps`/le modalità di interpolazione.
- Obiettivi della famiglia PlayStation — esplicitamente fuori scope;
  vedere `CLAUDE.md` per il ragionamento legale/sui termini di servizio.

## Progetti correlati

- [open-cuda](https://github.com/aon-co-jp/open-cuda) — il backend di
  esecuzione compute Vulkan verso cui questo progetto è progettato per
  inviare il proprio lavoro (`opencuda-core::GpuDevice`,
  `KernelSource::SpirV`). Contiene anche un crate `opencuda-directx` non
  correlato e già funzionante che esegue D3D12 **nativamente su Windows**
  — la direzione opposta rispetto a questo progetto (che esegue shader
  DirectX **su piattaforme non-Windows**).
- [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm) — nessuna
  dipendenza tecnica diretta da questo progetto (verificato, non
  presunto).
