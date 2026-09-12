# PORTING.md — what's reusable, by whom, and how

*日本語*: [PORTING-Japanese.md](PORTING-Japanese.md) ·
*Other languages*: [Deutsch](PORTING-German.md) · [Italiano](PORTING-Italian.md) ·
[Français](PORTING-French.md) · [Русский](PORTING-Russian.md) ·
[Українська](PORTING-Ukrainian.md) · [עברית](PORTING-Hebrew.md) ·
[فارسی](PORTING-Persian.md) · [العربية](PORTING-Arabic.md)

> 🎯🕒 **Porting prerequisite (2026-08-29)**: aruaru-db only delivers
> "no REST API needed, compatible with WunderGraph Cosmo's paid
> Enterprise tier" when paired (SET) with RPoem (canonical source:
> aruaru-db/CLAUDE.md's opening note). **open-directx is currently a
> GPU compute library with no HTTP surface and is out of scope for
> that policy** (confirmed via `grep`) — **but this is a provisional
> call for right now, not permanent**: as its DirectX-compatibility
> work matures and OS-level commands / hardware-accelerator
> (open-directx/open-cpu) execution paths become more advantageous,
> re-evaluate this scoping.

> **2026-08-08 更新(続き、2Dスプライト描画プロトタイプ)**: 新規クレート
> `crates/directx-graphics-window`(winit+ash-window、実ウィンドウ+実
> スワップチェーン+実キーボード入力)・`directx-graphics-vulkan`の
> `render_sprites_and_read_back`/`SpriteInstance`/`TextureRgba8`/
> `png_loader`を追加。**移設時の注意**: `directx-graphics-window`は
> `directx-graphics-vulkan`とは独立したVulkanインスタンス/デバイスを
> 持つ設計(オフスクリーン描画とウィンドウ描画のVulkanコンテキストは
> 統合されていない)。またアルファブレンドは`render_sprites_and_read_
> back`(オフスクリーン版)のみ有効化済みで、`directx-graphics-window`
> (実ウィンドウ版)側にはまだ反映していない——両方使う場合は同期を
> 取ること。PNGローダー(`png_loader::load_png_rgba8`)は`png`クレート
> (0.17系)への依存を新規追加している。
>
> *English*: Added a new `crates/directx-graphics-window` crate
> (winit + ash-window: real window, real swapchain, real keyboard
> input) and `directx-graphics-vulkan`'s `render_sprites_and_read_back`/
> `SpriteInstance`/`TextureRgba8`/`png_loader`. **Porting note**:
> `directx-graphics-window` holds its own independent Vulkan
> instance/device (not unified with the offscreen rendering context).
> Alpha blending is currently enabled only in `render_sprites_and_read_
> back` (offscreen), not yet mirrored into `directx-graphics-window`
> (real window) — keep them in sync if you use both. The PNG loader
> (`png_loader::load_png_rgba8`) adds a new dependency on the `png`
> crate (0.17.x).

> **2026-08-08 更新**: 境界チェック付き7項チェーンについてDXBC/DXIL両方の
> 実装が揃った(`vector_add_mul_div_sub_add_mul_div_chain7_bounded_dxil.hlsl`
> を新規追加、実`dxc.exe`コンパイル+実GT730検証済み)。移設先で7項チェーンの
> 変換ロジックを再利用する場合は、DXBC側・DXIL側どちらも同じ`decode_chain_
> shape`/`translate_dxil_chain_to_spirv`系の関数を経由するため、片方だけ
> 移植すると非対称なギャップが生じる点に注意(詳細はCLAUDE.md HANDOFF参照)。
>
> *English*: The boundary-checked 7-term chain now has both DXBC and DXIL
> implementations (new `vector_add_mul_div_sub_add_mul_div_chain7_bounded_
> dxil.hlsl`, real-`dxc.exe`-compiled and GT730-verified). When porting the
> chain-translation logic elsewhere, note that DXBC and DXIL both go
> through the same `decode_chain_shape`/`translate_dxil_chain_to_spirv`-
> family functions — porting only one side reintroduces the asymmetry gap
> just closed. See CLAUDE.md HANDOFF for details.

> **2026-07-25 更新**: 開発方針ファイル(`CLAUDE.md`)の見出しを
> 「設計思想＆開発方針＆開発環境ルール」へ改名しました
> (設計思想・開発方針・開発環境ルールを明確に区別)。移設先でも
> `CLAUDE.md`の内容を必ず確認してください。


## `crates/directx-shader-translate`

**Reusable by**: any Rust project that needs to inspect DXBC (D3D9/10/11
shader bytecode, Shader Model <= 5.1) containers — e.g. a future D3D9/10/11
graphics-pipeline layer in this same repo, a shader-cache/asset-pipeline
tool, or a completely unrelated project that needs to read `.cso`/`.fxc`
shader blobs. As of 2026-07-25 it also ships a narrow, honestly-scoped
DXBC(SM5.0)->SPIR-V translator for exactly one known shader shape
(see "Known-shader SPIR-V translation" below).

**How**: path dependency, same convention used elsewhere in this
ecosystem (e.g. `aruaru-llm/Cargo.toml`'s
`opencuda-core = { path = "../open-cuda/crates/opencuda-core" }`, or
`aruaru-db`'s `rust-json = { path = "../RS-JSON" }`):

```toml
[dependencies]
directx-shader-translate = { path = "../open-directx/crates/directx-shader-translate" }
```

Public API surface (container/chunk introspection, Phase 0):

```rust
pub struct ShaderModule {
    pub chunk_count: usize,
    pub has_resource_definitions: bool,
    pub has_input_signature: bool,
    pub has_output_signature: bool,
    pub instruction_count: Option<usize>,
}

pub fn parse_dxbc(bytes: &[u8]) -> Result<ShaderModule, TranslateError>;
```

This is intentionally a thin summary, not a full re-export of the
underlying `dxbc` crate's rich per-chunk structures (`ResourceDef`,
`Signature`, `Program`, etc.). Callers that need the full detail
should depend on the `dxbc` crate directly — this crate does not hide
it, it re-exports nothing exclusive.

## Known-shader SPIR-V translation (`spirv_gen` module, added 2026-07-25, generalized to 3 shapes same day)

```rust
pub enum BinaryOp { Add, Mul, Sub }

pub struct TranslatedKernel {
    pub spirv_words: Vec<u32>,
    pub entry_point: &'static str,        // always "main"
    pub local_size: (u32, u32, u32),      // extracted from dcl_thread_group
    pub uav_bind_points: (u32, u32, u32), // extracted from RDEF/ld_structured/store_structured
}

pub fn translate_shader(bytes: &[u8]) -> Result<TranslatedKernel, SpirvGenError>;
// thin backward-compatible alias, same behavior as translate_shader:
pub fn translate_vector_add_shader(bytes: &[u8]) -> Result<TranslatedKernel, SpirvGenError>;
```

**Honest scope**: this is *not* a general SM5.0 decoder. It recognizes
exactly 3 narrow opcode shapes that `fxc.exe` actually emits for the 3
shaders in `shaders/`, all sharing this skeleton:

