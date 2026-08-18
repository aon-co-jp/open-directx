# open-directx

> 📌 **Letztes Update (2026-08-08, 2D-Sprite-Rendering-Prototyp)**:
> Als Reaktion auf den Nutzervorschlag „auf dream-os/Linux via
> open-directx auf dem GT730-PC einen GAME-Prototyp entwickeln“ wurde
> zunächst eng auf 2D-Sprite-Rendering fokussiert begonnen. Textur-
> sampling (`Texture2D.Sample`) erste Implementierung → Unterstützung
> für mehrere Sprites/Sprite-Sheets → Game-Loop (Positionsupdate +
> Abprall-Physik) → **echtes Fenster + echte Vulkan-Swapchain + echte
> Tastatureingabe** (neue Crate `directx-graphics-window`, vom Nutzer
> selbst ausgeführt und mit „das Paddle hat sich bewegt und den Ball
> zurückgeschlagen“ visuell bestätigt) → Alpha-Blending (halbtransparente
> Sprites) → Textur-Laden aus echten PNG-Dateien — all diese
> Inkremente wurden auf echter Windows-Hardware (NVIDIA GT 730) und
> teilweise auch auf echter Linux-Hardware (WSL2 Ubuntu) verifiziert.
> Nächste Kandidaten: mehrere bewegte Sprites mit Kollisionserkennung,
> Unterstützung für Fenstergrößenänderung. Details siehe
> [CLAUDE.md](CLAUDE.md).

> 📌 Offene Aufgabe (2026-08-06): Es gibt ein Konzept zur Integration
> von Toshibas SBM-Technologie und DeepSeek-Techniken (für dream-os und
> 7 weitere Repositories). Details siehe [CLAUDE.md](CLAUDE.md).

> 📌 **Letztes Update (2026-08-08)**: Die Asymmetrie, dass es für die
> grenzwertgeprüfte 7-Term-Kette nur eine DXBC-Seite, aber keine
> DXIL-Seite gab, wurde geschlossen — `vector_add_mul_div_sub_
> add_mul_div_chain7_bounded_dxil.hlsl` wurde neu mit `dxc.exe` real
> kompiliert und auf echter NVIDIA-GT730-Hardware auf numerische
> Übereinstimmung mit der CPU-Referenzimplementierung sowie korrektes
> Verhalten der Grenzwertprüfung überprüft (gesamter Workspace: 50
> Unit-Tests + 22 Hardware-Tests, alle grün, 0 Warnungen). Details
> siehe [CLAUDE.md](CLAUDE.md).

> 📌 **Letztes Update (2026-08-07)**: Die grenzwertgeprüfte DXBC/DXIL-
> Kette wurde auf 6 Terme erweitert und auf echter NVIDIA-GT730-
> Hardware verifiziert. Eine tiefere Integration mit dream-os/open-cuda/
> aruaru-llm (SBM/DeepSeek-Transplantation) wurde geprüft, aber es
> wurde entschieden, keine Erweiterungen der DXBC/DXIL-Kettenlogik ohne
> tiefes Domänenverständnis zu raten — es wurde kein Code geändert, die
> Untersuchungsergebnisse wurden ehrlich in [CLAUDE.md](CLAUDE.md)
> festgehalten.

> **Aktualisiert 2026-07-25**: Die Überschrift der Entwicklungsrichtlinien-
> Datei (`CLAUDE.md`) wurde von „Entwicklungsrichtlinie & Regeln der
> Entwicklungsumgebung“ zu „Designphilosophie & Entwicklungsrichtlinie &
> Regeln der Entwicklungsumgebung“ umbenannt, um die Designphilosophie
> des Projekts (was wir schätzen), die Entwicklungsrichtlinie (wie wir
> arbeiten) und die Regeln der Entwicklungsumgebung (konkrete
> operative Konventionen) klarer zu trennen. Details siehe `CLAUDE.md`.


