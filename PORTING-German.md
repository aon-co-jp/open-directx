# PORTING.md — was wiederverwendbar ist, von wem, und wie (kondensiert)

> **Hinweis**: Dies ist eine kondensierte Übersetzung. Der vollständige
> technische Leitfaden mit Code-Details und Fallstricken bleibt nur
> im originalen [PORTING.md](PORTING.md) verfügbar — vor der
> tatsächlichen Übernahme eines Musters dort nachschlagen.

Zusammenfassung wiederverwendbarer Implementierungsmuster aus diesem
Projekt, für alle, die sie in ein anderes Projekt portieren möchten:

1. **`crates/directx-shader-translate`**: ein DXBC-(D3D9/10/11-Shader-
   Bytecode, SM<=5.1)-Container-/Chunk-Parser (`parse_dxbc`), plus ein
   eng gefasster DXBC(SM5.0)→SPIR-V-Übersetzer (`translate_shader`)
   für genau die Opcode-Formen, die `fxc.exe` tatsächlich für die
   eigenen Shader dieses Projekts emittiert: Einzeloperationen
   (add/mul/sub-via-negiertes-add/div), sequenzielle
   Binäroperationsketten beliebiger Länge (basierend auf einem
   Register-Ausdrucksbaum, nicht pro Länge hartkodiert — verifiziert
   bis zu 7 Termen mit *null* nötigen Änderungen am Produktionscode
   pro zusätzlicher Termanzahl), sowie grenzwertgeprüfte
   (`if (id.x < N)`) Varianten derselben, die echtes
   `OpSelectionMerge`/`OpBranchConditional`-SPIR-V erzeugen. Jede
   andere Opcode-Form wird ehrlich über
   `SpirvGenError::UnsupportedShader` abgelehnt statt sie still falsch
   zu übersetzen — dies ist **kein** allgemeiner SM5.0-Decoder.
2. **DXIL-(SM6+, D3D12)-Unterstützung** (`src/dxil.rs`):
   `parse_dxil_container` durchläuft den rohen LLVM-Bitstream (über
   die generische `llvm-bitcode`-Crate), um die Typtabelle
   aufzulösen und Funktionsinstruktionen zu dekodieren; die
   `resolve_vector_add_dxil_calls`-Funktionsfamilie disambiguiert
   DXILs `dx.op.*`-Intrinsic-Aufrufe (CreateHandle/ThreadId/
   BufferLoad/BufferStore) gegen Microsofts dokumentierte DXIL.rst-
   Opcode-Nummern. `translate_dxil_..._to_spirv` teilt sich einen
   SPIR-V-Emitter (`emit_spirv_for_kernel`) mit dem DXBC-Backend,
   sodass beide Containerformate SPIR-V über einen einzigen Codepfad
   erzeugen. Die Workgroup-Größe (`numthreads`) wird tatsächlich aus
   DXILs `METADATA_BLOCK` extrahiert (`dx.entryPoints` →
   `ShaderProperties`), nicht hartkodiert. Die Verallgemeinerung der
   grenzwertgeprüften Kette (siehe oben) wurde End-to-End auf DXBC
   und DXIL für Ketten bis zu 7 Termen verifiziert.
3. **D3D11-Grafikpipeline**: `translate_vertex_shader`/
   `translate_pixel_shader` emittieren echtes Grafik-SPIR-V für ein
   festes Passthrough-VS/PS-Paar, auf zwei unabhängigen Wegen
   validiert (rspirv-Re-Parsing + das echte Vulkan-SDK-`spirv-val.exe`,
   beide Exit-Code 0). Die neue Crate `crates/directx-graphics-vulkan`
   fügt `ash` als **direkte** eigene Abhängigkeit dieses Projekts
   hinzu (nicht auf `opencuda-vulkan` aufgesetzt, das per
   Quellcode-Audit als reines Compute-Dispatch bestätigt wurde) und
   implementiert einen echten Render-Pass, Framebuffer und
   `VkGraphicsPipelineCreateInfo`. Es zeichnet und liest Dreiecke und
   texturierte/Mehrfach-Sprite-Szenen mit Standard-„Over“-Alpha-
   Blending und echtem PNG-Datei-Textur-Laden (`png`-Crate) zurück,
   alles auf echter NVIDIA-GT-730-Hardware (Windows) *und* WSL2
   Ubuntu/Mesa llvmpipe (Linux) verifiziert, mit übereinstimmenden
   Ergebnissen auf beiden Betriebssystemen.
