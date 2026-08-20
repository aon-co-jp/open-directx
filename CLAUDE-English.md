# Design philosophy & development policy & environment rules (open-directx) — condensed

> **Note**: This is a condensed translation of the current state. The
> full historical HANDOFF log (dozens of entries since 2026-07-25)
> remains Japanese-only in CLAUDE.md for brevity — see there for
> per-session detail.

Work drive: `F:\runo`. This section follows the practice of copying
the relevant section from [`open-raid-z`](https://github.com/aon-co-jp/open-raid-z)'s
`CLAUDE.md` into each project as a reference. GitHub repo:
[aon-co-jp/open-directx](https://github.com/aon-co-jp/open-directx).

**Development start: 2026-07-25** (the empty GitHub repo itself was
created earlier, on 2026-07-01).

## Open task (added 2026-08-06, not yet started)

There is a plan, per user instruction, to bring Toshiba's Simulated
Bifurcation Machine technology (pseudo-quantum computing) and
DeepSeek's techniques (MLA, DeepSeekMoE, FP8 mixed-precision
training, researched from papers/implementation blogs, not just news)
into 8 repositories, including `open-directx`. No concrete
optimization target has been identified yet for this repo — the
investigation is deferred to a future session.

## Role of this project

A cross-platform DirectX (D3D9/10/11/12) compatibility layer, aiming
to run existing apps/games written for the Windows-only DirectX API
on Linux, Android, and eventually macOS and the PlayStation family.

On 2026-07-25, per explicit user decision, the project committed to
pursuing a **genuine backward-compatibility layer** (running
unmodified Windows DirectX binaries/shaders on other OSes), rather
than the earlier alternative of "providing a DirectX-like API on top
of Vulkan as a shared foundation."

## Correction to the technical positioning (2026-07-25, important)

An earlier assessment (2026-07-23) judged that DXVK/vkd3d-proton/
MoltenVK only translate one-way (DirectX → Vulkan/Metal) with no real
example of the reverse direction, and were therefore a poor technical
fit. This conflated two different axes, and was corrected:

- DXVK/vkd3d-proton (the technology behind Valve's Proton / Linux
  Steam's DirectX game compatibility) and MoltenVK-based CrossOver/
  Whisky (macOS) are real, working examples of exactly the backward
  compatibility the user wants: running real existing DirectX
  binaries/games (Windows-only API) unmodified on Linux/macOS.
- The **direction of translation** ("translate DirectX calls to
  Vulkan calls") and the **direction of the end-user experience**
  ("does a Windows-targeted DirectX app actually run on Linux/macOS")
  are separate axes — the former targeting Vulkan does not prevent
  the latter from correctly achieving "DirectX on another OS."
- Therefore, using Vulkan as the internal execution backend is not in
  conflict with the goal of a genuine backward-compatibility layer —
  DXVK etc. are exactly this precedent. This project takes the same
  approach (intercepting D3D API calls + runtime-translating DXBC/DXIL
  shader bytecode → executing via Vulkan).

## Scope and honest roadmap

**Phase 0 (current, design/research stage)**:
- Investigating the structure of DXBC/DXIL (DirectX shader bytecode
  formats).
- Studying the architecture of existing OSS implementations (DXVK,
  vkd3d-proton, [dxil-spirv](https://github.com/HansKristian-Work/dxil-spirv)
  — the actual DXIL→SPIR-V tool vkd3d-proton uses —, SPIRV-Cross,
  naga) to avoid reinventing the wheel and to build on proven design
  decisions.
- Carving out a realistic MVP scope: **a full graphics pipeline
  (rasterizer, texture sampler, blend state, etc.) is out of scope
  for now.** Starting with a vertical slice covering only D3D11
  Compute Shader (DirectCompute) dispatch — one simple compute
  shader, actually translated from DXBC/DXIL to SPIR-V, executed via
  `open-cuda`'s `opencuda-vulkan`, and verified to numerically match a
  CPU reference implementation. Graphics-pipeline work only begins as
  the next phase once this vertical slice is proven.

**Phase 1 and beyond (not yet started)**:
1. D3D11 Compute Shader vertical slice (DXBC/DXIL→SPIR-V translation
   + Vulkan dispatch).
2. D3D11 minimal graphics pipeline (vertex/pixel shaders + basic
   rasterization).
3. D3D12 support (command lists, descriptor heaps, root signatures).
4. Android support (Vulkan itself is Android-native, so most of the
   Linux assets should be reusable — though a Win32/COM emulation
   layer 〈Wine-equivalent〉 will likely be needed; in that case,
   consider collaborating with/reusing the Wine project itself).
5. macOS / iPhone / iPad support (via MoltenVK, the same approach as
   CrossOver/Whisky; iPhone/iPad requires the Apple Developer Program
   for official distribution — native execution on unofficial
   hardware is impossible, the same constraint identified in
   `dream-os`'s research).
6. Various UNIX systems (BSD etc.) — likely able to reuse most of the
   Linux path depending on Vulkan support (not yet investigated).

**On PlayStation 4/5/6/7 support (honest disclosure, as of
2026-07-25)**: included in the user's original vision, but there are
**legal/terms-of-service concerns independent of technical
difficulty** — PlayStation development SDKs are non-public and
NDA-protected, and unofficial reverse engineering risks violating
various terms of service and laws (e.g. DMCA). This project notes
PS4-7 support in the roadmap only as a **"future ambition"** and does
not currently include it in the design/implementation scope. Starting
it would require a separate legal risk assessment and re-confirmation
with the user.

**On Nintendo Switch 2/3 support (added 2026-08-17, honest
disclosure)**: Likewise noted only as a "future ambition" in the
roadmap. Switch 2 requires Nintendo's official dev hardware/NDA (the
same legal concern as PS4-7). **Switch 3 has not been officially
announced by Nintendo as of 2026-08-17 — this mention is only a
placeholder for if it is announced, not based on any real product
information** (stated explicitly to avoid overclaiming).

## Base projects (per user instruction, 2026-07-25)

- **[open-cuda](https://github.com/aon-co-jp/open-cuda)**: uses
  `opencuda-vulkan` (Vulkan compute execution backend, verified on
  real NVIDIA GT730 hardware) as the shader execution backend. Plans
  to reuse the `opencuda-core::GpuDevice` abstraction (alloc/memcpy/
  launch_kernel) unmodified, passing DXBC/DXIL→SPIR-V-translated
  kernels as `KernelSource::SpirV` (exact API details still to be
  confirmed against `opencuda-core`). Distinct from `opencuda-directx`
  (a Windows-only D3D12 backend, Phases 1&2 already implemented) —
  that one runs DirectX natively *on* Windows, the opposite direction
  from this project (which runs DirectX *on other OSes*).
- **[aruaru-llm](https://github.com/aon-co-jp/aruaru-llm)**: no direct
  technical dependency currently (aruaru-llm is an LLM inference
  service, this project is a graphics-API compatibility layer). The
  exact intent behind the user's mention of aruaru-llm as a "base" is
  unconfirmed — possibly meaning to follow the shared "clone/tenant"
  service pattern (e.g. applying the `TenantRegistry`-style management
  API pattern to a management surface of this project, such as a
  translation-cache server). Will be updated once concrete integration
  points are identified.

## Development policy (ecosystem-wide summary)

- Rust-based implementation. Uses the `windows` crate (Windows API)
  and Vulkan bindings (`ash` etc., matching what `opencuda-vulkan`
  uses).
- "Never report done based solely on passing type-checks/compilation"
  — only report "working" after real DXBC/DXIL bytecode has actually
  been translated, executed via real Vulkan, and confirmed to
  numerically match a CPU reference implementation on real hardware
  (ecosystem-wide discipline).
- Unimplemented/stub functionality must never falsely signal as
  "supported" (follows `opencuda-directx`'s `supports_dxil()`
  pattern).
- Before any decision involving new files, new crates, new
  repositories, or naming/placement judgment calls, check with the
  user first (a lesson from 2026-07-23, see `open-raid-z`'s
  CLAUDE.md).

## HANDOFF (most recent entries only — see CLAUDE.md for the full log)

- **2026-08-20: Fixed-size 2x2 GEMM compute-shader vertical slice
  implemented and verified on real hardware**, as a first step toward
  hardware acceleration for LLM inference. Added
  `translate_gemm2x2_shader` to `directx-shader-translate`: a HLSL
  2x2x2 GEMM shader was compiled for real with `fxc.exe`, its DXBC
  decoded, translated to SPIR-V, and dispatched on real Vulkan
  (NVIDIA GT 730) reusing `opencuda-vulkan`'s kernel contract. GPU
  output matched the CPU-reference matrix product exactly. Honest
  scope: fixed 2x2 size only — general variable-size GEMM (with
  loops), Attention ops, and wiring into `aruaru-llm` remain out of
  scope for this pass.
- **2026-08-19: Investigated whether an auto-update mechanism (like
  `open-english`'s GitHub-Releases self-update) could be added —
  deferred.** The only `fn main` binary in the workspace is
  `directx-graphics-window` (a visual-inspection Breakout-style demo)
  with no persistent service or `/healthz` endpoint, so the pattern
  doesn't apply.
- **2026-08-19: Investigated the claim that "lacking a persistent
  background service is a defect" — conclusion: it is not.** Real
  Microsoft DirectX ships as runtime DLLs that apps dynamically link
  against, not as an independent background service; the current
  design (library + demo binary only) matches upstream DirectX's
  architecture.
- **2026-08-20: Recorded an `open-cg-cad` idea memo (not started).** A
  future 3D modeling tool concept with motion capture, AI-chat-driven
  natural-language spec changes, and DirectX/OpenGL/WebGL/WebGPU
  support.
- **2026-08-20 (continued): Recorded an "AI real estate × AI
  construction" concept** combining `open-cg-cad` with `aruaru-llm` to
  auto-generate 3D models (houses, buildings, bridges, tunnels, etc.)
  from land/site data; later updated to also cover maglev trains and
  CPU/GPU/NPU semiconductor CAD models, with an honest note about the
  difficulty of covering architecture, civil engineering, rolling
  stock, and semiconductor design in one system. Idea memo only, not
  started.

- **2026-08-08 (continued, part 12): Real PNG file texture loading
  implemented, verified on real Windows and Linux hardware.** New
  module `png_loader.rs` (using the `png` crate, version 0.17.x)
  implements `load_png_rgba8`, which normalizes RGB/RGBA/grayscale/
  grayscale-with-alpha/palette images to RGBA8 (both palette
  expansion and 16-bit→8-bit normalization are delegated to the `png`
  crate's transform functions, not hand-implemented). A real test
  asset (`assets/sample_sprite.png`, a 2x2 checkerboard pattern with
  one semi-transparent quadrant) was generated and checked in.
  Verified on real Windows hardware (NVIDIA GT 730) and real Linux
  hardware (WSL2 Ubuntu/Mesa llvmpipe): the opaque quadrants match
  exactly, and the semi-transparent quadrant produces exactly the
  alpha-blended composite color predicted by the standard "over"
  formula. Whole workspace: `cargo build`/`clippy` clean (0 warnings);
  `cargo test --workspace --release` passes all 33 hardware tests +
  56 unit tests, no regressions. Honest disclosure: interlaced PNGs
  and 16-bit-per-channel PNGs are not actually tested (relying only on
  the `png` crate's automatic handling); `directx-graphics-window`
  (the real windowed demo) does not yet call this loader (still a 1x1
  solid-color texture).

- **2026-08-08 (continued, part 11): Alpha blending (semi-transparent
  sprites) implemented, verified on real Windows and Linux hardware.**
  Standard "over" alpha blending enabled in `render_sprites_and_read_
  back`'s pipeline construction (`SRC_ALPHA`/`ONE_MINUS_SRC_ALPHA`/
  `ADD`). Since `src.a=1.0` is numerically equivalent to
  `blend_enable=false`, confirmed all existing opaque-sprite tests
  produce identical results (a non-breaking, additive change). A new
  test verified the standard "over" formula's result on real hardware,
  exact match on both OSes. Honest disclosure: this blending is only
  enabled in the offscreen path `render_sprites_and_read_back` —
  `directx-graphics-window` (the real windowed demo) has not yet been
  updated to match; only "over" blending is supported (no additive/
  multiplicative modes); the interaction of depth testing/blending is
  untested (this pass has no depth buffer, only 2D sprites).

For the full session-by-session history (including the earlier D3D11
graphics-pipeline milestone, the DXIL vertical slice, the real
window+swapchain+keyboard-input game loop, and the many extensions of
boundary-checked DXBC/DXIL chain length), see [CLAUDE.md](CLAUDE.md)
(Japanese, authoritative).

---

Other languages: [日本語 (原文、完全なHANDOFF履歴)](CLAUDE.md) ·
[Deutsch](CLAUDE-German.md) · [Italiano](CLAUDE-Italian.md) ·
[Français](CLAUDE-French.md) · [Русский](CLAUDE-Russian.md) ·
[Українська](CLAUDE-Ukrainian.md) · [עברית](CLAUDE-Hebrew.md) ·
[فارسی](CLAUDE-Persian.md) · [العربية](CLAUDE-Arabic.md)