```
dcl_globalFlags -> (dcl_constantbuffer(b0))? -> dcl_uav_structured x3
-> dcl_input(vThreadID) -> dcl_temps -> dcl_thread_group
-> (ult + if)? -> ld_structured x2 -> (add | mul | add-with-negate)
-> store_structured -> (endif)? -> ret
```

- `vector_add.hlsl` -> `add`, no bounds check.
- `vector_mul.hlsl` -> `mul` instead of `add`.
- `vector_sub_bounded.hlsl` -> `add` whose first source operand has the
  `negate` flag set (confirmed by dumping real `fxc.exe` output with
  `examples/dump_shex.rs`: `fxc` optimizes `a - b` into
  `add dest, -b, a` rather than emitting a dedicated `sub` opcode), plus
  a real `if (id.x < N)` bounds check (`ult` against a constant buffer,
  then `if`/`endif`).

Any other opcode, or a shape that doesn't match one of the 3 above (e.g.
more than 2 reads, a different register class, a partial/incomplete
bounds-check construct), is rejected with `SpirvGenError::UnsupportedShader`
rather than silently mistranslated. The UAV bind points, thread-group
size, detected operator, and bounds-check presence embedded in the
emitted SPIR-V are **not hardcoded** — they come from actually parsing
the real DXBC container's `RDEF`/`SHEX` chunks via the `dxbc` crate. The
SPIR-V binary itself is assembled with `rspirv` (not hand-rolled binary
bytes); for the bounds-checked shader this includes a real
`OpSelectionMerge`/`OpBranchConditional` pair, not just a declared-but-
unused push constant. Self-consistency is validated by re-parsing with
`rspirv::binary::parse_bytes` in the test suite.

**Verified end-to-end (2026-07-25)**: `crates/directx-shader-translate/tests/vector_add_real_vulkan.rs`,
`tests/vector_mul_real_vulkan.rs`, and `tests/vector_sub_bounded_real_vulkan.rs`
each parse a real `fxc.exe`-compiled `.dxbc` fixture, translate it with
`translate_shader`, dispatch the resulting SPIR-V through `open-cuda`'s
real `opencuda-vulkan::VulkanDevice` (`ash`-based, `real-vulkan` feature)
on this machine's NVIDIA GeForce GT 730, and confirm the GPU output
matches a CPU reference for all elements within tolerance. The
bounds-check test additionally dispatches 320 threads against a logical
element count of 256 and asserts elements 256..320 are never written
(stay at a sentinel value), proving the branch actually gates execution.
See `CLAUDE.md`'s HANDOFF (2026-07-25, second continuation entry) for
the exact `cargo test` output.

```rust
// actual, working (not conceptual) — see tests/vector_add_real_vulkan.rs
let kernel = directx_shader_translate::translate_shader(&dxbc_bytes)?;
let spirv_bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
let compiled = opencuda_core::CompiledKernel::spirv("vector_add", kernel.entry_point, spirv_bytes);
device.launch_kernel(&compiled, &cfg, &args)?; // opencuda_vulkan::VulkanDevice, real hardware
```

Note: `opencuda-vulkan::VulkanDevice::launch_kernel` currently dispatches
based on `CompiledKernel::name` (only `"vector_add"`/`"vector_add_f32"`
and `"matmul"`/`"matmul_f32"` are recognized, each selecting an
args-plumbing path of N storage buffers + a fixed-size push constant).
The `vector_mul`/`vector_sub_bounded` tests reuse the `"vector_add"` name
because their argument layout (3 buffers + 1 `uint` push constant) is
identical — the actual operation executed is whatever the dispatched
SPIR-V bytes say, not something implied by the name string.

## DXIL (SM6+, D3D12) — container/bitstream parsing, added 2026-07-25

`src/dxil.rs`'s `parse_dxil_container(bytes) -> Result<DxilModule, DxilParseError>`
parses a real `dxc.exe -T cs_6_0`-compiled DXBC container
(`shaders/vector_add_dxil.hlsl` -> `shaders/vector_add.dxil`, same
`vector_add` contract as the SM5.0/DXBC shader, compiled separately so
the DXBC-vs-DXIL container diff isn't entangled with a shader-content
diff). Two real steps, chained:

1. The existing `dxbc` crate (already a dependency) parses the `DXIL`
   chunk's `DxilProgramHeader` (shader kind, SM major/minor) and
   `DxilBitcodeHeader` (magic `'DXIL'`, bitcode offset/size) and returns
   the raw LLVM bitcode bytes (`dxbc::chunks::dxil::DxilData::bitcode`).
   This part already existed in the crate before today; it just wasn't
   being called from this repo.
2. The newly-added `llvm-bitcode = "0.4.0"` crate (generic LLVM
   bitstream reader, no DXIL/HLSL-specific knowledge) reads the raw
   bitcode bytes into a `Bitcode { elements: Vec<BitcodeElement> }` tree
   of blocks/records. `dxil.rs` walks the top level and one level of
   children, recording each block's raw numeric LLVM block ID.

Actually confirmed against the real `vector_add.dxil` bytes (not
assumed from documentation): the LLVM bitcode wrapper magic
`BC\xC0\xDE`, a single top-level `MODULE_BLOCK_ID` (8), and inside it
`TYPE_BLOCK_ID_NEW`(17), `PARAMATTR_GROUP_BLOCK_ID`(10),
`PARAMATTR_BLOCK_ID`(9), `CONSTANTS_BLOCK_ID`(11), `FUNCTION_BLOCK_ID`
(12, appearing 5 times — one per basic block in `main`'s single-function
body), `VALUE_SYMTAB_BLOCK_ID`(14), and `METADATA_BLOCK_ID`(15, twice).
This was found empirically with `examples/dump_dxil.rs` before writing
the assertions in `dxil.rs`'s tests, the same "dump first, decode
narrowly second" discipline used for the DXBC/SM5.0 SHEX opcode work.

