# Designphilosophie & Entwicklungsrichtlinie & Regeln der Entwicklungsumgebung (open-directx) — kondensiert

> **Hinweis**: Dies ist eine kondensierte Übersetzung des aktuellen
> Zustands. Das vollständige historische HANDOFF-Protokoll (Dutzende
> von Einträgen seit 2026-07-25) bleibt aus Gründen der Kürze nur auf
> Japanisch in CLAUDE.md verfügbar — siehe dort für Details zu
> einzelnen Sitzungen.

Arbeitslaufwerk: `F:\runo`. Dieser Abschnitt folgt der Praxis, das
`CLAUDE.md` von
[`open-raid-z`](https://github.com/aon-co-jp/open-raid-z) als
Referenz zu übernehmen und in jedes Projekt zu kopieren. GitHub-Repo:
[aon-co-jp/open-directx](https://github.com/aon-co-jp/open-directx).

**Entwicklungsbeginn: 2026-07-25** (das leere Repository auf GitHub
selbst wurde bereits am 2026-07-01 angelegt).

## Offene Aufgabe (hinzugefügt 2026-08-06, noch nicht begonnen)

Es gibt einen Plan, per Nutzeranweisung, Toshibas Simulated-
Bifurcation-Machine-Technologie (pseudo-quantum computing) und
DeepSeeks Techniken (MLA, DeepSeekMoE, FP8-Mixed-Precision-Training,
recherchiert aus Papers/Implementierungs-Blogs, nicht nur aus
Nachrichten) in 8 Repositories einzubringen, darunter `open-directx`.
Für dieses Repo wurde noch kein konkretes Optimierungsziel
identifiziert — die Untersuchung ist auf eine künftige Sitzung
verschoben.

## Rolle dieses Projekts

Eine plattformübergreifende DirectX-(D3D9/10/11/12)-
Kompatibilitätsschicht mit dem Ziel, bestehende Apps/Spiele, die für
die reine Windows-API DirectX geschrieben wurden, unter Linux, Android
und künftig auch macOS und der PlayStation-Familie laufen zu lassen.

Am 2026-07-25 verpflichtete sich das Projekt durch ausdrückliche
Nutzerentscheidung, eine **echte Rückwärts-Kompatibilitätsschicht**
(unveränderte Windows-DirectX-Binaries/-Shader auf anderen
Betriebssystemen ausführen) zu verfolgen, statt der früheren
Alternative, „eine DirectX-artige API auf Vulkan als gemeinsamer Basis
bereitzustellen“.

## Korrektur der technischen Positionierung (2026-07-25, wichtig)

Eine frühere Einschätzung (2026-07-23) urteilte, DXVK/vkd3d-proton/
MoltenVK würden nur einseitig übersetzen (DirectX → Vulkan/Metal) ohne
echtes Beispiel für die umgekehrte Richtung, und seien daher technisch
schlecht geeignet. Dies war eine **Vermischung zweier verschiedener
Achsen** und wurde korrigiert:

- DXVK/vkd3d-proton (die Technologie hinter Valves Proton / der
  Linux-Steam-DirectX-Spielekompatibilität) und MoltenVK-basiertes
  CrossOver/Whisky (macOS) sind echte, funktionierende Beispiele für
  genau die vom Nutzer gewünschte Rückwärtskompatibilität: echte
  bestehende DirectX-Binaries/-Spiele (reine Windows-API) unverändert
  unter Linux/macOS auszuführen.
- Die **Richtung der Übersetzung** („DirectX-Aufrufe zu Vulkan-Aufrufen
  übersetzen“) und die **Richtung des Endnutzererlebnisses**
  („läuft eine für Windows bestimmte DirectX-App tatsächlich unter
  Linux/macOS“) sind getrennte Achsen — dass Erstere auf Vulkan
  abzielt, hindert Letztere nicht daran, korrekt „DirectX auf einem
  anderen OS“ zu erreichen.
- Daher steht die Nutzung von Vulkan als internes Ausführungs-Backend
  nicht im Widerspruch zum Ziel einer echten Rückwärts-
  Kompatibilitätsschicht — DXVK usw. sind genau dieser Präzedenzfall.
  Dieses Projekt übernimmt denselben Ansatz (Abfangen von D3D-API-
  Aufrufen + Laufzeitübersetzung von DXBC/DXIL-Shader-Bytecode →
  Ausführung via Vulkan).

## Umfang und ehrlicher Fahrplan

**Phase 0 (aktuell, Design-/Recherchestadium)**:
- Untersuchung der Struktur von DXBC/DXIL (DirectX-Shader-Bytecode-
  Formate).
- Studium der Architektur bestehender OSS-Implementierungen (DXVK,
  vkd3d-proton, [dxil-spirv](https://github.com/HansKristian-Work/dxil-spirv)
  — das tatsächliche DXIL→SPIR-V-Werkzeug, das vkd3d-proton nutzt —,
  SPIRV-Cross, naga), um das Rad nicht neu zu erfinden und bewährte
  Designentscheidungen zu übernehmen.
- Herausarbeiten eines realistischen MVP-Umfangs: **eine vollständige
  Grafikpipeline (Rasterizer, Textursampler, Blend-State usw.) ist
  vorerst außerhalb des Scopes.** Begonnen wird mit einer
  Vertikalscheibe, die nur D3D11-Compute-Shader-(DirectCompute)-
  Dispatch abdeckt — ein einfacher Compute-Shader, tatsächlich von
  DXBC/DXIL nach SPIR-V übersetzt, über `open-cuda`s
  `opencuda-vulkan` ausgeführt und auf numerische Übereinstimmung mit
  einer CPU-Referenzimplementierung verifiziert. Die Grafikpipeline-
  Arbeit beginnt erst als nächste Phase, sobald diese Vertikalscheibe
  bewiesen ist.

**Phase 1 und danach (noch nicht begonnen)**:
1. D3D11-Compute-Shader-Vertikalscheibe (DXBC/DXIL→SPIR-V-Übersetzung
   + Vulkan-Dispatch).
2. D3D11-Minimal-Grafikpipeline (Vertex-/Pixel-Shader + einfache
   Rasterisierung).
3. D3D12-Unterstützung (Command Lists, Descriptor Heaps, Root
   Signatures).
4. Android-Unterstützung (Vulkan selbst ist Android-nativ, daher
   sollte sich der Großteil der Linux-Assets wiederverwenden lassen —
   allerdings wird voraussichtlich eine Win32/COM-Emulationsschicht
   〈Wine-Äquivalent〉 nötig sein; in diesem Fall Zusammenarbeit mit/
   Wiederverwendung des Wine-Projekts selbst erwägen).
5. macOS-/iPhone-/iPad-Unterstützung (via MoltenVK, gleicher Ansatz
   wie CrossOver/Whisky; iPhone/iPad setzt für offizielle Distribution
   das Apple Developer Program voraus — native Ausführung auf
   inoffizieller Hardware ist unmöglich, dieselbe Einschränkung wie
   in `dream-os`s Recherche identifiziert).
6. Diverse UNIX-Systeme (BSD usw.) — je nach Vulkan-Unterstützung
   voraussichtlich in der Lage, den Großteil des Linux-Pfads
   wiederzuverwenden (noch nicht untersucht).

**Zur PlayStation-4/5/6/7-Unterstützung (ehrliche Offenlegung, Stand
2026-07-25)**: in der ursprünglichen Vision des Nutzers enthalten,
aber es bestehen **rechtliche/AGB-Bedenken unabhängig von der
technischen Schwierigkeit** — PlayStation-Entwicklungs-SDKs sind
nicht öffentlich und NDA-geschützt, und inoffizielles Reverse
Engineering riskiert, gegen diverse Nutzungsbedingungen und Gesetze
(z. B. DMCA) zu verstoßen. Dieses Projekt vermerkt PS4-7-
Unterstützung im Fahrplan nur als **„zukünftige Ambition“** und nimmt
sie derzeit nicht in den Design-/Implementierungsumfang auf. Ein
Beginn würde eine separate rechtliche Risikobewertung sowie erneute
Rücksprache mit dem Nutzer erfordern.

**Zur Nintendo-Switch-2/3-Unterstützung (hinzugefügt 2026-08-17,
ehrliche Offenlegung)**: Ebenso nur als „zukünftige Ambition“ im
Fahrplan vermerkt. Switch 2 setzt Nintendos offizielle
Entwicklungshardware/NDA voraus (dieselbe rechtliche Sorge wie bei
PS4-7). **Switch 3 ist zum Stand 2026-08-17 von Nintendo nicht
offiziell angekündigt — diese Erwähnung ist lediglich ein Platzhalter
für den Fall einer Ankündigung, nicht auf realen Produktinformationen
basierend** (ausdrücklich vermerkt, um nicht zu übertreiben).

## Basisprojekte (per Nutzeranweisung, 2026-07-25)

- **[open-cuda](https://github.com/aon-co-jp/open-cuda)**: nutzt
  `opencuda-vulkan` (Vulkan-Compute-Ausführungs-Backend, auf echter
  NVIDIA-GT-730-Hardware verifiziert) als Shader-Ausführungs-Backend.
  Plant, die `opencuda-core::GpuDevice`-Abstraktion (alloc/memcpy/
  launch_kernel) unverändert wiederzuverwenden und DXBC/DXIL→SPIR-V-
  übersetzte Kernel als `KernelSource::SpirV` zu übergeben (genaue
  API-Details noch gegen `opencuda-core` zu bestätigen). Unterscheidet
  sich von `opencuda-directx` (ein reines Windows-D3D12-Backend, Phase
  1&2 bereits implementiert) — dieses führt DirectX nativ *auf*
  Windows aus, die entgegengesetzte Richtung zu diesem Projekt (das
  DirectX *auf anderen Betriebssystemen* ausführt).
- **[aruaru-llm](https://github.com/aon-co-jp/aruaru-llm)**: derzeit
  keine direkte technische Abhängigkeit (aruaru-llm ist ein
  LLM-Inferenzdienst, dieses Projekt eine Grafik-API-
  Kompatibilitätsschicht). Die genaue Absicht hinter der
  Nutzererwähnung von aruaru-llm als „Basis“ ist unbestätigt —
  möglicherweise gemeint, dem gemeinsamen „Klon-/Tenant“-
  Dienstmuster zu folgen (z. B. das `TenantRegistry`-artige
  Verwaltungs-API-Muster auf eine Verwaltungsfläche dieses Projekts
  anzuwenden, etwa einen Übersetzungs-Cache-Server). Wird
  aktualisiert, sobald konkrete Integrationspunkte identifiziert sind.

## Entwicklungsrichtlinie (ökosystemweite Zusammenfassung)

- Rust-basierte Implementierung. Nutzt die `windows`-Crate (Windows-
  API) und Vulkan-Bindings (`ash` usw., passend zu dem, was
  `opencuda-vulkan` verwendet).
- „Nie allein aufgrund erfolgreicher Typprüfung/Kompilierung als
  erledigt melden“ — erst nachdem echter DXBC/DXIL-Bytecode
  tatsächlich übersetzt, über echtes Vulkan ausgeführt und die
  numerische Übereinstimmung mit einer CPU-Referenzimplementierung auf
  echter Hardware bestätigt wurde, als „funktioniert“ melden
  (ökosystemweite Disziplin).
- Nicht implementierte/Stub-Funktionen dürfen nie fälschlich
  signalisieren, „unterstützt“ zu sein (folgt `opencuda-directx`s
  `supports_dxil()`-Muster).
- Vor jeder Entscheidung, die neue Dateien, neue Crates, neue
  Repositories oder Namensgebungs-/Platzierungsurteile betrifft, mit
  dem Nutzer Rücksprache halten (Lehre aus 2026-07-23, siehe
  `open-raid-z`s CLAUDE.md).

## HANDOFF (nur jüngste Einträge — siehe CLAUDE.md für das vollständige Protokoll)

- **2026-08-08 (Fortsetzung 12): Laden echter PNG-Dateitexturen
  implementiert, auf echter Windows- und Linux-Hardware verifiziert.**
  Neues Modul `png_loader.rs` (mit der `png`-Crate, Version 0.17.x)
  implementiert `load_png_rgba8`, das RGB-/RGBA-/Graustufen-/
  Graustufen-mit-Alpha-/Palette-Bilder zu RGBA8 normalisiert
  (Palettenexpansion und 16-Bit→8-Bit-Normalisierung werden beide an
  die Transformationsfunktionen der `png`-Crate delegiert, nicht von
  Hand implementiert). Ein echtes Test-Asset
  (`assets/sample_sprite.png`, ein 2x2-Schachbrettmuster mit einem
  halbtransparenten Quadranten) wurde generiert und eingecheckt. Auf
  echter Windows-Hardware (NVIDIA GT 730) und echter Linux-Hardware
  (WSL2 Ubuntu/Mesa llvmpipe) verifiziert: die opaken Quadranten
  stimmen exakt überein, und der halbtransparente Quadrant erzeugt
  genau die alpha-geblendete Kompositfarbe, die von der Standard-
  „Over“-Formel vorhergesagt wird. Gesamter Workspace: `cargo build`/
  `clippy` sauber (0 Warnungen); `cargo test --workspace --release`
  besteht alle 33 Hardware-Tests + 56 Unit-Tests, keine Regressionen.
  Ehrliche Offenlegung: interlaced PNGs und PNGs mit 16 Bit pro Kanal
  sind nicht tatsächlich getestet (nur über die automatische
  Behandlung der `png`-Crate verlassen); `directx-graphics-window`
  (die echte Fenster-Demo) ruft diesen Loader noch nicht auf (noch
  eine 1x1-Volltonfarbtextur).

- **2026-08-08 (Fortsetzung 11): Alpha-Blending (halbtransparente
  Sprites) implementiert, auf echter Windows- und Linux-Hardware
  verifiziert.** Standard-„Over“-Alpha-Blending in der Pipeline-
  Konstruktion von `render_sprites_and_read_back` aktiviert
  (`SRC_ALPHA`/`ONE_MINUS_SRC_ALPHA`/`ADD`). Da `src.a=1.0` numerisch
  äquivalent zu `blend_enable=false` ist, wurde bestätigt, dass alle
  bestehenden Tests für opake Sprites identische Ergebnisse liefern
  (nicht-brechende, additive Änderung). Ein neuer Test verifizierte
  das Ergebnis der Standard-„Over“-Formel auf echter Hardware, exakte
  Übereinstimmung auf beiden Betriebssystemen. Ehrliche Offenlegung:
  dieses Blending ist nur im Offscreen-Pfad
  `render_sprites_and_read_back` aktiviert — `directx-graphics-window`
  (die echte Fenster-Demo) wurde noch nicht entsprechend
  aktualisiert; nur „Over“-Blending wird unterstützt (keine
  additiven/multiplikativen Modi); das Zusammenspiel von
  Tiefentest/Blending ist ungetestet (dieser Durchgang hat keinen
  Tiefenpuffer, nur 2D-Sprites).

Für die vollständige sitzungsweise Historie (einschließlich des
früheren D3D11-Grafikpipeline-Meilensteins, der DXIL-Vertikalscheibe,
der echten Fenster+Swapchain+Tastatureingabe-Game-Loop und der vielen
Erweiterungen der grenzwertgeprüften DXBC/DXIL-Kettenlänge) siehe
[CLAUDE.md](CLAUDE.md) (Japanisch, maßgeblich).
