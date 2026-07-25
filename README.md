# open-directx

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
  -> ExtractValue -> BinOp -> Call -> Ret`). **Still no DXIL-to-SPIR-V
  translation** — DXIL routes every intrinsic op through an ordinary
  LLVM `CALL`, so the 7 `Call` records are indistinguishable without
  resolving `VALUE_SYMTAB_BLOCK` function names and the relative-value
  operand encoding, neither of which is implemented yet. See "Not
  implemented" below.
- **D3D11 graphics pipeline — DXBC parsing only, no SPIR-V.**
  `shaders/triangle_vs.hlsl`/`shaders/triangle_ps.hlsl` (minimal
  passthrough vertex+pixel shader pair, `POSITION`/`COLOR` in,
  `SV_POSITION`/`SV_TARGET` out) compiled with real `fxc.exe
  /T vs_5_0`/`/T ps_5_0`. `parse_dxbc` (already existing, container-level
  only) parses both without modification — confirming the same DXBC
  container/chunk front-end works for graphics shaders, not just
  compute. Dumping the real SHEX stream with `examples/dump_shex.rs`
  confirmed the opcode/operand vocabulary is genuinely different from
  compute shaders: `dcl_input`/`dcl_input_ps`(with `linear`
  interpolation)/`dcl_output`/`dcl_output_siv`(`SV_POSITION`)/`mov` — no
  `dcl_uav_structured`, `ld_structured`/`store_structured`, or
  `dcl_thread_group` at all. `translate_shader` (compute-only) correctly
  rejects both with `SpirvGenError::UnsupportedShader` rather than
  attempting a wrong translation (verified by a new test). No SPIR-V
  codegen, rasterizer, or actual Vulkan triangle draw exists — that is
  explicitly out of scope for this pass, see `CLAUDE.md` HANDOFF.

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

## Build & test

```powershell
cargo build --workspace
cargo test --workspace -- --nocapture
```

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
- **DXIL (Shader Model 6+, D3D12): real bytes are now parsed down to a
  resolved type table and a coarse instruction list — but no further.**
  `resolve_type_table`/`decode_function_instructions` in `dxil.rs`
  decode real `TYPE_BLOCK`/`FUNCTION_BLOCK` records against LLVM's
  documented codes, and `decode_vector_add_dxil_shape` narrowly matches
  the exact instruction shape `vector_add.dxil` produces. **Still no
  DXIL-to-SPIR-V translation**: DXIL represents every intrinsic op
  (`CreateHandle`/`ThreadId`/`BufferLoad`/`BufferStore`) as an ordinary
  LLVM `Call`, and this code doesn't yet resolve `VALUE_SYMTAB_BLOCK`
  function names or LLVM bitcode's relative-value operand encoding, so
  it cannot tell which `Call` is which or recover UAV bind points. D3D12
  command list/descriptor heap/root signature support (the layer above
  shader translation) is untouched. Next step if this is picked up
  again: read `VALUE_SYMTAB_BLOCK` to name-resolve `Call` targets, then
  decode the relative-value operand encoding, before reusing
  `spirv_gen.rs`'s `emit_spirv` for a DXIL-sourced `vector_add`.
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