**Update (2026-07-25, continued, D3D12 track)**: type-table resolution
and coarse instruction decoding have since been added on top of the
above (still in `src/dxil.rs`). `resolve_type_table(&Block) ->
Vec<DxilType>` applies LLVM's documented `TYPE_BLOCK` record codes
(`VOID`=2, `FLOAT`=3, `INTEGER`=7, `POINTER`=8, `FUNCTION`=21,
`STRUCT_NAME`=19/`STRUCT_NAMED`=20, `METADATA`=16) to the real
`vector_add.dxil` type table (22 resolved types), confirming type#12 is
`Float` and type#19 is `StructNamed{"class.RWStructuredBuffer<float>"}`.
`decode_function_instructions(&Block) -> Vec<DxilInstruction>` applies
LLVM's `FUNC_CODE_*` table (`DECLAREBLOCKS`=1, `BINOP`=2, `RET`=10,
`EXTRACTVAL`=26, `CALL`=34) to the real `FUNCTION_BLOCK`, yielding
`DeclareBlocks(1) -> Call*5 -> ExtractValue -> Call -> ExtractValue ->
BinOp -> Call -> Ret` for `vector_add.dxil`'s `main`. `decode_
vector_add_dxil_shape` narrowly validates this exact shape (one basic
block, exactly one `BinOp`, at least one trailing `Call` after it, ends
in `Ret`) and honestly rejects anything else via `DxilShapeError`,
mirroring `SpirvGenError::UnsupportedShader` on the DXBC side.

**Update (2026-07-25, continued 6): all 7 `Call` records are now
disambiguated.** DXIL represents every intrinsic op
(`CreateHandle`/`ThreadId`/`BufferLoad`/`BufferStore`) as an ordinary
LLVM `CALL` to a `dx.op.*` function; `resolve_vector_add_dxil_calls` in
`dxil.rs` resolves this by:
1. Reading `VALUE_SYMTAB_BLOCK` (id=14) — found that `llvm-bitcode`'s
   `Record::fields()` only returns the value ID for `VST_CODE_ENTRY`
   records; the actual name string is in `Record::take_payload()`
   (`Payload::Char6String`), confirmed by extending `examples/dump_dxil.rs`
   to dump it. Real result: value IDs 0-4 map to `main`,
   `dx.op.threadId.i32`, `dx.op.createHandle`, `dx.op.bufferLoad.f32`,
   `dx.op.bufferStore.f32`.
2. Hand-decoding LLVM's relative-value operand encoding (operand field
   = `current_value_no - relative`, where `current_value_no` is the
   count of values defined so far, not including this instruction's own
   result) — verified by manual arithmetic against the real byte
   sequence (global value numbering: 5 function decls -> module-level
   constants, skipping value-free `CST_CODE_SETTYPE` -> function-local
   constants).
3. DXIL opcode numbers (`CreateHandle`=57, `BufferLoad`=68,
   `BufferStore`=69, `ThreadId`=93) confirmed via web search against
   Microsoft's `DirectXShaderCompiler/docs/DXIL.rst`, and cross-checked
   against the real decoded constant values (all matched).

Result for `vector_add.dxil`: `[CreateHandle{range_id:2},
CreateHandle{range_id:1}, CreateHandle{range_id:0}, ThreadId,
BufferLoad{handle_range_id:0}, BufferLoad{handle_range_id:1},
BufferStore{handle_range_id:2}]` — i.e. the first `BufferLoad` reads
u0, the second reads u1, and the `BufferStore` writes u2, exactly
mirroring the DXBC side's `vector_add` shape. Unexpected callees, arg
counts, opcode constants, or operand shapes are rejected via
`DxilCallResolutionError`, matching `SpirvGenError::UnsupportedShader`'s
pattern.

### DXIL-to-SPIR-V translation + real hardware dispatch (added 2026-07-25, later same day)

`spirv_gen.rs`'s `emit_spirv(shape: &ShaderShape)` body was renamed to
`emit_spirv_impl` and factored into a shape-agnostic
`pub(crate) fn emit_spirv_for_kernel(thread_group, uav_a, uav_b, uav_c,
op: BinaryOp, bounds_check: bool) -> Vec<u32>`, so both the DXBC and
DXIL backends emit SPIR-V from one shared code path (the DXBC-facing
`emit_spirv` is now a thin wrapper; existing DXBC tests are unaffected).

`dxil.rs`'s new `translate_dxil_vector_add_to_spirv(bytes) ->
Result<TranslatedKernel, DxilSpirvError>` takes the 7 resolved
`ResolvedDxilCall` values above and maps them onto that shared emitter:
the first `BufferLoad`'s `handle_range_id` becomes buffer A, the
second becomes B, and `BufferStore`'s `handle_range_id` becomes C
(same "discovery order" convention as the DXBC side's `ld_uavs`). The
operation is fixed to `BinaryOp::Add` with no bounds check, since that
is what `vector_add_dxil.hlsl` is confirmed to produce.

**Update (2026-07-25, "continued 9"): the workgroup-size hardcode above
is now closed.** `dxil::extract_numthreads_from_metadata` decodes the
real `METADATA_BLOCK` path: `dx.entryPoints` (a `METADATA_NAMED_NODE`)
-> the per-entry-point 5-tuple (`Function, Name, Signatures, Resources,
ShaderProperties`) -> `ShaderProperties` (a repeating `{tag, value}`
list) -> the pair whose tag resolves to `kDxilMDHelper::kDxilNumThreadsTag`
(confirmed = `4` against Microsoft `DirectXShaderCompiler`'s
`include/dxc/DXIL/DxilMetadataHelper.h` and
`lib/DXIL/DxilMetadataHelper.cpp` sources) -> a 3-element node whose
operands resolve (via `METADATA_VALUE` -> absolute value-list index,
against the same module value list — functions ++ module
`CONSTANTS_BLOCK` — already built for `resolve_vector_add_dxil_calls`,
now factored into a shared `build_module_value_list`) to the real
constants `64, 1, 1`. This was hand-traced end to end against
`vector_add.dxil`'s actual bytes (not assumed) before being coded, and
a synthetic unit test (`finds_numthreads_pair_even_when_a_different_value_precedes_it`)
proves the pair-scanning logic returns a *different* triple `(32,8,2)`
when given different metadata — guarding against a silent regression to
hardcoding. `translate_dxil_vector_add_to_spirv` now calls this instead
of using a literal `(64,1,1)`, and the existing
`dxil_vector_add_matches_cpu_reference_on_real_vulkan_hardware` test
still passes with the now-extracted value.

Original honest gap being closed here, for historical context: DXBC's
`dcl_thread_group` has no DXIL equivalent in what this project decoded
before this pass; `numthreads` is actually encoded in DXIL's
`METADATA_BLOCK` (`dx.entryPoints`), which was out of scope until now.

`tests/vector_add_dxil_real_vulkan.rs` mirrors
`vector_add_real_vulkan.rs` exactly: parse real `vector_add.dxil` ->
run the full DXIL decode pipeline -> `translate_dxil_vector_add_to_spirv`
-> dispatch via `opencuda_vulkan::VulkanDevice` on real hardware ->
compare against the CPU reference `a[i]+b[i]`. One integration wrinkle
surfaced only at runtime: `VulkanDevice`'s `launch_kernel` dispatches
by `CompiledKernel::name`, and only recognizes the literal string
`"vector_add"` (not `"vector_add_dxil"`) — using the wrong name failed
with `VulkanDevice v0.4.0 only implements vector_add/vector_add_f32 and
matmul/matmul_f32; got \`vector_add_dxil\``, which is why the test
passes `"vector_add"` as the kernel name despite the DXIL origin.

Real output (NVIDIA GeForce GT 730, `cargo test --workspace`):

```
test dxil_vector_add_matches_cpu_reference_on_real_vulkan_hardware ... ok
```
with stdout `device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)` and
`OK: DXIL(dxc.exe実コンパイル、SM6.0)->SPIR-V(自前生成)->実Vulkan(NVIDIA GT 730)経路が、CPU参照実装(a[i]+b[i])と256要素すべてで数値一致した`.
All 5 real-hardware tests (4 DXBC + 1 DXIL) and 22 unit tests pass;
`cargo build --workspace` / `cargo clippy --workspace --all-targets`
are clean (0 warnings).

