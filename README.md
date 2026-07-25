# open-directx

A cross-platform DirectX (D3D9/10/11/12) compatibility layer — in the
spirit of DXVK / vkd3d-proton — aiming to run unmodified Windows
DirectX applications on Linux (and eventually Android/macOS) by
translating DXBC/DXIL shader bytecode to SPIR-V and dispatching through
an existing Vulkan compute backend ([open-cuda](https://github.com/aon-co-jp/open-cuda)'s
`opencuda-vulkan`).

See [`CLAUDE.md`](CLAUDE.md) for the full design rationale, honest
scope/roadmap, and session HANDOFF log — this README only summarizes
current, verified state.

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
- **DXIL (Shader Model 6+, D3D12) parsing/translation — investigated at
  the container level only (2026-07-25), not implemented.** DXIL is
  LLVM 3.7-era bitcode wrapped in a `DXIL` part inside the same
  DXContainer/DXBC-style outer container (ProgramHeader + BitcodeHeader
  + serialized LLVM IR module, magic `0x4C495844`). LLVM's own docs now
  describe this container format and a native-LLVM DXIL backend
  architecture (`llvm.org/docs/DirectX/DXContainer.html`,
  `.../DXILArchitecture.html`) — this is newer/more official coverage
  than existed when this was last surveyed. Candidate Rust building
  blocks if this is picked up later: the `dxbc` crate already used here
  exposes the `DXIL` chunk as an opaque blob (no decode); a generic
  `llvm-bitcode` crate exists on crates.io for the bitcode layer itself.
  No DXIL bytes have been parsed in this repo; this section is
  container-format research only, matching the depth of the original
  Phase 0 DXBC container research.
- Full graphics pipeline (rasterizer, texture sampling, blend state) —
  explicitly out of scope until then.
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