Eine plattformübergreifende DirectX-(D3D9/10/11/12)-Kompatibilitätsschicht
— im Geiste von DXVK / vkd3d-proton — mit dem Ziel, unveränderte Windows-
DirectX-Anwendungen unter Linux (und künftig Android/macOS) auszuführen,
indem DXBC/DXIL-Shader-Bytecode nach SPIR-V übersetzt und über ein
bestehendes Vulkan-Compute-Backend
([open-cuda](https://github.com/aon-co-jp/open-cuda)s
`opencuda-vulkan`) ausgeführt wird.

Siehe [`CLAUDE.md`](CLAUDE.md) für die vollständige Design-Begründung,
den ehrlichen Umfang/Fahrplan und das Sitzungs-HANDOFF-Protokoll —
dieses README fasst nur den aktuellen, verifizierten Zustand zusammen.

### Plattform- und Hersteller-Unterstützungsmatrix (hinzugefügt 2026-07-27, ehrliche Offenlegung)

DirectX selbst ist eine reine Windows/Xbox-API — „plattformübergreifend“
bedeutet hier, dass DXBC/DXIL-Bytecode nach SPIR-V übersetzt und über
Vulkan ausgeführt wird, was tatsächlich Nicht-Windows-Plattformen
erreicht. Im Code dieses Repos existiert heute kein `cfg(windows)` oder
sonstiges Plattform-Gating (der DXBC-Parser, die SPIR-V-Codegenerierung
und `directx-graphics-vulkan` sind allesamt schlichtes,
plattformneutrales Rust + `ash`), sodass die Build-/Test-Portabilität
der Reichweite von Vulkan selbst folgt:

| Plattform | Pfad | Status |
|---|---|---|
| Windows | natives Vulkan | **Auf echter Hardware verifiziert** (Entwicklungsrechner dieses Repos, NVIDIA GeForce GT 730) |
| Linux | natives Vulkan | Sollte unverändert bauen/laufen (kein Windows-spezifischer Code blockiert dies) — **in dieser Umgebung noch nicht auf echter Linux-Hardware getestet** |
| Android | natives Vulkan | `open-cuda` hat die erfolgreiche `aarch64-linux-android`-Cross-Kompilierung verifiziert (laut dessen CLAUDE.md); die Ausführung auf echter Hardware (`vkCreateInstance` auf einem echten Smartphone) steht noch aus |
| macOS | Vulkan via [MoltenVK](https://github.com/KhronosGroup/MoltenVK) (übersetzt zu Metal) | Noch nicht versucht — MoltenVK ist eine Übersetzungsschicht, kein natives Vulkan, daher ist dies eine schwächere Garantie als bei Linux/Android |
| iOS / iPadOS (hinzugefügt 2026-08-17) | Vulkan via MoltenVK (übersetzt zu Metal) | Noch nicht versucht. **Der gleiche MoltenVK-Vorbehalt wie bei macOS gilt** — Vulkan läuft auf iOS/iPadOS nicht nativ, nur über diese Übersetzungsschicht, sodass Parität mit dem Windows/Vulkan-nativen Pfad erst nach tatsächlichem Test auf einem Gerät garantiert ist. Erfordert außerdem das Apple Developer Program für offizielle Distribution. |
| Diverse UNIX/BSD-Systeme (hinzugefügt 2026-08-17) | vermutlich natives Vulkan | Unerforscht — Vulkan-Unterstützung variiert je nach Distribution/Treiber; erwartet, dass sich der Großteil des Linux-Pfads wiederverwenden lässt, sobald untersucht |
| Sony PlayStation 4/5/6/7 | n/a | Bewusst vorerst außerhalb des Scopes — siehe Hinweis „PlayStation-Familie als Ziel“ unten sowie `CLAUDE.md` |
| Nintendo Switch 2 / 3 (hinzugefügt 2026-08-17) | n/a | Gleicher Status „zukünftige Ambition, aufgeschoben bis offizielles SDK/NDA“ wie PlayStation. **Switch 3 ist zum Stand 2026-08-17 von Nintendo noch nicht offiziell angekündigt worden — die Erwähnung hier ist nur ein Platzhalter für den Fall einer Ankündigung, nicht auf realen Produktinformationen basierend.** |

Abdeckung der GPU-Hersteller (PCI-Vendor-ID-Abgleich, konsistent über
dieses Repo und `open-cuda`: NVIDIA `0x10DE`, AMD `0x1002`/`0x1022`,
Intel `0x8086`):

| Hersteller | Status |
|---|---|
| NVIDIA | **Auf echter Hardware verifiziert** (GeForce GT 730) |
| AMD | Vendor-ID-Abgleichscode existiert und ist typkorrekt, wurde aber in dieser Umgebung **niemals gegen echte AMD-Hardware ausgeführt** — als unverifiziert zu behandeln |
| Intel | Wie AMD: Code existiert, **niemals auf echter Intel-GPU-Hardware verifiziert** |

Es ist keine Korrektur nötig, um diese drei Vendor-IDs *erkennbar* zu
machen — der Code ist bereits korrekt und identisch über
`open-directx`/`opencuda-vulkan`/`opencuda-directx` hinweg. Was fehlt,
ist echte AMD/Intel-Hardware, um diesen Codepfad tatsächlich
auszuüben — diese Entwicklungsumgebung besitzt sie nicht.

## Aktueller Stand (2026-07-27, neuestes: Gradienteninterpolation, GPU-Herstellerdiagnose, sub/div-Kette)

Drei Inkremente sind auf der D3D11-Minimal-Grafikpipeline und der
DXBC-Kettenklassen-Arbeit unten aufgebaut worden, alle auf echter
NVIDIA GT 730 dieser Maschine verifiziert: (1)
`render_gradient_triangle_and_read_back` — die Grafikpipeline kann nun
jedem Vertex eine eigene Farbe zuweisen (nicht mehr nur den entarteten
Fall einheitlicher Farbe), verifiziert über eine
Partition-der-Einheit-Invariante anhand von Readback-Pixeln echter
Hardware. (2) `enumerate_graphics_devices()` — schließt eine
Diagnoselücke, bei der `open-cuda`s Compute-Pfad Vendor-ID-Erkennung
hatte, der Graphics-Pfad hier jedoch nicht; eigenständig, ohne neue
Abhängigkeit von `opencuda-vulkan`. (3) `decode_chain_shape`
unterstützt nun `sub`/`div` (zuvor explizit als nicht verifizierbar
abgelehnt) — ein neuer Shader (`vector_sub_div_chain.hlsl`) wurde
tatsächlich mit `fxc.exe` kompiliert und dessen SHEX-Dump genutzt, um
die exakte Operandenreihenfolge zu bestätigen, dann End-to-End gegen
eine CPU-Referenz auf echter Hardware verifiziert. Siehe `CLAUDE.md`-
HANDOFF (Einträge vom 2026-07-27) für den vollständigen Bericht.

## Aktueller Stand (2026-07-25, neuestes: DXIL-Vertikalscheibe vollständig auf echter Hardware)

Die D3D12/DXIL-Compute-Shader-Vertikalscheibe erreicht nun volle Parität
mit der D3D11/DXBC-Scheibe: `vector_add.dxil` (echte
`dxc.exe -T cs_6_0`-Ausgabe) wird End-to-End dekodiert (Container ->
LLVM-Bitstream -> Typtabelle -> Instruktionen -> alle 7
`Call`-Datensätze zu ihrer realen `dx.op.*`-Bedeutung disambiguiert) und
in echtes SPIR-V übersetzt
(`directx_shader_translate::translate_dxil_vector_add_to_spirv`), was
`tests/vector_add_dxil_real_vulkan.rs` auf der echten NVIDIA GT 730
dieser Maschine ausführt und numerisch mit der CPU-Referenz `a[i]+b[i]`
abgleicht. Dies ist weiterhin nur eine bekannte Shader-Form, kein
allgemeiner SM6.0-Decoder — siehe „Nicht implementiert (ehrlicher
Umfang)“ unten für die genaue Grenze. Die SPIR-V-Workgroup-Größe wird
nun tatsächlich aus DXILs `METADATA_BLOCK` extrahiert
(`dx.entryPoints` -> `ShaderProperties` -> `NumThreads`), nicht mehr
hartkodiert — siehe der HANDOFF-Eintrag „Fortsetzung 9“ vom 2026-07-25
in `CLAUDE.md` für den vollständigen Bericht, und „Fortsetzung 7“ für
die ursprüngliche Errungenschaft der Vertikalscheibe, deren bekannte
Lücke dies schloss.

## Aktueller Stand (2026-07-25, Fortsetzung: DXIL-Bitstream-Level-Parsing + D3D11-VS/PS-DXBC-Parsing)

Zwei neue Arbeiten sind auf der Phase-1-Compute-Shader-Vertikalscheibe
unten aufgebaut worden:

- **DXIL (D3D12/SM6+) — echte Bytes geparst, nur auf Container-/
  Bitstream-Ebene.** `crates/directx-shader-translate/src/dxil.rs`
  (`parse_dxil_container`) parst einen echten, mit `dxc.exe -T cs_6_0`
  kompilierten DXBC-Container (`shaders/vector_add_dxil.hlsl` ->
  `shaders/vector_add.dxil`, erzeugt von
  `tools/compile-dxbc-shaders.ps1`): extrahiert den `DxilProgramHeader`/
  `DxilBitcodeHeader` (Shader-Art, SM6.0, DXIL-Version) des `DXIL`-
  Chunks über die bestehende `dxbc`-Crate, und übergibt dann die rohe
  LLVM-Bitcode-Payload an die `llvm-bitcode`-Crate (neu hinzugefügte
  Abhängigkeit, generischer LLVM-Bitstream-Reader ohne DXIL-spezifisches
  Wissen), um den Block-/Datensatzbaum tatsächlich zu dekodieren.
  Gegen echte Bytes bestätigt: die LLVM-Wrapper-Magic `BC\xC0\xDE`, ein
  einzelner Top-Level-`MODULE_BLOCK` (ID 8) und standardisierte
  LLVM-Unterblöcke darin — `TYPE_BLOCK_ID_NEW`(17),
  `PARAMATTR_GROUP_BLOCK`(10), `PARAMATTR_BLOCK`(9),
  `CONSTANTS_BLOCK`(11), `FUNCTION_BLOCK`(12, x5 — einer pro Basisblock
  von `main`), `VALUE_SYMTAB_BLOCK`(14), `METADATA_BLOCK`(15, x2).
  **Update (2026-07-25, Fortsetzung, D3D12-Spur)**: Die Auflösung der
  Typtabelle und grobe Instruktionsdekodierung wurden inzwischen
  hinzugefügt (`resolve_type_table`/`decode_function_instructions` in
  derselben Datei), unter Anwendung von LLVMs dokumentierten
  `TYPE_BLOCK`/`FUNC_CODE`-Datensatztabellen auf die echten
  `vector_add.dxil`-Bytes — eine 22-Einträge-Typtabelle inklusive
  `Float` und `StructNamed{"class.RWStructuredBuffer<float>"}` sowie
  eine echte Instruktionssequenz (`DeclareBlocks -> Call*5 ->
  ExtractValue -> Call -> ExtractValue -> BinOp -> Call -> Ret`)
  bestätigt. **Update (2026-07-25, Fortsetzung 6)**: alle 7
  `Call`-Datensätze sind nun disambiguiert.
  `resolve_vector_add_dxil_calls` löst `VALUE_SYMTAB_BLOCK`-
  Funktionsnamen auf (gefunden via `Record::take_payload()`, nicht
  `fields()` — eine echte Lücke im Verständnis der Crate im vorherigen
  Eintrag) und dekodiert von Hand LLVMs relative-Wert-Operanden-
  Kodierung (von Hand gegen die echten Bytes verifiziert), was
  `[CreateHandle{range_id:2}, CreateHandle{range_id:1},
  CreateHandle{range_id:0}, ThreadId, BufferLoad{handle_range_id:0},
  BufferLoad{handle_range_id:1}, BufferStore{handle_range_id:2}]`
  ergibt. DXIL-Opcode-Nummern (`CreateHandle`=57, `BufferLoad`=68,
  `BufferStore`=69, `ThreadId`=93) wurden per Websuche gegen Microsofts
  `DirectXShaderCompiler/docs/DXIL.rst` bestätigt, nicht aus dem
  Gedächtnis angenommen, und stimmten exakt mit den echten dekodierten
  Konstanten überein. **Noch keine DXIL-zu-SPIR-V-Übersetzung** — das
  ist das nächste Inkrement. Siehe „Nicht implementiert“ unten.
- **D3D11-Grafikpipeline — echte SPIR-V-Generierung für VS/PS erreicht
  und validiert, noch kein Rasterizer/Draw.**
  `shaders/triangle_vs.hlsl`/`shaders/triangle_ps.hlsl` (minimales
  Passthrough-Vertex-+Pixel-Shader-Paar, `POSITION`/`COLOR` als Eingabe,
  `SV_POSITION`/`SV_TARGET` als Ausgabe) mit echtem
  `fxc.exe /T vs_5_0`/`/T ps_5_0` kompiliert. `parse_dxbc` parst beide
  ohne Änderung. `spirv_gen::translate_vertex_shader`/
  `translate_pixel_shader` (neu) dekodieren die echte, feste SHEX-
  Opcode-Sequenz (`dcl_input`x2/`dcl_output_siv`/`dcl_output`/`mov`x3/
  `ret` für VS; `dcl_input_ps`(linear)/`dcl_output`/`mov`/`ret` für PS)
  und emittieren echtes Grafik-SPIR-V: `OpEntryPoint Vertex`/`Fragment`
  (nicht `GLCompute`), `Input`/`Output`-Speicherklassen-Variablen mit
  `Location`-Dekorationen, `BuiltIn Position` auf der `SV_POSITION`-
  Ausgabe des Vertex-Shaders, und `OpExecutionMode ... OriginUpperLeft`
  auf dem Fragment-Shader. Auf zwei Arten validiert: (1) `rspirv`s
  eigener Loader parst die emittierten Bytes ohne Fehler erneut, (2)
  das echte Vulkan-SDK-`spirv-val.exe`
  (`C:\VulkanSDK\1.4.350.0\Bin\spirv-val.exe`) wurde gegen beide
  emittierten Module ausgeführt und lieferte Exit-Code 0 ohne
  Diagnosen für beide. `translate_shader`/`translate_chain_shader`
  (nur Compute) lehnen weiterhin beide Shader korrekt ab. **Kein
  Rasterizer, kein Framebuffer, kein tatsächlicher Vulkan-Draw-Call
  existiert** — `opencuda-vulkan` wurde (durch Lesen dessen echten
  Quellcodes) als komplett ohne
  `VkGraphicsPipelineCreateInfo`/Render-Pass-/Framebuffer-Code bestätigt,
  nur Compute-Dispatch, sodass ein tatsächlich gerendertes Pixel für
  diesen Durchgang außerhalb des Scopes liegt. Siehe `CLAUDE.md`-
  HANDOFF für die ehrliche Meilenstein-Grenze.

## Aktueller Stand (2026-07-26, Meilenstein D3D11-Minimal-Grafikpipeline erreicht)

Die neue Crate `crates/directx-graphics-vulkan` fügt `ash` als
**direkte** Abhängigkeit dieses Workspaces hinzu (nicht auf
`opencuda-vulkan` aufgesetzt, das durch Quellcode-Audit als reines
Compute-Dispatch bestätigt wurde). Sie implementiert einen echten
Render-Pass, Framebuffer und `VkGraphicsPipelineCreateInfo`, unter
Wiederverwendung des bereits generierten, bereits `spirv-val`-
bestandenen SPIR-V aus `translate_vertex_shader`/`translate_pixel_shader`
oben (keine Shader-Übersetzung wird neu implementiert).
`render_uniform_triangle_and_read_back` zeichnet ein vollflächiges
„großes Dreieck“ mit einer einheitlichen Vertex-Farbe, liest das
gerenderte Bild über einen host-sichtbaren Staging-Buffer zurück, und
der echte Hardware-Test
(`crates/directx-graphics-vulkan/tests/triangle_real_vulkan.rs`)
bestätigt, dass jedes zurückgelesene Pixel der Passthrough-Vertex-
Farbe auf der echten NVIDIA GT 730 dieser Maschine entspricht
(`cargo test -p directx-graphics-vulkan --test triangle_real_vulkan --
--nocapture`: 1 bestanden). Der Umfang ist bewusst eng: ein festes
Shader-Paar, ein Draw-Call, keine Tiefenpuffer/Texturen/Swapchain/
Mehrdreieck-Interpolationsprüfung. Siehe `CLAUDE.md`-HANDOFF (2026-07-26
Fortsetzung) für die vollständige ehrliche Offenlegung.

## Aktueller Stand (2026-07-25, Phase-1-Vertikalscheibe auf 3 bekannte Shader verallgemeinert)

`crates/directx-shader-translate` erledigt nun die vollständige
Vertikalscheibe für **drei spezifische bekannte Shader**
(`vector_add.hlsl`, `vector_mul.hlsl`, `vector_sub_bounded.hlsl`):
DXBC-Parsing -> Dekodierung des engen SM5.0-Opcode-Teilmenge ->
SPIR-V-Codegenerierung (via `rspirv`) -> echter Vulkan-Dispatch
(`open-cuda`s `opencuda-vulkan`) -> numerischer Abgleich mit der
CPU-Referenz, verifiziert auf der echten NVIDIA GT 730 dieser Maschine.
**Dies ist weiterhin kein allgemeiner SM5.0-zu-SPIR-V-Decoder** — siehe
„Nicht implementiert“ unten.

- `parse_dxbc` (Phase 0): DXBC-Container-/Chunk-Introspektion
  (Vorhandensein von RDEF/ISGN/OSGN/SHEX), unverändert gegenüber dem
  ursprünglichen Frontend.
- `spirv_gen::translate_shader` (Phase 1, verallgemeinert 2026-07-25):
  erkennt 3 Opcode-Formen, die tatsächlich von `fxc.exe` emittiert
  werden, alle mit einem gemeinsamen Grundgerüst
  (`dcl_globalFlags` -> optional `dcl_constantbuffer` -> 3x
  `dcl_uav_structured` -> `dcl_input` -> `dcl_temps` ->
  `dcl_thread_group` -> optional `ult`+`if` -> 2x `ld_structured` ->
  `add`/`mul` -> `store_structured` -> optional `endif` -> `ret`):
  - `vector_add.hlsl`: `add`, ohne Grenzwertprüfung.
  - `vector_mul.hlsl`: `mul` statt `add`.
  - `vector_sub_bounded.hlsl`: `add` mit einem `negate`-Flag am ersten
    Quelloperanden (durch Dump der echten `fxc.exe`-Ausgabe bestätigt —
    `fxc` optimiert `a - b` zu `add dest, -b, a` statt einen eigenen
    `sub`-Opcode zu emittieren), plus eine echte
    `if (id.x < N)`-Grenzwertprüfung (`ult` gegen einen Constant Buffer
    + `if`/`endif`), die das emittierte SPIR-V mit einem echten
    `OpSelectionMerge`/`OpBranchConditional` implementiert, wobei die
    Push-Constant `n` für den Vergleich verwendet wird.
  Jede andere Opcode-/Operanden-Form wird über
  `SpirvGenError::UnsupportedShader` abgelehnt statt sie still falsch
  zu übersetzen. UAV-Bindungspunkte, Thread-Group-Größe, Operator und
  Vorhandensein der Grenzwertprüfung werden allesamt aus dem echten
  geparsten DXBC extrahiert, nicht hartkodiert.
  `translate_vector_add_shader` bleibt als dünner rückwärtskompatibler
  Alias für `translate_shader` erhalten.
- `tests/vector_add_real_vulkan.rs`, `tests/vector_mul_real_vulkan.rs`,
  `tests/vector_sub_bounded_real_vulkan.rs`: jeder übergibt sein
  übersetztes SPIR-V an `open-cuda`s echtes
  `opencuda-vulkan::VulkanDevice` (`ash`, Feature `real-vulkan`) und
  prüft die GPU-Ausgabe gegen eine CPU-Referenz für 256 Elemente
  (Epsilon 1e-3/1e-2). Der Grenzwerttest übergibt zusätzlich 320
  Threads mit einer logischen Elementanzahl von 256 und stellt sicher,
  dass die Elemente 256..320 nie geschrieben werden (bleiben beim
  Sentinel-Wert), was beweist, dass der `if (id.x < N)`-Zweig im
  generierten SPIR-V die Ausführung tatsächlich gate't statt nur zu
  kompilieren.
- `examples/dump_shex.rs`: ein kleines eigenständiges Werkzeug
  (`cargo run -p directx-shader-translate --example dump_shex --
  <path.dxbc>`), das während dieser Sitzung genutzt wurde, um echte
  SHEX-Opcode-Ströme vor dem Schreiben von Decoder-Unterstützung dafür
  zu inspizieren; für künftige opcodeweise Verallgemeinerungsarbeit
  erhalten.

**Seit der Titel dieses Abschnitts geschrieben wurde**, wurde ein
vierter Einzeloperations-Shader (`vector_div.hlsl`, reines `div`) zu
`translate_shader` nach demselben Muster hinzugefügt, und — jüngeren
Datums — eine wirklich andere Musterklasse,
`spirv_gen::translate_chain_shader`, wurde daneben (nicht anstelle
davon) hinzugefügt: sie dekodiert einen echten
Register-Ausdrucksbaum sequenzieller binärer Operationen (add/mul,
keine Kontrollflusssteuerung) statt einer einzelnen festen Operation,
verifiziert gegen einen neu kompilierten Shader, dessen echtes SHEX
sich als Wiederverwendung der Komponenten eines Temp-Registers durch
fxcs CSE herausstellte, statt zusätzliche Temps zu deklarieren. Siehe
den HANDOFF-Eintrag „Fortsetzung 9“ vom 2026-07-25 in `CLAUDE.md` für
den vollständigen, aktuellen Bericht (dieser Abschnitt bleibt wie
ursprünglich geschrieben, aus historischer Genauigkeit über den
Zwischenstand vom 2026-07-25 Mittag).

## Build & Test

```powershell
cargo build --workspace
cargo test --workspace -- --nocapture
```

### Etwas tatsächlich zeichnen sehen (hinzugefügt 2026-07-27)

Dieses Repo ist eine Sammlung von Bibliotheken ohne eigenes `fn main`,
daher ist der schnellste Weg, die Grafikpipeline auf der eigenen GPU
*sehen* zu können — statt den Test-Quellcode zu lesen —:

```bash
cargo run -p directx-graphics-vulkan --example render_triangle
```

Dies verwendet dieselben, mit echtem fxc.exe kompilierten und nach
SPIR-V übersetzten Shader wie `tests/triangle_real_vulkan.rs`, zeichnet
ein Gradienten-Dreieck (rot/grün/blau) auf echter Vulkan-Hardware,
liest den Framebuffer zurück und schreibt ihn nach
`render_triangle.ppm` (reines PPM, keine zusätzliche Image-Crate-
Abhängigkeit nötig — konvertierbar z. B. mit
`magick render_triangle.ppm render_triangle.png` oder direkt in den
meisten Bildbetrachtern zu öffnen). Ist kein nutzbares Vulkan-Gerät/
-Treiber vorhanden, wird ehrlich ein Fehler ausgegeben und mit
Exit-Code ungleich Null beendet, statt einen Erfolg vorzutäuschen.

Tatsächlich beobachtete Ausgabe (2026-07-25, diese Maschine, NVIDIA
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

`cargo clippy --workspace --all-targets`: 0 Warnungen.

Nach der DXIL-Typtabellen-/Instruktionsdekodierungsarbeit (2026-07-25,
Fortsetzung, D3D12-Spur) führt `cargo test --workspace` insgesamt 23
Tests aus (19 Unit- + 4 echte Vulkan-Integrationstests), alle
bestanden, einschließlich 3 neuer gegenüber den vorherigen 20:
`dxil::tests::resolves_real_dxil_type_table_and_finds_float_and_
resource_struct`, `dxil::tests::decodes_real_dxil_function_block_into_
matching_vector_add_shape` und `dxil::tests::shape_matcher_honestly_
rejects_unexpected_instruction_orderings`.

Um die DXBC-Fixtures aus HLSL neu zu generieren (erfordert das
`fxc.exe` des Windows SDK — Hinweis: `dxc.exe` zielt nur auf
DXIL/SM6+ ab und kann kein DXBC erzeugen):

```powershell
pwsh tools/compile-dxbc-shaders.ps1
```

## Nicht implementiert (ehrlicher Umfang)

- **Allgemeine SM5.0-Instruktionsdekodierung.** Nur die 3 obigen
  Opcode-Formen werden behandelt; jeder andere D3D11-Compute-Shader
  (anderes Ressourcenlayout, anderer Kontrollfluss, andere
  Intrinsics, mehr als eine Grenzwertprüfung, `div`/`sub` als echter
  Opcode statt negiertem `add` usw.) wird abgelehnt, nicht falsch
  übersetzt. Einen echten allgemeinen Decoder zu bauen (oder einen
  bestehenden zu übernehmen/portieren, z. B. `dxbc-spirv`/
  `dxil-spirv`s Ansatz genauer zu studieren) bleibt der eigentliche
  nächste Meilenstein.
- **DXIL (Shader Model 6+, D3D12): Die `vector_add.dxil`-
  Vertikalscheibe ist nun End-to-End vollständig, auf echter Hardware
  — aber weiterhin nur für diese eine bekannte Shader-Form, nicht für
  allgemeines SM6.0.** `resolve_type_table`/
  `decode_function_instructions`/`resolve_vector_add_dxil_calls` in
  `dxil.rs` dekodieren die echten `TYPE_BLOCK`/`FUNCTION_BLOCK`/
  `VALUE_SYMTAB_BLOCK`-Datensätze gegen LLVMs dokumentierte Codes und
  disambiguieren alle 7 `Call`-Datensätze zu ihrer echten `dx.op.*`-
  Bedeutung (`CreateHandle`/`ThreadId`/`BufferLoad`/`BufferStore`, mit
  UAV-Bindungspunkten). `translate_dxil_vector_add_to_spirv` (neu)
  speist diese aufgelöste Ausgabe in `spirv_gen.rs`s gemeinsamen
  `emit_spirv_for_kernel` (aus dem DXBC-Pfad-`emit_spirv`
  herausgezogen, sodass beide Backends aus einem einzigen Codepfad
  emittieren), um echtes SPIR-V zu erzeugen, welches
  `tests/vector_add_dxil_real_vulkan.rs` auf der echten NVIDIA GT 730
  dieser Maschine via `opencuda-vulkan` ausführt und gegen die
  CPU-Referenz `a[i]+b[i]` für alle 256 Elemente prüft — dieselbe
  Strenge wie der DXBC-`vector_add`-Test. **Die Workgroup-Größe wird
  nun tatsächlich extrahiert, nicht hartkodiert**:
  `extract_numthreads_from_metadata` (`dxil.rs`) durchläuft den echten
  `METADATA_BLOCK`-Pfad `dx.entryPoints` -> Tupel pro Entry-Point ->
  `ShaderProperties` -> `kDxilNumThreadsTag` (=4, gegen Microsofts
  `DirectXShaderCompiler`s `DxilMetadataHelper.h`/`.cpp`-Quellen
  bestätigt) und löst den `{x,y,z}`-Knoten gegen die echte Wertliste
  des Moduls auf, was `(64,1,1)` aus den tatsächlichen Bytes von
  `vector_add.dxil` liefert — die bekannte Hartkodierung aus dem
  vorherigen Eintrag ist geschlossen, und ein synthetischer
  Regressionstest beweist, dass die Extraktionslogik einen
  *anderen* Wert zurückgibt, wenn andere Metadaten übergeben werden
  (nicht nur „gibt immer 64,1,1 zurück, egal was“). Jede andere
  Opcode-/Operanden-Form (andere Operation, mehrere Basisblöcke,
  Grenzwertprüfungen) wird weiterhin abgelehnt, nicht falsch
  übersetzt. D3D12-Command-List-/Descriptor-Heap-/Root-Signature-
  Unterstützung (die Schicht über der Shader-Übersetzung) ist
  unangetastet.
- **DXBC-Decoder über 4 feste Einzeloperations-Formen hinaus
  verallgemeinert: behandelt nun Ketten sequenzieller binärer
  Operationen (kein Kontrollfluss) über einen echten
  Register-Ausdrucksbaum, nicht als 5. hartkodierte Form.**
  `spirv_gen::translate_chain_shader`/`decode_chain_shape` durchlaufen
  `ld_structured`/`add`/`mul`/`store_structured` und bauen einen
  echten Ausdrucksbaum auf, verschlüsselt nach (Temp-Register,
  Komponente), sodass 1 Op, 2 Ops oder N Ops gleich behandelt werden
  — verifiziert gegen einen neu kompilierten echten Shader
  (`vector_add_mul_chain.hlsl`, `t = A[i]+B[i]; Out[i] = t*A[i]`),
  dessen echtes SHEX sich als Wiederverwendung der `.x`/`.y`-
  Komponenten eines einzelnen Temp-Registers herausstellte (fxc hat
  das wiederholte `A[i]`-Load per CSE wegoptimiert statt erneut
  `ld_structured` auszugeben) — ein echter, unvorhergesehener Fund,
  den der baumbasierte Decoder ohne zusätzliche Fälle behandelt.
  Auf der echten NVIDIA GT 730 gegen die CPU-Referenz
  `(a[i]+b[i])*a[i]` ausgeführt und verifiziert. `sub`/`div` innerhalb
  einer Kette werden bewusst weiterhin abgelehnt (deren
  Operandenreihenfolgen-Semantik wurde nur für den Einzeloperations-
  Fall verifiziert). Die ursprünglichen 4 Einzeloperations-Formen sind
  unangetastet und bestehen weiterhin unverändert.
- **D3D11-Grafikpipeline: DXBC-Container-Parsing für VS/PS als
  funktionierend bestätigt, aber keine SPIR-V-Codegenerierung, kein
  Rasterizer, kein tatsächlich auf dem Bildschirm gezeichnetes
  Dreieck.** Die vollständige Pipeline (Rasterizer, Texturesampling,
  Blend-State, Output-Merger) bleibt außerhalb des Scopes; ebenso die
  Erweiterung von `spirv_gen`s enger Opcode-Form-Decoder um
  `dcl_output_siv`/`dcl_input_ps`/Interpolationsmodi.
- PlayStation-Familie als Ziel — bewusst außerhalb des Scopes; siehe
  `CLAUDE.md` für die rechtliche/AGB-Begründung.

## Verwandte Projekte

- [open-cuda](https://github.com/aon-co-jp/open-cuda) — das Vulkan-
  Compute-Ausführungs-Backend, über das dieses Projekt gedacht ist zu
  dispatchen (`opencuda-core::GpuDevice`, `KernelSource::SpirV`).
  Enthält außerdem eine unabhängige, bereits funktionierende
  `opencuda-directx`-Crate, die D3D12 **nativ unter Windows**
  ausführt — die entgegengesetzte Richtung zu diesem Projekt (das
  DirectX-Shader **auf Nicht-Windows-Zielen** ausführt).
- [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm) — keine
  direkte technische Abhängigkeit von diesem Projekt (verifiziert,
  nicht angenommen).