**This reaches parity with the DXBC `vector_add` milestone, but only
for this one known DXIL shader shape** — not a general SM6.0 decoder.
Any operation other than `add`, more than one basic block, or bounds
checks is still honestly rejected, not mistranslated. (`numthreads` is
no longer hardcoded — see the update above.)

## DXBC decoder generalized: sequential binary-op chains (2026-07-25, "continued 9")

The 4 single-op DXBC shapes above (`add`/`mul`/`div`/negated-add-as-sub)
are untouched. Added alongside them: `spirv_gen::translate_chain_shader`
/ `decode_chain_shape`, a genuinely more general pattern class ("N
sequential binary operations, no control flow") rather than a 5th
hardcoded shape.

**Real finding that shaped the design**: a new shader,
`vector_add_mul_chain.hlsl` (`t = InputA[i] + InputB[i]; Output[i] = t *
InputA[i];`, still 3 UAVs so it fits `opencuda-vulkan`'s fixed 3-buffer
`"vector_add"` argument wiring — `ensure_vector_add_args`/
`ensure_matmul_args` in `open-cuda`'s `real.rs` are both hardcoded to
exactly 3 buffers, and this project intentionally does not modify
`open-cuda`), was compiled with real `fxc.exe` and its real SHEX dumped
with `examples/dump_shex.rs`. Expected `dcl_temps` to grow to 2 (one
register per HLSL local). Instead `dcl_temps` stayed at **1**: `fxc`
reused register `r0`'s `.x`/`.y` components for `t` and the reload of
`InputA[i]`, and — a second, unpredicted optimization — it didn't even
re-issue a second `ld_structured` for the repeated `InputA[i]`
reference; it reused the first load's result via component `.y`
(classic CSE). A decoder that assumed "one temp register per operation"
would have missed this shader entirely.

**Design**: `decode_chain_shape` walks the instruction stream building a
`HashMap<(temp_index, component), RegExpr>` where `RegExpr` is either
`Load(uav_bind_point)` (from `ld_structured`) or `BinOp(op, lhs, rhs)`
(from `add`/`mul`, looking up its two source operands' current
`RegExpr` by their `(temp, component)` key — so it doesn't matter
whether those operands came from a fresh load or were CSE'd from an
earlier one). `store_structured`'s source operand resolves to the root
of the expression tree. `emit_chain_spirv` then recursively (post-order)
emits `OpAccessChain`/`OpLoad`/`OpFAdd`/`OpFMul` for the tree — handling
1 op, 2 ops, or (by construction, though only 2 is exercised by a real
compiled shader so far) N ops identically. `sub` (negated-add
optimization) and `div` are explicitly rejected inside a chain — their
operand-order semantics were only confirmed for the single, non-chained
case, and this project does not claim support it hasn't verified.

`tests/vector_add_mul_chain_real_vulkan.rs` (same pattern as the
existing 4 real-hardware tests) dispatches the chain-translated SPIR-V
on the real NVIDIA GT 730 and checks against the CPU reference
`(a[i]+b[i])*a[i]` for 256 elements. Real output:

```
device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
OK: DXBC(fxc.exe実コンパイル, 2項演算2回のチェーン)->SPIR-V(自前生成、式木の再帰翻訳)->実Vulkan経路が、CPU参照実装((a[i]+b[i])*a[i])と256要素すべてで数値一致した
c[0]=65, c[255]=708.875
test dxbc_vector_add_mul_chain_matches_cpu_reference_on_real_vulkan_hardware ... ok
```

All 6 real-hardware tests (4 single-op DXBC + 1 chain DXBC + 1 DXIL)
and 28 unit tests pass; `cargo build --workspace` /
`cargo clippy --workspace --all-targets` are clean (0 warnings). The
original 4 single-op shapes and the DXIL vertical slice are unmodified
and still pass — this was purely additive.

## D3D11 graphics pipeline (vertex/pixel shaders) — DXBC parsing only, added 2026-07-25

`shaders/triangle_vs.hlsl` (`POSITION`/`COLOR` in, `SV_POSITION`/`COLOR`
out) and `shaders/triangle_ps.hlsl` (`COLOR` in, `SV_TARGET` out) — a
minimal passthrough pair for a solid-color triangle — were compiled with
real `fxc.exe /T vs_5_0` / `/T ps_5_0` (`tools/compile-dxbc-shaders.ps1`).
The existing `parse_dxbc` front-end (unmodified) parses both containers
successfully — new tests
`parses_real_fxc_compiled_vertex_shader_dxbc_container` /
`_pixel_shader_dxbc_container` in `src/lib.rs` confirm `has_input_signature`/
`has_output_signature`/`instruction_count > 0` for both.

Dumping the real SHEX stream (`examples/dump_shex.rs`) confirmed the
opcode/operand vocabulary really is different from compute shaders, not
assumed: `dcl_globalFlags`, `dcl_input` (positional, for VS) /
`dcl_input_ps` (with `linear` interpolation, for PS), `dcl_output` /
`dcl_output_siv` (`SV_POSITION`), `mov`, `ret` — no
`dcl_uav_structured`, `ld_structured`/`store_structured`, or
`dcl_thread_group` at all (those are compute-only). Passing either
shader's DXBC into `translate_shader` (the existing compute-only SPIR-V
generator) is confirmed, via a new test
(`vertex_shader_spirv_translation_is_honestly_unimplemented_not_silently_wrong`),
to fail with `SpirvGenError::UnsupportedShader` rather than silently
emitting something wrong.

**Honest scope (superseded below, kept for history)**: this pass
established the parsing prerequisite only. The next increment (below)
built real SPIR-V generation on top of it.

## D3D11 graphics pipeline — real SPIR-V generation for VS/PS, validated, added 2026-07-25 (later pass)

Built directly on the parsing groundwork above. `spirv_gen.rs` gained a
new, independent section (existing compute-only `decode_shader_shape`/
`decode_chain_shape` untouched): `decode_vertex_shader_shape`/
`decode_pixel_shader_shape` strictly match the real, fixed SHEX
instruction sequences dumped for `triangle_vs.dxbc`/`triangle_ps.dxbc`
(no free parameters — this pair of shaders is a pure passthrough, so
unlike the compute decoders there is nothing to extract, only a shape
to verify):

- VS (9 instructions): `dcl_globalFlags` -> `dcl_input`(v0, mask=7=xyz,
  POSITION) -> `dcl_input`(v1, mask=15=xyzw, COLOR) ->
  `dcl_output_siv`(o0, mask=15, SV_POSITION) -> `dcl_output`(o1,
  mask=15, COLOR) -> `mov o0.xyz, v0.xyzx` -> `mov o0.w, l(1.0)` ->
  `mov o1.xyzw, v1.xyzw` -> `ret`.
