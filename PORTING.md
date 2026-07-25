# PORTING.md — what's reusable, by whom, and how

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

**Where this still stops, honestly**: DXIL represents every intrinsic
op (`CreateHandle`/`ThreadId`/`BufferLoad`/`BufferStore`) as an
ordinary LLVM `CALL` to a `dx.op.*` function, so all 7 real `Call`
records in `vector_add.dxil` are indistinguishable from each other in
this code — `VALUE_SYMTAB_BLOCK` (function-name resolution) is not read
yet, and LLVM bitcode's relative-value-reference operand encoding is
not decoded, so no UAV bind point can be recovered from a `Call`'s
`fields`. Without that, there is **still no DXIL-to-SPIR-V translation
of any kind** — the type table and instruction list exist, but nothing
consumes them to emit SPIR-V. If this is picked up again, the natural
next step is reading `VALUE_SYMTAB_BLOCK` to name-resolve the `Call`
targets, then decoding the relative-value operand encoding to recover
actual UAV bindings, before attempting to reuse `spirv_gen.rs`'s
`emit_spirv` for a DXIL-sourced `vector_add`.

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

**Honest scope**: no SPIR-V generation for graphics shaders exists, no
rasterizer, no output-merger, no actual Vulkan triangle draw. This pass
only establishes that the DXBC container front-end generalizes to
graphics shaders and records what their SHEX opcode vocabulary actually
looks like, as the documented prerequisite for eventually extending
`spirv_gen` (or writing a parallel graphics-focused decoder) to cover
this pipeline stage.

## What is NOT yet reusable (honest gaps)

- **No general SM5.0 instruction decoder.** Only the 3 opcode shapes
  above are handled. A different D3D11 compute shader (different
  resource count/types, other control flow, intrinsics beyond
  `SV_DispatchThreadID` indexing, a real dedicated `sub`/`div` opcode
  instead of negated-`add`, more than one bounds check, etc.) will be
  rejected by `decode_shader_shape`, not silently mistranslated.
- **DXIL (SM6+) is parsed down to a resolved type table + a coarse
  instruction list (see the dedicated section above, updated
  2026-07-25) but no further — no `Call`-target name resolution, no
  operand/relative-value decoding, no DXIL-to-SPIR-V translation.**
  D3D12's higher-level layers (command lists, descriptor heaps, root
  signatures) remain entirely unimplemented, Phase 3+ per `CLAUDE.md`'s
  roadmap.
- **D3D11 graphics pipeline: DXBC container parsing for vertex/pixel
  shaders is confirmed working (see the dedicated section above, added
  2026-07-25), but there is no SPIR-V generation, rasterizer, texture
  sampler, blend state, output-merger, or actual Vulkan triangle draw.**
  Compute-only SPIR-V codegen, per the original vertical-slice scope in
  `CLAUDE.md`.

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