4. **`crates/directx-graphics-window`**: ein echtes Fenster + echte
   Vulkan-Swapchain + echte Tastatureingabe (winit + ash-window), mit
   einer eigenen, unabhängigen Vulkan-Instanz/-Gerät getrennt von
   `directx-graphics-vulkan`s Offscreen-Kontext — die beiden bei
   gemeinsamer Nutzung synchron halten (z. B. ist Alpha-Blending
   derzeit nur im Offscreen-Pfad aktiviert). Erzeugte eine
   interaktive Breakout-artige Demo, vom Nutzer selbst mit eigenen
   Augen als funktionierend bestätigt (Paddle-Bewegung + Ball-Abprall).
5. **Pfadabhängigkeits-Konvention**: diese Crate hängt von
   `open-cuda`s `opencuda-core`/`opencuda-vulkan` über relative
   Pfadabhängigkeiten ab, eine Verzeichnisebene tiefer als die im
   restlichen Ökosystem übliche Konvention „Geschwister-Repos unter
   `F:\runo`“ (z. B. `aruaru-llm`, `aruaru-db`) — dies sind
   **ausschließlich Dev-Dependencies**, die veröffentlichte
   Bibliothek selbst hat keine Abhängigkeit von `open-cuda`.
6. **Hinweis zum Umfang bezüglich Kernel-Level-Anti-Cheat**: für alle,
   die dieses Projekt in Richtung „echte Windows-Spiele unter Linux
   ausführen“ portieren wollen — Kernel-Modus-Anti-Cheat (Riot
   Vanguard, Kernel-Modus-BattlEye usw.) blockiert Linux-/Proton-
   artige Umgebungen konstruktionsbedingt, unabhängig davon, wie
   vollständig diese Shader-Übersetzungsschicht wird. Dies ist kein
   zu behebender Mangel; Titel mit solchem Anti-Cheat liegen
   unabhängig von der Vollständigkeit der Übersetzung außerhalb der
   Reichweite dieses Projekts.

## Was noch NICHT wiederverwendbar ist (ehrliche Lücken)

- Kein allgemeiner SM5.0- oder SM6.0-Instruktionsdecoder — nur die
  oben beschriebenen spezifischen Opcode-/Formklassen werden
  behandelt; alles andere wird ehrlich abgelehnt, nicht falsch
  übersetzt.
- D3D12s höhere Schichten (Command Lists, Descriptor Heaps, Root
  Signatures) sind vollständig unimplementiert.
- Kein Tiefenpuffer, keine Prüfung der Interpolation zwischen
  unterschiedlich gefärbten Vertices über den Gradienten-Dreieck-Fall
  hinaus, und AMD-/Intel-GPU-Hardware sowie native macOS-/Linux-
  Desktop-Ausführung bleiben unverifiziert (Codepfade für AMD/Intel-
  PCI-Vendor-ID-Erkennung existieren, wurden aber in dieser
  Entwicklungsumgebung nie gegen echte AMD-/Intel-Hardware
  ausgeführt).

---

Für die vollständigen technischen Details (exakte Opcode-Sequenzen,
Byte-Level-DXIL-Bitstream-Traces, Code-Snippets und die vollständigen
Cargo.toml-Beispiele der Pfadabhängigkeiten) siehe das originale
[PORTING.md](PORTING.md) (Englisch, maßgeblich).