- PS (5 instructions): `dcl_globalFlags` -> `dcl_input_ps`(linear, v1,
  mask=15, COLOR) -> `dcl_output`(o0, mask=15) -> `mov o0.xyzw,
  v1.xyzw` -> `ret`.

`translate_vertex_shader`/`translate_pixel_shader` (new public API)
check this exact shape and, only if it matches, emit a real graphics
SPIR-V module via `rspirv::dr::Builder`:

- `OpEntryPoint Vertex`/`Fragment` (not `GLCompute` — the previous
  compute-only `emit_spirv_impl`/`emit_chain_spirv` are unchanged and
  still only emit `GLCompute`).
- `Input`/`Output` storage-class variables with `Location` decorations
  (not the storage-buffer/push-constant layout used by the compute
  path).
- `BuiltIn Position` decoration on the vertex shader's `SV_POSITION`
  output variable (a `vec4`, constructed in the shader body from the
  `vec3` POSITION input plus a literal `1.0` for `.w`, matching the
  real `mov o0.xyz, v0.xyz` / `mov o0.w, l(1.0)` pair instead of
  hand-waving a single passthrough).
- `OpExecutionMode ... OriginUpperLeft` on the fragment shader (a
  Vulkan-mandated execution mode with no DXBC equivalent to extract —
  added because Vulkan requires it, not derived from the shader bytes).

**Validated two independent ways, both with real output quoted here**:

1. `rspirv`'s own loader re-parses the emitted byte stream without
   error (`rspirv::binary::parse_bytes` succeeds, `OpEntryPoint`
   `Vertex`/`Fragment` confirmed present in the re-parsed module) — new
   tests `translates_real_fxc_compiled_triangle_vs_dxbc_to_valid_vertex_spirv`
   / `_triangle_ps_dxbc_to_valid_fragment_spirv`.
2. The real Vulkan SDK's own validator was run against both emitted
   modules (dumped to files via a new `examples/dump_graphics_spirv.rs`):
   ```
   $ /c/VulkanSDK/1.4.350.0/Bin/spirv-val.exe triangle_vs.spv; echo "exit=$?"
   exit=0
   $ /c/VulkanSDK/1.4.350.0/Bin/spirv-val.exe triangle_ps.spv; echo "exit=$?"
   exit=0
   ```
   No diagnostics were printed for either file — `spirv-val` prints
   nothing on success, so the two blank outputs above plus the `exit=0`
   codes are the real, unedited terminal output.

Regression tests added: `vertex_translator_honestly_rejects_the_pixel_shader_and_vice_versa`
(cross-feeding VS DXBC to `translate_pixel_shader` and vice versa both
fail), `graphics_translators_honestly_reject_garbage_bytes`, and
`compute_translators_still_honestly_reject_graphics_shaders` (confirms
`translate_shader`/`translate_chain_shader` still reject both graphics
shaders — the pre-existing "no false positive" guarantee is unbroken).
`cargo test --workspace --lib` passes all 33 unit tests (27 pre-existing
+ 6 new); `cargo test --workspace --test '*'` re-confirms all 6
real-hardware Compute Shader tests (4 single-op DXBC + 1 chain DXBC + 1
DXIL) still pass unchanged. `cargo build --workspace` /
`cargo clippy --workspace --all-targets` are both clean (0 warnings).

**Honest milestone reached — no further**: real SPIR-V generation for
both shaders, validated by two independent tools. **No rasterizer, no
output-merger/framebuffer, no actual Vulkan draw call, no rendered
pixel readback.** This is not an oversight or a time-boxing shortcut
taken lightly — `opencuda-vulkan`'s real source
(`../open-cuda/crates/opencuda-vulkan/src/{lib,real}.rs`) was read and
confirmed to contain zero `VkGraphicsPipelineCreateInfo`/render-pass/
framebuffer code; it is a Compute-dispatch-only backend (`ash` is
already a transitive dev-dependency here only via its `real-vulkan`
feature, gated to the compute path). Actually drawing the triangle
would require either (a) extending `opencuda-vulkan` with graphics-
pipeline support — explicitly out of scope per this project's
"depend on open-cuda, don't modify it" convention — or (b) adding `ash`
as a **direct** dependency of `open-directx` itself and hand-rolling a
minimal `VkGraphicsPipelineCreateInfo` + render pass + framebuffer +
draw call + readback. Option (b) is a legitimate next increment but was
not attempted this pass, in keeping with this project's "narrow but
real, not a stretch claim" discipline — the SPIR-V groundwork above is
solid and independently validated; the draw call is honestly left as
the next step, not silently skipped.

## What is NOT yet reusable (honest gaps)

- **No general SM5.0 instruction decoder.** Only the single-op shapes
  (`decode_shader_shape`, 4 opcodes) and the sequential-chain pattern
  class (`decode_chain_shape`, add/mul only, no control flow) are
  handled. A different D3D11 compute shader (different resource
  count/types, real branches/loops beyond a single top-level bounds
  check, intrinsics beyond `SV_DispatchThreadID` indexing, `sub`/`div`
  inside a chain, etc.) will be rejected by one of these decoders, not
  silently mistranslated.
- **DXIL (SM6+): the `vector_add.dxil` vertical slice is complete on
  real hardware (see the dedicated section above, updated 2026-07-25),
  but only for this one known shader shape — not a general SM6.0
  decoder.** Its SPIR-V workgroup size is now genuinely extracted from
  `METADATA_BLOCK` (no longer hardcoded, see the update above); any
  other operation, basic-block count, or bounds-check shape is still
  rejected. D3D12's higher-level layers (command lists, descriptor
  heaps, root signatures) remain entirely unimplemented, Phase 3+ per
  `CLAUDE.md`'s roadmap.
- **D3D11 graphics pipeline: real SPIR-V generation for the specific
  `triangle_vs.hlsl`/`triangle_ps.hlsl` pair is done and validated
  (`rspirv` re-parse + real `spirv-val.exe`, see the dedicated section
  above, added 2026-07-25), but this is not a general VS/PS decoder —
  any other vertex/pixel shader (different semantics, more than one
  `mov`-chain, texture sampling, multiple render targets, etc.) is
  rejected, not mistranslated. There is still no rasterizer, no texture
  sampler, no blend state, no output-merger, and no actual Vulkan
  triangle draw — `opencuda-vulkan` was confirmed by reading its source
  to be Compute-dispatch-only with zero graphics-pipeline code.**
  **Update 2026-07-26: the actual draw call now exists.** New crate
  `crates/directx-graphics-vulkan` adds `ash` as a direct dependency of
  this workspace (not layered on `opencuda-vulkan`) and implements a real
  render pass + framebuffer + `VkGraphicsPipelineCreateInfo`, reusing the
  SPIR-V above unmodified. It draws one full-viewport "big triangle" with
  a uniform vertex color and reads the rendered image back through a
  host-visible staging buffer; the real-hardware test asserts all
  read-back pixels match the passthrough vertex color on the real NVIDIA
  GT 730 present on this machine (`cargo test -p directx-graphics-vulkan
  --test triangle_real_vulkan -- --nocapture`: 1 passed, see `CLAUDE.md`
  HANDOFF 2026-07-26 continuation for full transcript). Still no depth
  buffer, texture sampler, swapchain/on-screen presentation, multiple
  triangles, or interpolation check across differently-colored vertices —
  those remain out of scope for this pass.

