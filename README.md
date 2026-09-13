# open-directx

*日本語*: [README-Japanese.md](README-Japanese.md) ·
*Other languages*: [Deutsch](README-German.md) · [Italiano](README-Italian.md) ·
[Français](README-French.md) · [Русский](README-Russian.md) ·
[Українська](README-Ukrainian.md) · [עברית](README-Hebrew.md) ·
[فارسی](README-Persian.md) · [العربية](README-Arabic.md)

> 📌 **最近の更新(2026-09-13続き)**: MEDの2次元近傍参照(`x-1`/`y-1`)
> を、`vector_add`等と同じ「1つの既知コンパイル結果専用」方式の新規
> モジュール`med2d.rs`として実装した(実`fxc.exe`出力の29命令
> オペコード列を検証した上でSPIR-Vを再構築)。8x9画像(境界含む全72
> ピクセル)で**実GT730ハードウェア上でCPU参照実装と完全一致**。
> レンジコーダーの並列レーン数も128→256→512→1024まで拡張し、
> いずれも実機で完全一致(GT730の`maxComputeWorkGroupInvocations`
> 1536に対し1024が実質的な上限と判断、ここで打ち止め)。速度計測も
> 実施し、**逐次版≈2.06ms/回に対し並列版≈2.31ms/回**(並列版の方が
> 遅い)という結果を正直に報告——パイプライン構築オーバーヘッドが
> 支配的と考えられる。open-cpuのAVX2/AVX512 gather命令との関連、
> 東芝SBM(`open-cuda`に既存の`sbm_ising`を確認)、DeepSeekのMLA
> (低ランクKVキャッシュ圧縮)、複数GPUプーリング(aggregation/
> partitioning)についても調査した(いずれも調査のみ、新規実装は
> 今回無し)。詳細は[PORTING.md](PORTING.md)・[CLAUDE.md](CLAUDE.md)
> 参照。
>
> *English*: Implemented MED's 2D neighbor addressing (`x-1`/`y-1`) as
> a new dedicated module `med2d.rs` (same "one known-compiled-shader
> shape" approach as `vector_add` etc. — verifies the real `fxc.exe`
> output's 29-opcode sequence, then rebuilds the SPIR-V). An 8×9 image
> (72 pixels, borders included) **matched the CPU reference exactly on
> real GT730 hardware**. Also extended the range coder's parallel lane
> count to 128/256/512/1024, all real-hardware-verified (GT730's
> `maxComputeWorkGroupInvocations` of 1536 makes 1024 the practical
> ceiling for this single-workgroup design). Performed the previously-
> unmeasured speed comparison: **serial ≈2.06 ms/call vs. parallel
> ≈2.31 ms/call — parallel was slower**, honestly reported (likely
> dominated by per-dispatch Vulkan pipeline rebuild overhead, not the
> actual computation). Also researched (no new implementation) the
> connection to `open-cpu`'s AVX2/AVX512 gather instructions, Toshiba's
> SBM (already implemented in `open-cuda` as `sbm_ising`), DeepSeek's
> real low-rank compression technique (MLA), and multi-GPU pooling
> terminology. See [PORTING.md](PORTING.md) / [CLAUDE.md](CLAUDE.md).

> 📌 **最近の更新(2026-09-13)**: 前回「レンジコーダーは32レーン
> subgroup shuffleで並列化される」と記録したが、**FFmpeg本家の実ソース
> (`libavcodec/vulkan/rangecoder.glsl`)を実際にfetchして読んだところ
> 誤りだったと判明**——実際の機構はワークグループ共有メモリ(`shared`)
> +`barrier()`であり、subgroup shuffleは使われていなかった(前回の
> shuffle実証自体は無駄ではないが、FFv1の実機構ではなかった)。この
> 訂正の上で、実際の機構に忠実な`build_range_decoder_parallel_kernel`
> を新規実装: 32本のinvocationが並列に自分のコンテキスト状態を共有
> メモリへ書く→バリア→lane0だけが逐次`get_rac`を実行→バリア→32本が
> 並列に書き戻す、という設計。**実GT730ハードウェア上でCPU参照実装と
> 32コンテキスト分の復号ビット・最終状態の両方で完全一致**した。
> さらにユーザーの指示(「32レーン成功後は64レーンに挑戦」)により
> `context_size`をパラメータ化し、**64レーンでも実GT730ハードウェア上で
> 完全一致**することを確認——ワークグループバリア方式がGT730の
> subgroup幅(32)を超えても正しく機能することを実証した。ワークスペース
> 全体で回帰無し。詳細は[PORTING.md](PORTING.md)・
> [CLAUDE.md](CLAUDE.md)参照。
>
> *English*: The earlier note that the range coder parallelizes via
> 32-lane subgroup shuffle was **wrong** — after actually fetching and
> reading FFmpeg's real source
> (`libavcodec/vulkan/rangecoder.glsl`), the real mechanism is
> **workgroup shared memory (`shared`) + `barrier()`**, not subgroup
> shuffle (the earlier shuffle proof remains a valid separate result,
> just not FFv1's actual mechanism). Implemented
> `build_range_decoder_parallel_kernel` faithfully to this real design:
> all invocations write their own context state into shared memory in
> parallel, a barrier, lane 0 alone runs the serial `get_rac` loop, a
> barrier, then all invocations write results back in parallel —
> **matched the CPU reference bit-for-bit and state-for-state across 32
> contexts on real GT730 hardware**. Per the user's own roadmap
> ("after 32 lanes succeed, try 64"), parameterized `context_size` and
> confirmed **64 lanes also match exactly on real GT730 hardware** —
> proving the barrier-based design scales past this GPU's native
> 32-wide subgroup. Zero regressions. See [PORTING.md](PORTING.md) /
> [CLAUDE.md](CLAUDE.md) for details.

> 📌 **最近の更新(2026-09-12)**: H.264/H.265/HEVCの自前シェーダー実装は
> 非推奨・クローズを再確認し、代わりに**FFv1**を現実的な次の目標として
> 特定、同日中に5つの実装を完成させた: (1) MED予測器の比較器(`max`/
> `min`)、(2) `open-cuda`への汎用Nバッファディスパッチ追加でGチャンネル
> (4バッファ)を**実GT730ハードウェアでの数値検証へ格上げ**、(3) MED
> 予測器本体完成(`fxc.exe`が3分岐if/else if/elseを分岐命令無しで
> `ge`+`movc`だけに平坦化することを発見、`RegExpr::Ge`/`Select`で対応、
> 3分岐全てを実機検証)、(4) レンジコーダーの前提条件である32レーン
> subgroup shuffleを`OpGroupNonUniformShuffle`で実機実証、(5)
> **レンジコーダーの状態遷移テーブル+`get_rac`本体を実装**——RFC 9043
> から256要素の`default_state_transition`テーブルを実際に転記し、
> `rspirv`で直接組み立てたSPIR-Vループとして実装したところ、**実GT730
> ハードウェア上でCPU参照実装と32シンボル分ビット単位で完全一致**した。
> MEDの2次元近傍参照(`x-1`/`y-1`)は実際にコンパイルして必要な命令
> (`IMul`/`UDiv`/`IMad`/`Iadd`/`And`+実分岐)を調査したが、現在の
> デコーダを大きく超える拡張が必要と判明したため実装は次回へ持ち越し。
> ワークスペース全体で回帰無し。次: レンジコーダーの32レーン並列化
> (土台2つは完成済み)、MEDの2次元化、`put_symbol`/`get_symbol`本体。
> 詳細は[PORTING.md](PORTING.md)・[CLAUDE.md](CLAUDE.md)参照。
>
> *English*: Re-confirmed H.264/H.265/HEVC shader-codec work as not
> recommended/closed on GT730, identified **FFv1** as the realistic next
> target, and completed five real pieces the same day: (1) MED predictor
> comparator (`max`/`min`); (2) generic N-buffer dispatch added to
> `open-cuda`, upgrading the G-channel kernel to **real numeric
> verification on GT730 hardware**; (3) the MED predictor itself,
> completed (`fxc` flattens the 3-way branch into branch-free `ge`+
> `movc`, handled via `RegExpr::Ge`/`Select`, verified on hardware
> exercising all three branches); (4) the range coder's 32-lane subgroup
> shuffle prerequisite, proven with `OpGroupNonUniformShuffle` on real
> hardware; (5) **the range coder's state-transition table and `get_rac`
> itself, implemented** — transcribed RFC 9043's 256-entry
> `default_state_transition` table and hand-built a SPIR-V loop that
> **matched the CPU reference bit-for-bit across 32 symbols on real
> GT730 hardware**. MED's 2D neighbor addressing (`x-1`/`y-1`) was
> compiled and researched (`IMul`/`UDiv`/`IMad`/`Iadd`/`And` plus real
> branching) but found to need a materially larger decoder extension, so
> implementation is deferred to next session. Zero regressions. Next:
> parallelize the range coder across 32 lanes (both prerequisites now
> done), MED's 2D indexing, and `put_symbol`/`get_symbol` itself. See
> [PORTING.md](PORTING.md) / [CLAUDE.md](CLAUDE.md) for details.

> 📌 **最近の更新(2026-09-03)**: ユーザー指示「open-directx/open-cuda/
> aruaru-llmで、今後32GB VRAM級のNVIDIA/AMD/Intel GPUを想定し、
> F16/F32/F64、さらにF128まで見据えて開発する」への対応として、
> DXIL(LLVMビットコード型システム)デコーダが`half`/`min16float`
> (HLSL SM6.2+のネイティブ16bit型)に対応する`TYPE_CODE_HALF`
> (LLVMコード10)を未知の型として取りこぼしていた実バグを発見・修正。
> **正直な開示**: F128(四倍精度)はHLSL/DXILに対応する構文自体が
> 存在しないため(HLSLの数値型は`min16float`/`half`/`float`/`double`
> まで)対象外と判断した——実際の多精度計算カーネルは
> `open-cuda/OmniGPU-Design.md`§13参照。このリポジトリには32GB VRAM/
> GPUベンダー判定に関わるコードパス自体が無い(診断用のベンダー名
> 参照のみ)。詳細は[CLAUDE.md](CLAUDE.md)参照。
>
> *English*: Per user instruction to target 32GB-VRAM-class NVIDIA/AMD/
> Intel GPUs and support F16/F32/F64/F128 going forward, fixed a real
> bug where the DXIL (LLVM bitcode) type decoder failed to recognize
> `TYPE_CODE_HALF` (LLVM code 10) — the type HLSL's `half`/`min16float`
> (SM6.2+ native 16-bit types) compiles to — silently falling through to
> an unknown-type bucket. **Honest scoping**: F128 has no HLSL/DXIL
> construct at all (HLSL's numeric types stop at `double`), so it's out
> of scope here — see `open-cuda/OmniGPU-Design.md` §13 for the actual
> multi-precision compute work. This repo has no VRAM-size or vendor-ID
> branching to begin with (only diagnostic vendor-name lookups). See
> [CLAUDE.md](CLAUDE.md) for details.
>
> 📌 **最近の更新(2026-08-08、2Dスプライト描画プロトタイプ)**:
> ユーザー提案「dream-os/Linux上でopen-directx経由でGT730のPCでGAME…の
> 試作品を開発」を受け、まず2Dスプライト描画に絞って着手。テクスチャ
> サンプリング(`Texture2D.Sample`)初実装→複数スプライト/スプライト
> シート対応→ゲームループ(位置更新+跳ね返り物理)→**実ウィンドウ+
> 実Vulkanスワップチェーン+実キーボード入力**(`directx-graphics-window`
> クレート新設、ユーザー自身が実行し「パドルが動いて玉を弾き返した」と
> 目視確認済み)→アルファブレンド(半透明スプライト)→実PNGファイル
> からのテクスチャ読み込み、という一連の増分をすべてWindows実機
> (NVIDIA GT 730)、一部はLinux実機(WSL2 Ubuntu)でも検証した。次の候補:
> 複数の動くスプライト+衝突判定、ウィンドウリサイズ対応。詳細は
> [CLAUDE.md](CLAUDE.md)参照。
>
> *English*: Following the user's proposal to build game/mining/LLM
> prototypes for the GT 730 via open-directx on dream-os/Linux, started
> narrowly with a 2D sprite-rendering prototype: first texture sampling
> support, then multi-sprite/sprite-sheet support, a game loop
> (position update + bounce physics), a **real window + real Vulkan
> swapchain + real keyboard input** (new `directx-graphics-window`
> crate — the user ran it themselves and confirmed "the paddle moved
> and hit the ball back"), alpha blending, and loading textures from
> real PNG files. All verified on real Windows hardware (NVIDIA GT 730),
> some also on real Linux hardware (WSL2 Ubuntu). Next candidates:
> multiple moving sprites with collision detection, window resize
> support. See [CLAUDE.md](CLAUDE.md) for details.

> 📌 保留タスク(2026-08-06): 東芝SBM・DeepSeek技術の組み込み構想あり(dream-os等8リポジトリ対象)。詳細は[CLAUDE.md](CLAUDE.md)参照。

> 📌 **最近の更新(2026-08-08)**: 境界チェック付き7項チェーンについて
> DXBC側のみでDXIL側が無いという非対称を解消——`vector_add_mul_div_sub_
> add_mul_div_chain7_bounded_dxil.hlsl`を新規に`dxc.exe`で実コンパイルし、
> NVIDIA GT730実機でCPU参照実装との数値一致・境界チェックの動作を確認
> 済み(ワークスペース全体50単体テスト+実機テスト22本すべてgreen、
> 警告0件)。詳細は[CLAUDE.md](CLAUDE.md)参照。
>
> *English*: Closed the DXBC-only vs. DXIL-missing asymmetry for the
> boundary-checked 7-term chain — added and real-`dxc.exe`-compiled
> `vector_add_mul_div_sub_add_mul_div_chain7_bounded_dxil.hlsl`, verified
> on real NVIDIA GT730 hardware (matches the CPU reference, boundary
> check confirmed; full workspace: 50 unit tests + 22 real-hardware
> tests all green, zero warnings). See [CLAUDE.md](CLAUDE.md) for
> details.

> 📌 **最近の更新(2026-08-07)**: 境界チェック付きDXBC/DXILチェーンを
> 6項へ拡張し、NVIDIA GT730実機で動作確認。dream-os/open-cuda/
> aruaru-llmとの連携強化(SBM/DeepSeek移植等)を検討したが、既存の
> DXBC/DXILチェーン生成ロジックへの深い理解を伴わない拡張は数値的な
> 誤りを見逃すリスクがあると判断し、コード変更は行わず調査結果を
> [CLAUDE.md](CLAUDE.md)へ正直に記録した。
>
> *English*: Extended the boundary-checked DXBC/DXIL chain to 6 terms,
> verified on real NVIDIA GT730 hardware. Investigated deeper
> integration with dream-os/open-cuda/aruaru-llm (SBM/DeepSeek
> transplant) but decided against guessing extensions to the DXBC/DXIL
> chain logic without deep domain understanding — no code changed there,
> findings honestly recorded in [CLAUDE.md](CLAUDE.md).

> **Updated 2026-07-25**: The dev-policy file (`CLAUDE.md`) heading was
> renamed from "Development Policy & Dev Environment Rules" to
> "Design Philosophy & Development Policy & Dev Environment Rules",
> to more clearly separate the project's design philosophy (what we
> value), development policy (how we work), and dev environment rules
> (concrete operational conventions). See `CLAUDE.md` for details.


A cross-platform DirectX (D3D9/10/11/12) compatibility layer — in the
spirit of DXVK / vkd3d-proton — aiming to run unmodified Windows
DirectX applications on Linux (and eventually Android/macOS) by
translating DXBC/DXIL shader bytecode to SPIR-V and dispatching through
an existing Vulkan compute backend ([open-cuda](https://github.com/aon-co-jp/open-cuda)'s
`opencuda-vulkan`).

See [`CLAUDE.md`](CLAUDE.md) for the full design rationale, honest
scope/roadmap, and session HANDOFF log — this README only summarizes
current, verified state.

### Platform & vendor support matrix (added 2026-07-27, honest disclosure)

DirectX itself is a Windows/Xbox-only API — "cross-platform" here means
DXBC/DXIL bytecode is translated to SPIR-V and dispatched via Vulkan,
which is what actually reaches non-Windows platforms. No `cfg(windows)`
or other platform-gating exists in this repo's own code today (the DXBC
parser, SPIR-V codegen, and `directx-graphics-vulkan` are all plain,
platform-neutral Rust + `ash`), so build/test portability follows
Vulkan's own reach:

| Platform | Path | Status |
|---|---|---|
| Windows | native Vulkan | **Verified on real hardware** (this repo's dev machine, NVIDIA GeForce GT 730) |
| Linux | native Vulkan | **Verified 2026-08-18** — `cargo build --workspace --release` and `cargo test --workspace --release` both succeed on real Linux (WSL2 Ubuntu), all real-hardware Vulkan tests pass (0 failures across the full workspace). Honest disclosure: the Vulkan device backing these tests is Mesa's `llvmpipe` (a software/CPU rasterizer), not a discrete/integrated GPU — so this confirms the code path works correctly on Linux, but does not exercise a real GPU driver on Linux the way the Windows GT730 runs do. |
| Android | native Vulkan | `open-cuda` has verified `aarch64-linux-android` cross-compilation succeeds (per its CLAUDE.md); real-device execution (`vkCreateInstance` on an actual phone) is still pending |
| macOS | Vulkan via [MoltenVK](https://github.com/KhronosGroup/MoltenVK) (translates to Metal) | Not yet attempted — MoltenVK is a translation layer, not native Vulkan, so this is a weaker guarantee than Linux/Android |
| iOS / iPadOS (added 2026-08-17) | Vulkan via MoltenVK (translates to Metal) | Not yet attempted. **Same MoltenVK caveat as macOS applies** — Vulkan does not run natively on iOS/iPadOS, only through this translation layer, so parity with the Windows/Vulkan-native path is not guaranteed until actually tried on a device. Also requires the Apple Developer Program for official distribution. |
| Various UNIX/BSD (added 2026-08-17) | native Vulkan, likely | Unresearched — Vulkan support varies by distribution/driver; expected to reuse most of the Linux path once investigated |
| Sony PlayStation 4/5/6/7 | n/a | Explicitly out of scope for now — see the "PlayStation family targets" note below and `CLAUDE.md` |
| Nintendo Switch 2 / 3 (added 2026-08-17) | n/a | Same "future ambition, deferred pending official SDK/NDA" status as PlayStation. **Switch 3 has not been officially announced by Nintendo as of 2026-08-17 — its inclusion here is only a placeholder for if/when it is announced, not based on real product information.** |

GPU vendor coverage (PCI vendor ID matching, consistent across this repo
and `open-cuda`: NVIDIA `0x10DE`, AMD `0x1002`/`0x1022`, Intel `0x8086`):

| Vendor | Status |
|---|---|
| NVIDIA | **Verified on real hardware** (GeForce GT 730) |
| AMD | Vendor-ID matching code exists and type-checks, but has **never run against real AMD hardware** in this environment — treat as unverified |
| Intel | Same as AMD: code exists, **never verified on real Intel GPU hardware** |

No fix is needed to make these three vendor IDs *detectable* — the code
is already correct and identical across `open-directx`/`opencuda-vulkan`/
`opencuda-directx`. What's missing is real AMD/Intel hardware to actually
exercise that code path, which this development environment does not
have.

## Current state (2026-07-27, latest: gradient interpolation, GPU vendor diagnostics, chain sub/div)

Three increments landed on top of the D3D11 minimal graphics pipeline and
DXBC chain-class work below, all verified on this machine's real NVIDIA
GT 730: (1) `render_gradient_triangle_and_read_back` — the graphics
pipeline can now assign a distinct color per vertex (not just the
degenerate uniform-color case), verified via a partition-of-unity
invariant check on real hardware readback pixels. (2)
`enumerate_graphics_devices()` — closes a diagnostic parity gap where
`open-cuda`'s Compute path had vendor-ID detection but the Graphics path
here had none; standalone, no new dependency on `opencuda-vulkan`. (3)
`decode_chain_shape` now supports `sub`/`div` (previously explicitly
rejected as unverifiable) — a new shader (`vector_sub_div_chain.hlsl`)
was actually compiled with `fxc.exe` and its SHEX dump used to confirm
the exact operand ordering, then verified end-to-end against a CPU
reference on real hardware. See `CLAUDE.md` HANDOFF (2026-07-27 entries)
for the full account.

## Current state (2026-07-25, latest: DXIL vertical slice complete on real hardware)

The D3D12/DXIL compute-shader vertical slice now reaches full parity
with the D3D11/DXBC one: `vector_add.dxil` (real `dxc.exe -T cs_6_0`
output) is decoded end-to-end (container -> LLVM bitstream ->
type table -> instructions -> all 7 `Call` records disambiguated to
real `dx.op.*` meaning) and translated to real SPIR-V
(`directx_shader_translate::translate_dxil_vector_add_to_spirv`), which
`tests/vector_add_dxil_real_vulkan.rs` dispatches on this machine's
real NVIDIA GT 730 and verifies numerically matches the CPU reference
`a[i]+b[i]`. This is still one known shader shape only, not a general
SM6.0 decoder — see "Not implemented (honest scope)" below for the
precise boundary. The SPIR-V workgroup size is now genuinely extracted
from DXIL's `METADATA_BLOCK` (`dx.entryPoints` -> `ShaderProperties` ->
`NumThreads`), not hardcoded — see the 2026-07-25 "continued 9" HANDOFF
entry in `CLAUDE.md` for the full account, and "continued 7" for the
original vertical-slice achievement this closed a known gap in.

## Current state (2026-07-25, continued: DXIL bitstream-level parsing + D3D11 VS/PS DXBC parsing)

Two new pieces of work landed on top of the Phase 1 compute-shader
vertical slice below:

- **DXIL (D3D12/SM6+) — actual bytes parsed, container/bitstream level
  only.** `crates/directx-shader-translate/src/dxil.rs`
  (`parse_dxil_container`) parses a real `dxc.exe -T cs_6_0`-compiled
  DXBC container (`shaders/vector_add_dxil.hlsl` ->
  `shaders/vector_add.dxil`, produced by `tools/compile-dxbc-shaders.ps1`):
  extracts the `DXIL` chunk's `DxilProgramHeader`/`DxilBitcodeHeader`
  (shader kind, SM6.0, DXIL version) via the existing `dxbc` crate, then
  hands the raw LLVM bitcode payload to the `llvm-bitcode` crate (newly
  added dependency, generic LLVM bitstream reader with no DXIL-specific
  knowledge) to actually decode the block/record tree. Confirmed against
  real bytes: the LLVM wrapper magic `BC\xC0\xDE`, a single top-level
  `MODULE_BLOCK` (id 8), and standard LLVM sub-blocks inside it —
  `TYPE_BLOCK_ID_NEW`(17), `PARAMATTR_GROUP_BLOCK`(10),
  `PARAMATTR_BLOCK`(9), `CONSTANTS_BLOCK`(11), `FUNCTION_BLOCK`(12, x5 —
  one per basic block of `main`), `VALUE_SYMTAB_BLOCK`(14),
  `METADATA_BLOCK`(15, x2). **Update (2026-07-25, continued, D3D12
  track)**: type-table resolution and coarse instruction decoding have
  since been added (`resolve_type_table`/`decode_function_instructions`
  in the same file), applying LLVM's documented `TYPE_BLOCK`/`FUNC_CODE`
  record tables to the real `vector_add.dxil` bytes — confirmed a
  22-entry type table including `Float` and
  `StructNamed{"class.RWStructuredBuffer<float>"}`, and a real
  instruction sequence (`DeclareBlocks -> Call*5 -> ExtractValue -> Call
  -> ExtractValue -> BinOp -> Call -> Ret`). **Update (2026-07-25,
  continued 6)**: all 7 `Call` records are now disambiguated.
  `resolve_vector_add_dxil_calls` resolves `VALUE_SYMTAB_BLOCK` function
  names (found via `Record::take_payload()`, not `fields()` — a real gap
  in the previous entry's understanding of the crate) and hand-decodes
  LLVM's relative-value operand encoding (verified by hand against the
  real bytes), giving `[CreateHandle{range_id:2}, CreateHandle{range_id:1},
  CreateHandle{range_id:0}, ThreadId, BufferLoad{handle_range_id:0},
  BufferLoad{handle_range_id:1}, BufferStore{handle_range_id:2}]`. DXIL
  opcode numbers (`CreateHandle`=57, `BufferLoad`=68, `BufferStore`=69,
  `ThreadId`=93) were confirmed via web search against Microsoft's
  `DirectXShaderCompiler/docs/DXIL.rst`, not assumed from memory, and
  matched the real decoded constants exactly. **Still no DXIL-to-SPIR-V
  translation** — that's the next increment. See "Not implemented"
  below.
- **D3D11 graphics pipeline — real SPIR-V generation for VS/PS reached
  and validated, no rasterizer/draw yet.** `shaders/triangle_vs.hlsl`/
  `shaders/triangle_ps.hlsl` (minimal passthrough vertex+pixel shader
  pair, `POSITION`/`COLOR` in, `SV_POSITION`/`SV_TARGET` out) compiled
  with real `fxc.exe /T vs_5_0`/`/T ps_5_0`. `parse_dxbc` parses both
  without modification. `spirv_gen::translate_vertex_shader`/
  `translate_pixel_shader` (new) decode the real, fixed SHEX opcode
  sequence (`dcl_input`x2/`dcl_output_siv`/`dcl_output`/`mov`x3/`ret` for
  VS; `dcl_input_ps`(linear)/`dcl_output`/`mov`/`ret` for PS) and emit
  real graphics SPIR-V: `OpEntryPoint Vertex`/`Fragment` (not
  `GLCompute`), `Input`/`Output` storage-class variables with `Location`
  decorations, `BuiltIn Position` on the vertex shader's `SV_POSITION`
  output, and `OpExecutionMode ... OriginUpperLeft` on the fragment
  shader. Validated two ways: (1) `rspirv`'s own loader re-parses the
  emitted bytes without error, (2) the real Vulkan SDK's `spirv-val.exe`
  (`C:\VulkanSDK\1.4.350.0\Bin\spirv-val.exe`) was run against both
  emitted modules and returned exit code 0 with no diagnostics for both.
  `translate_shader`/`translate_chain_shader` (compute-only) still
  correctly reject both shaders. **No rasterizer, no framebuffer, no
  actual Vulkan draw call exists** — `opencuda-vulkan` was confirmed (by
  reading its real source) to have no `VkGraphicsPipelineCreateInfo`/
  render-pass/framebuffer code at all, compute-dispatch only, so an
  actual rendered pixel is out of scope for this pass. See `CLAUDE.md`
  HANDOFF for the honest milestone boundary.

## Current state (2026-07-26, D3D11 minimal graphics pipeline milestone reached)

New crate `crates/directx-graphics-vulkan` adds `ash` as a **direct**
dependency of this workspace (not layered on `opencuda-vulkan`, which was
confirmed by source audit to be compute-dispatch only). It implements a
real render pass, framebuffer, and `VkGraphicsPipelineCreateInfo`, reusing
the already-generated, already `spirv-val`-passing SPIR-V from
`translate_vertex_shader`/`translate_pixel_shader` above (no shader
translation is re-implemented). `render_uniform_triangle_and_read_back`
draws one full-viewport "big triangle" with a single uniform vertex color,
reads the rendered image back through a host-visible staging buffer, and
the real-hardware test
(`crates/directx-graphics-vulkan/tests/triangle_real_vulkan.rs`) asserts
every read-back pixel matches the passthrough vertex color on the real
NVIDIA GT 730 present on this machine (`cargo test -p
directx-graphics-vulkan --test triangle_real_vulkan -- --nocapture`: 1
passed). Scope is intentionally narrow: one fixed shader pair, one draw
call, no depth buffer/textures/swapchain/multi-triangle interpolation
check. See `CLAUDE.md` HANDOFF (2026-07-26 continuation) for the full
honest disclosure.

## Current state (2026-07-25, Phase 1 vertical slice generalized to 3 known shaders)

`crates/directx-shader-translate` now does the full vertical slice for
**three specific known shaders** (`vector_add.hlsl`, `vector_mul.hlsl`,
`vector_sub_bounded.hlsl`): DXBC parse -> narrow SM5.0 opcode-subset
decode -> SPIR-V codegen (via `rspirv`) -> real Vulkan dispatch
(`open-cuda`'s `opencuda-vulkan`) -> CPU-reference numeric match,
verified on this machine's real NVIDIA GT 730. **This is still not a
general SM5.0-to-SPIR-V decoder** — see "Not implemented" below.

- `parse_dxbc` (Phase 0): DXBC container/chunk introspection (RDEF/ISGN/
  OSGN/SHEX presence), unchanged from the original front-end.
- `spirv_gen::translate_shader` (Phase 1, generalized 2026-07-25):
  recognizes 3 opcode shapes actually emitted by `fxc.exe`, all sharing
  a common skeleton (`dcl_globalFlags` -> optional `dcl_constantbuffer`
  -> 3x `dcl_uav_structured` -> `dcl_input` -> `dcl_temps` ->
  `dcl_thread_group` -> optional `ult`+`if` -> 2x `ld_structured` ->
  `add`/`mul` -> `store_structured` -> optional `endif` -> `ret`):
  - `vector_add.hlsl`: `add`, no bounds check.
  - `vector_mul.hlsl`: `mul` instead of `add`.
  - `vector_sub_bounded.hlsl`: `add` with a `negate` flag on its first
    source operand (confirmed by dumping real `fxc.exe` output — `fxc`
    optimizes `a - b` into `add dest, -b, a` rather than emitting a
    dedicated `sub` opcode), plus a real `if (id.x < N)` bounds check
    (`ult` against a constant buffer + `if`/`endif`), which the emitted
    SPIR-V implements with an actual `OpSelectionMerge`/
    `OpBranchConditional`, using the push-constant `n` for the compare.
  Any other opcode/shape is rejected via `SpirvGenError::UnsupportedShader`
  rather than silently mistranslated. UAV bind points, thread-group
  size, operator, and bounds-check presence are all extracted from the
  real parsed DXBC, not hardcoded. `translate_vector_add_shader` is kept
  as a thin backward-compatible alias for `translate_shader`.
- `tests/vector_add_real_vulkan.rs`, `tests/vector_mul_real_vulkan.rs`,
  `tests/vector_sub_bounded_real_vulkan.rs`: each dispatches its
  translated SPIR-V through `open-cuda`'s real
  `opencuda-vulkan::VulkanDevice` (`ash`, `real-vulkan` feature) and
  checks the GPU output against a CPU reference for 256 elements
  (1e-3/1e-2 epsilon). The bounds-check test additionally dispatches
  320 threads with a logical element count of 256 and asserts that
  elements 256..320 are never written (stay at a sentinel value),
  proving the `if (id.x < N)` branch in the generated SPIR-V actually
  gates execution rather than just compiling.
- `examples/dump_shex.rs`: a small standalone tool
  (`cargo run -p directx-shader-translate --example dump_shex -- <path.dxbc>`)
  used during this session to inspect real SHEX opcode streams before
  writing decoder support for them; kept for future opcode-by-opcode
  generalization work.

**Since this section's title was written**, a 4th single-op shader
(`vector_div.hlsl`, plain `div`) was added to `translate_shader`
following the exact same pattern, and — more recently — a genuinely
different pattern class, `spirv_gen::translate_chain_shader`, was added
alongside it (not replacing it): it decodes an actual
register-expression tree of sequential binary operations (add/mul, no
control flow) instead of a single fixed op, verified against a newly
compiled shader whose real SHEX turned out to reuse one temp register's
components via fxc's CSE rather than declaring extra temps. See the
2026-07-25 "continued 9" HANDOFF entry in `CLAUDE.md` for the full,
current account (this section is left as originally written for
historical accuracy about the 2026-07-25 mid-day state).

## Build & test

```powershell
cargo build --workspace
cargo test --workspace -- --nocapture
```

### See it actually draw something (added 2026-07-27)

This repo is a set of libraries with no `fn main` of its own, so the fastest
way to *see* the graphics pipeline work on your own GPU — rather than reading
test source — is:

```bash
cargo run -p directx-graphics-vulkan --example render_triangle
```

This reuses the same real fxc.exe-compiled DXBC → SPIR-V translated shaders
as `tests/triangle_real_vulkan.rs`, draws a gradient (red/green/blue)
triangle on real Vulkan hardware, reads the framebuffer back, and writes it
to `render_triangle.ppm` (plain PPM, no extra image-crate dependency needed —
convert it with e.g. `magick render_triangle.ppm render_triangle.png` or open
it directly in most image viewers). If no usable Vulkan device/driver is
present, it prints an honest error and exits non-zero rather than faking
success.

Actually observed output (2026-07-25, this machine, NVIDIA GeForce GT 730):

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

`cargo clippy --workspace --all-targets`: 0 warnings.

After the DXIL type-table/instruction-decoding work (2026-07-25,
continued, D3D12 track), `cargo test --workspace` runs 23 tests total
(19 unit + 4 real-Vulkan integration tests), all passing, including 3
new ones on top of the earlier 20: `dxil::tests::resolves_real_dxil_
type_table_and_finds_float_and_resource_struct`, `dxil::tests::decodes_
real_dxil_function_block_into_matching_vector_add_shape`, and `dxil::
tests::shape_matcher_honestly_rejects_unexpected_instruction_orderings`.

To regenerate the DXBC fixtures from HLSL (requires the Windows SDK's
`fxc.exe` — note `dxc.exe` only targets DXIL/SM6+ and cannot produce
DXBC):

```powershell
pwsh tools/compile-dxbc-shaders.ps1
```

## Not implemented (honest scope)

- **General SM5.0 instruction decoding.** Only the 3 opcode shapes above
  are handled; any other D3D11 compute shader (different resource
  layout, other control flow, other intrinsics, more than one bounds
  check, `div`/`sub`-as-a-real-opcode instead of negated-`add`, etc.) is
  rejected, not mistranslated. Building a real general decoder (or
  adopting/porting an existing one, e.g. studying `dxbc-spirv`/
  `dxil-spirv`'s approach more closely) remains the actual next
  milestone.
- **DXIL (Shader Model 6+, D3D12): the `vector_add.dxil` vertical slice
  is now complete end-to-end, on real hardware — but still only for
  this one known shader shape, not general SM6.0.**
  `resolve_type_table`/`decode_function_instructions`/
  `resolve_vector_add_dxil_calls` in `dxil.rs` decode the real
  `TYPE_BLOCK`/`FUNCTION_BLOCK`/`VALUE_SYMTAB_BLOCK` records against
  LLVM's documented codes and disambiguate all 7 `Call` records to their
  real `dx.op.*` meaning (`CreateHandle`/`ThreadId`/`BufferLoad`/
  `BufferStore`, with UAV bind points). `translate_dxil_vector_add_to_spirv`
  (new) feeds that resolved output into `spirv_gen.rs`'s shared
  `emit_spirv_for_kernel` (factored out of the DXBC path's `emit_spirv`
  so both backends emit from one code path) to produce real SPIR-V,
  which `tests/vector_add_dxil_real_vulkan.rs` dispatches on this
  machine's real NVIDIA GT 730 via `opencuda-vulkan` and verifies
  matches the CPU reference `a[i]+b[i]` for all 256 elements — the same
  rigor as the DXBC `vector_add` test. **Workgroup size is now actually
  extracted, not hardcoded**: `extract_numthreads_from_metadata`
  (`dxil.rs`) walks the real `METADATA_BLOCK` path
  `dx.entryPoints` -> per-entry-point tuple -> `ShaderProperties` ->
  `kDxilNumThreadsTag` (=4, confirmed against Microsoft
  `DirectXShaderCompiler`'s `DxilMetadataHelper.h`/`.cpp` sources) and
  resolves the `{x,y,z}` node against the module's real value list,
  yielding `(64,1,1)` from the actual bytes of `vector_add.dxil` — the
  known hardcode from the previous entry is closed, and a synthetic
  regression test proves the extraction logic returns a *different*
  value when given different metadata (not just "returns 64,1,1 no
  matter what"). Any other opcode/operand shape (different operation,
  multiple basic blocks, bounds checks) is still rejected, not
  mistranslated. D3D12 command list/descriptor heap/root signature
  support (the layer above shader translation) is untouched.
- **DXBC decoder generalized beyond 4 fixed single-op shapes: now
  handles chains of sequential binary operations (no control flow) via
  a real register-expression tree, not a 5th hardcoded shape.**
  `spirv_gen::translate_chain_shader`/`decode_chain_shape` walk
  `ld_structured`/`add`/`mul`/`store_structured` and build an actual
  expression tree keyed by (temp register, component), so it handles 1
  op, 2 ops, or N ops the same way — verified against a newly compiled
  real shader (`vector_add_mul_chain.hlsl`, `t = A[i]+B[i]; Out[i] =
  t*A[i]`) whose real SHEX turned out to reuse a single temp register's
  `.x`/`.y` components (fxc CSE'd the repeated `A[i]` load away instead
  of re-issuing `ld_structured`) — a genuine, unpredicted finding the
  tree-based decoder handles without extra cases. Dispatched and
  verified on the real NVIDIA GT 730 against the CPU reference
  `(a[i]+b[i])*a[i]`. `sub`/`div` inside a chain are intentionally still
  rejected (their operand-order semantics were only verified for the
  single-op case). The original 4 single-op shapes are untouched and
  still pass unmodified.
- **D3D11 graphics pipeline: DXBC container parsing confirmed working
  for VS/PS, but no SPIR-V codegen, no rasterizer, no actual triangle
  drawn on screen.** The full pipeline (rasterizer, texture sampling,
  blend state, output-merger) remains out of scope; so does extending
  `spirv_gen`'s narrow opcode-shape decoder to understand
  `dcl_output_siv`/`dcl_input_ps`/interpolation modes.
- PlayStation family targets — explicitly out of scope; see `CLAUDE.md`
  for the legal/ToS reasoning.

## Related projects

- [open-cuda](https://github.com/aon-co-jp/open-cuda) — the Vulkan
  compute execution backend this project is designed to dispatch
  through (`opencuda-core::GpuDevice`, `KernelSource::SpirV`). Also
  contains an unrelated, already-working `opencuda-directx` crate that
  runs D3D12 **natively on Windows** — the opposite direction from this
  project (which runs DirectX shaders **on non-Windows targets**).
- [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm) — no direct
  technical dependency on this project (verified, not assumed).