## Path-dependency convention used in this ecosystem (for reference)

Confirmed by reading sibling repos before adding any dependency here:

- `aruaru-llm/Cargo.toml`: `opencuda-core`, `opencuda-cpu`,
  `opencuda-blas`, `opencuda-bert` all as `{ path = "../open-cuda/crates/<name>" }`.
- `aruaru-db/Cargo.toml` (workspace root): `rust-json = { path = "../RS-JSON" }`
  under `[workspace.dependencies]`, with a comment explaining the
  sibling-repo-under-`F:\runo` convention.

This project follows the same pattern, but one directory level deeper
(this crate lives at `open-directx/crates/directx-shader-translate/`, two
levels below the `F:\runo` sibling root, not one):

```toml
# crates/directx-shader-translate/Cargo.toml, [dev-dependencies]
opencuda-core = { path = "../../../open-cuda/crates/opencuda-core" }
opencuda-vulkan = { path = "../../../open-cuda/crates/opencuda-vulkan", features = ["real-vulkan"] }
```

These are **dev-dependencies only** — the published library
(`directx-shader-translate`'s non-test code) does not depend on
`open-cuda` at all; only the real-hardware dispatch test in `tests/`
does. A downstream consumer that wants to actually dispatch translated
SPIR-V is expected to depend on `opencuda-vulkan` itself, the same way
this crate's test does.

## Bounds-checked chain generalization now covers 5 terms + kernel-level anti-cheat scope note (2026-08-06)

The bounds-checked binary-op chain decoder (`decode_chain_shape` for
DXBC, `resolve_dxil_calls_and_chain` for DXIL) has been exercised up to
a **5-term** chain (`add->mul->div->sub->add`) on both DXBC and DXIL,
with zero production-code changes required each time a new term count
was added — only new compiled shaders + real-hardware tests. This is
strong evidence the generalized instruction-walking approach (not
per-shape hardcoding) was the right call from the start.

## H.264/H.265/HEVC shader feasibility research, closed; FFv1 identified as the realistic next target (2026-09-12)

Following up on the yuv444_to_rgb prototype (see the entry above dated
2026-09-12 in `make-disk/PORTING.md` — R/B verified on real GT730
hardware, G verified structurally only), the user asked for a further,
world-language (Google + GitHub, multiple languages) research pass on
whether H.264/H.265/HEVC encoder/decoder/shader work is feasible for
this project's target GPU (GT730), and specifically to read and follow
up on Khronos's own blog post about FFmpeg's Vulkan compute video work.

**Two distinct GPU-video mechanisms, not to be confused**:
1. **Vulkan Video extension** (`VK_KHR_video_*`) — calls into the GPU
   vendor's dedicated fixed-function video ASIC. No shader code runs the
   actual codec; the chip's hardware block does it. Confirmed via
   `vulkaninfo` earlier in this project that GT730 exposes **zero**
   `VK_KHR_video_*` extensions — this path is closed for this GPU,
   permanently (a driver update cannot add hardware that isn't there).
2. **Generic Vulkan **compute shaders*** — an ordinary compute
   dispatch, like every other kernel in this repo, implementing the
   codec's math in GLSL/SPIR-V. This is the path Khronos's blog post
   and FFmpeg's `cyanreg/FFmpeg` `vulkan` branch actually use, and it
   works on any Vulkan 1.3-capable GPU including old ones like GT730 —
   **no vendor video ASIC required**.

**What the Khronos blog (["Video Encoding and Decoding with Vulkan
Compute Shaders in FFmpeg"](https://www.khronos.org/blog/video-encoding-and-decoding-with-vulkan-compute-shaders-in-ffmpeg))
actually says**: FFmpeg 8.1 shipped pure-compute-shader Vulkan
encode/decode for **FFv1** (both directions), **ProRes** (both
directions), **ProRes RAW** decode, and DPX unpacking. VC-2 and APV are
still in progress. Crucially, the article explains *why these formats
specifically*: they either have no entropy coder at all (ProRes, DPX)
or an entropy coder that is line/slice-parallel by design (FFv1's range
coder, workable via a 32-wide subgroup where 32 lanes do lookup+adapt in
parallel while one lane serializes the actual bit output). This is the
opposite structural shape from H.264/H.265/HEVC's CABAC, which is
inherently one-bit-at-a-time serial with no such parallel decomposition
— exactly the academic conclusion already recorded in this repo's
2026-09-12 (H.264/HEVC) research entry, now independently confirmed by
FFmpeg's own maintainers choosing not to attempt CABAC this way.

**Conclusion on H.264/H.265/HEVC**: unchanged, and now doubly
confirmed — not recommended for this project, on this GPU, via either
mechanism. This is a closed research question.

**FFv1 identified as the realistic next target, if one is wanted**:
FFv1 is a real IETF-standardized (RFC 9043), open, royalty-free,
mathematically lossless codec used heavily in the archival/preservation
community (its designers explicitly optimized for GPU/SIMD-style
parallelism from the start: up to 1024 independent slices per frame,
line-parallel prediction). FFmpeg's `cyanreg/FFmpeg` `vulkan` branch
(mailing-list patch series, `ffv1dec_vulkan`/`ffv1_vulkan`, landed
2025) is real, working, upstream-track proof this is achievable on
ordinary Vulkan 1.3 compute, no special hardware required — matching
exactly the kind of "small kernel, grown incrementally" approach this
repo already uses for the yuv444_to_rgb prototype.

**Honest scoping of what a first step here would look like** (not yet
started — this entry records research only, per this task's own
instruction to research before committing to an implementation plan):
FFv1's median/gradient (MED) spatial predictor and its RGB reversible
color transform (RCT, `G, B-G, R-G` plus a wraparound mod so the
transform is exactly invertible in integer arithmetic) are ordinary
per-pixel compute-shader work, structurally similar to the
`yuv444_to_rgb` kernels already proven here. The genuinely hard part —
and the part FFmpeg's own developers call out as the biggest challenge
— is the **adaptive range coder**: each symbol's each bit carries its
own running 8-bit adaptation state, so a naive per-pixel-independent
shader does not work; it needs the 32-wide "31 lanes help, 1 lane
serializes" subgroup trick described above, which is real shader
architecture work, not a simple kernel port. The realistic incremental
path, if this is picked up: (1) MED predictor kernel first (no entropy
coding, straightforward per-pixel like yuv444_to_rgb), verified on real
GT730 hardware; (2) RCT kernel next; (3) the range-coder subgroup kernel
last, as its own dedicated research-plus-implementation pass, since it
is the one piece with no existing precedent in this repo's codebase to
generalize from.

## FFv1 step 1 implemented: MED-predictor comparator (max/min) decoding, real-GPU-verified (2026-09-12, same day)

Following the research entry above, actually started on the honestly-
scoped first step ("MED predictor kernel first ... verified on real
GT730 hardware"): the comparator primitive MED's `min`/`max` calls need.

- Compiled two new minimal shaders with real `fxc.exe`:
  `shaders/vector_max.hlsl` (`Output[i] = max(A[i], B[i])`) and
  `shaders/vector_min.hlsl` (the `min` counterpart). Dumped their real
  SHEX instruction streams with `examples/dump_shex` — confirmed HLSL's
  `max`/`min` compile to a single native `Opcode::Max`/`Opcode::Min`
  instruction with the exact same operand shape as `add`/`mul`/`div`
  (`dest, src1, src2`), **not** a compare-then-branch decomposition.
- Added `BinaryOp::Max`/`BinaryOp::Min` to `spirv_gen.rs`'s `RegExpr`
  chain decoder (`decode_chain_shape`, alongside the existing
  `Add|Mul|Div` arm — same negate-rejection convention: any negate flag
  on either source is rejected as unverified, matching existing
  practice for this decoder).
- Added real SPIR-V emission for both: rather than inventing a manual
  compare+select sequence, translated them to the **GLSL.std.450**
  extended-instruction-set `FMax`/`FMin` ops (`OpExtInst` via
  `Builder::ext_inst`, importing `"GLSL.std.450"` once per module) —
  this is the standard SPIR-V idiom for `max`/`min` (matches how GLSL's
  own `max`/`min` builtins lower) and mirrors the native-single-
  instruction shape actually observed in the DXBC.
- New real-hardware test `tests/vector_max_min_real_vulkan.rs`: both
  `dxbc_vector_max_matches_reference_on_real_vulkan_hardware` and
  `dxbc_vector_min_matches_reference_on_real_vulkan_hardware` **passed
  on this machine's real NVIDIA GT 730**, 256/256 elements exactly
  matching `f32::max`/`f32::min` CPU reference.
- `cargo test --workspace`: full suite green, zero regressions (all
  pre-existing real-Vulkan/real-D3D12 tests still pass).
- Added both compile steps to `tools/compile-dxbc-shaders.ps1`.

**Honest scope of what this does NOT yet cover**: this is the isolated
comparator primitive only, on the existing flat 1D-buffer chain
decoder. MED's actual `if/else if/else` 3-way branch structure, and its
2D neighbor addressing (left/top/top-left pixels, i.e. `x-1`/`y-1`
index arithmetic rather than a single `id.x`), are not implemented —
those are the next real sub-steps toward an actual MED kernel, not yet
attempted.

## Range coder prerequisite check: GT730 confirmed capable, not a hardware wall (2026-09-12, same day)

Continuing the world-language research pass on the range coder (the
piece FFmpeg's own developers call the hardest part, using a 32-wide
subgroup where 31 lanes do adaptation lookup in parallel and 1 lane
serializes the actual bit output — see the research entry above), a
further Google/GitHub search turned up the actual FFmpeg source: real
`.comp` GLSL files (`vulkan/common.comp`, `vulkan/ffv1_enc_ac.comp`) in
the `cyanreg/FFmpeg` `vulkan` branch, confirming this is real, shipped
GLSL, not merely a blog-post description. FFmpeg's own encoder patch
notes state the Vulkan FFv1 encoder "requires a Vulkan 1.3 supporting
GPU with the BDA (Buffer Device Address) extension" and uses subgroup
shuffle operations to distribute the adaptation work.

**Checked directly on this machine with `vulkaninfo`, rather than
assuming**: GT730 reports Vulkan API version **1.2.175** (not 1.3
core), but it does expose the two specific features FFmpeg's design
actually needs as extensions/properties:
- `VK_KHR_buffer_device_address` (revision 1) — present.
- `subgroupSize = 32` with `SUBGROUP_FEATURE_SHUFFLE_BIT` and
  `SUBGROUP_FEATURE_SHUFFLE_RELATIVE_BIT` set — present, and the
  subgroup size **exactly matches** the 32-wide design FFmpeg's own
  implementation uses (not a coincidence to rely on, but a good sign
  this GPU generation's SIMD width is the one this algorithm was
  designed around).

**This is the opposite finding from the H.264/H.265/HEVC research**:
that was a genuine, permanent hardware/driver ceiling (no
`VK_KHR_video_*` extensions exist on this chip, full stop). The range
coder's prerequisites are, by contrast, actually present on this GPU.
Whether `opencuda-vulkan`'s current device/instance setup already
requests the needed extension (it targets a narrower baseline today)
and whether a subgroup-shuffle-based kernel can be expressed through
this project's existing SPIR-V-emission approach are real open
implementation questions — but they are engineering work, not a closed
research question the way CABAC was. This is recorded as the concrete
starting point for the next work session on FFv1's hardest remaining
piece.

**日本語(要約)**: FFv1の第一歩として、MED予測器が使う比較器
(`max`/`min`)命令のDXBC→SPIR-Vデコードを実装した。`vector_max.hlsl`/
`vector_min.hlsl`を実際に`fxc.exe`でコンパイルし、SHEXダンプで
`max`/`min`が比較+分岐への分解ではなくネイティブ単一命令であることを
確認、`BinaryOp::Max`/`BinaryOp::Min`をチェーンデコーダに追加し、
SPIR-V生成側はGLSL.std.450拡張命令`FMax`/`FMin`として翻訳した。実GT730
ハードウェアで256要素すべて数値一致を確認、ワークスペース全体の既存
テストに回帰なし。**未実装(正直な開示)**: MEDの3分岐構造そのものと、
2次元近傍参照(left/top/top-left、`x-1`/`y-1`のインデックス計算)は
まだ手つかず。また、最難関のレンジコーダーについて、FFmpeg本家が
要求する`VK_KHR_buffer_device_address`拡張と32レーンsubgroup shuffle
(`SUBGROUP_FEATURE_SHUFFLE_BIT`)の両方を、この開発機のGT730が
`vulkaninfo`で実際にサポートしていることを確認した(`subgroupSize=32`、
FFmpegの設計と一致)。H.264/H.265/HEVCのような恒久的なハードウェアの
壁ではなく、実装すれば動く可能性がある前向きな発見であり、次回
セッションの具体的な着手点として記録する。

**Scope note for anyone porting this project into a "run real Windows
games on Linux" context**: kernel-level anti-cheat (Riot Vanguard,
kernel-mode BattlEye, etc.) blocks Linux/Proton-style environments by
design, independent of how complete this shader-translation layer gets.
This is not a defect to "fix" — see `CLAUDE.md`'s 2026-08-06 HANDOFF
entry for the full honest disclosure. Titles using such anti-cheat are
out of reach for this project regardless of translation completeness.

## FFv1 progress (same day, continued): MED predictor completed end-to-end, G-channel real-GPU blocker resolved, subgroup shuffle proven on real hardware (2026-09-12)

Continuing directly from the comparator (max/min) step above, three more
real, tested pieces landed in the same session:

**1. Generic N-buffer dispatch added to `open-cuda` (resolves a
previously-recorded top-priority blocker)**. `opencuda-vulkan`'s
internal `dispatch_spirv` was already buffer-count-generic
(`buffers: &[vk::Buffer]`), but the only public entry points
(`launch_kernel` dispatching by kernel name) hardcoded 3 buffers for
`"vector_add"`. Added `"chain_n_buffer"`/`"chain_n_buffer_f32"`
(`ensure_chain_n_buffer_args`/`run_chain_n_buffer_spirv` in
`opencuda-vulkan/src/real.rs`) accepting any number of `KernelArg::Ptr`
buffers followed by one `KernelArg::Usize(n)`. Purely additive — no
existing kernel name's behavior changed. See `open-cuda/PORTING.md` for
that repo's own record of this change.

**2. `yuv444_to_g_real_vulkan.rs` upgraded from structural-only to a
real numeric GPU test.** With the generic dispatch available, the
4-buffer G-channel kernel (previously blocked, see the 2026-09-12
"yuv444_to_g" entry above) now actually dispatches and **matches the
BT.601 CPU reference across all 256 elements on real GT730 hardware**.
This closes out the yuv444_to_rgb prototype fully — R, G, and B are now
all real-GPU-verified, not just R and B.

**3. MED predictor implemented end-to-end and real-GPU-verified.**
Compiled `shaders/med_predictor.hlsl` (the textbook FFv1/JPEG-LS MED
predictor: `if (topleft>=max(left,top)) pred=min(left,top); else if
(topleft<=min(left,top)) pred=max(left,top); else pred=left+top-topleft;`)
with real `fxc.exe` and dumped its SHEX with `examples/dump_shex`. The
key discovery: **fxc flattens the entire 3-way if/else-if/else into
branch-free `ge` (comparison) + `movc` (conditional move) instructions
— there is no actual control flow in the compiled shader at all**. This
happens to fit perfectly into this decoder's existing "control-flow-free
expression tree" model, so no branching support was needed:
- Added `RegExpr::Ge(lhs, rhs)` (bool-valued comparison node) and
  `RegExpr::Select { cond, then_expr, else_expr }` (the `movc`
  equivalent) to `spirv_gen.rs`.
- Verified empirically (not assumed) that DXBC's `ge` instruction means
  `dest = (src1 >= src2)` directly — the opposite convention from
  `add`/`mul`/`div`'s "src2 OP src1" reversal — by checking both `ge`
  instances against the known HLSL source.
- `movc dest, cond, then, else` decodes directly to `RegExpr::Select`;
  the decoder requires `cond` to resolve to exactly a prior `Ge` result
  (rejects anything else as unverified, per this file's existing
  scope-honesty convention).
- SPIR-V emission: `Ge` → `OpFOrdGreaterThanEqual`, `Select` →
  **`OpSelect`** (SPIR-V's own branch-free scalar select) — mirroring
  the branch-free shape actually observed in the DXBC, not inventing an
  `OpBranchConditional`-based decomposition.
- New test `tests/med_predictor_real_vulkan.rs`: test data was chosen
  to actually exercise **all three MED branches** (asserted via branch
  counters in the test itself, not just "some passing case") — **passed
  on real GT730 hardware**, all 256 elements matching the Rust
  reference implementation of the MED predictor.
- `cargo test --workspace`: full suite green, zero regressions.
  `cargo clippy -p directx-shader-translate --all-targets -- -D
  warnings`: clean except one pre-existing, unrelated lint in
  `dxil.rs` (confirmed via a clean checkout diff that this session did
  not touch that file or introduce that lint).

**Honest scope still remaining for a "real" MED kernel**: this
prototype takes left/top/topleft as three pre-sliced flat buffers (the
same simplification `yuv444_to_rgb` used for Y/U/V) — actual 2D image
neighbor addressing (`x-1`/`y-1` index arithmetic into a single 2D
image bubuffer) is not implemented. That remains the next real step
before this is a MED kernel usable on an actual image plane.

**4. Range coder prerequisite proven on real hardware, not just
`vulkaninfo`-confirmed.** Built a standalone SPIR-V kernel directly via
`rspirv` (there is no DXBC/SM5.0 equivalent of subgroup operations to
translate from — D3D12/SM6.0's `WaveReadLaneAt` is the closest DXIL
analog, out of this crate's DXBC scope — so this is hand-built SPIR-V,
not a DXBC translation) using `OpGroupNonUniformShuffle` (SPIR-V 1.3,
`GroupNonUniform`+`GroupNonUniformShuffle` capabilities) to swap values
between lane `2k` and lane `2k+1` within each 32-wide subgroup.
`tests/subgroup_shuffle_real_vulkan.rs`: **passed on real GT730
hardware**, all 64 test elements (2 workgroups of 32) showing exactly
the expected lane-swap pattern, dispatched through the same
`chain_n_buffer` generic path added in item 1. This directly confirms
— through this project's own translation-and-dispatch pipeline, not
merely a capability-bit query — that the core mechanism FFv1's range
coder needs (32-lane subgroup shuffle) actually works on this GPU.

**日本語(要約)**: 同日中にさらに3つの実装を完成させた。(1)
`open-cuda`に汎用Nバッファディスパッチ(`chain_n_buffer`)を追加し、
以前「最優先」と記録していたブロッカーを解消。(2)これによりGチャンネル
(4バッファ)を構造検証止まりから**実GT730ハードウェアでの数値検証
(256/256要素一致)へ格上げ**——yuv444_to_rgbプロトタイプがR/G/B全て
実機検証済みになった。(3)MED予測器を実装——fxc.exeが3分岐if/else
if/elseを**分岐命令を一切使わず**`ge`(比較)+`movc`(条件付き代入)へ
平坦化することを実SHEXダンプで発見し、`RegExpr::Ge`/`RegExpr::Select`
(SPIR-Vの`OpSelect`、分岐無し)を追加。新規テストは3分岐すべてを
実際に踏んだ上で**実GT730ハードウェアで256要素すべて数値一致**。
未実装として正直に開示: 実画像の2次元近傍参照(`x-1`/`y-1`)は今回も
対象外(yuv444_to_rgbと同じ簡略化)。(4)レンジコーダーの核心である
32レーンsubgroup shuffleを、`rspirv`で直接組み立てたSPIR-V
(`OpGroupNonUniformShuffle`)として実装し、**実GT730ハードウェアで
期待通りのレーン交換を確認**——`vulkaninfo`の申告を鵜呑みにせず、
このプロジェクト自身の翻訳・ディスパッチ経路で裏付けた。
ワークスペース全体で回帰無し。

**次にすべきこと**: (a) MEDの2次元近傍参照(実画像バッファへの
`x-1`/`y-1`インデックス計算)、(b) レンジコーダーの状態遷移テーブル・
適応ロジック本体(今回証明したのはあくまで「32レーンが値を交換できる」
という土台のみ、実際の適応型エントロピー符号化はまだ手つかず)。
