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

## Current state (2026-07-25, Phase 1 vertical slice achieved for one known shader)

`crates/directx-shader-translate` now does the full vertical slice for
**one specific known shader** (`vector_add.hlsl`): DXBC parse -> narrow
SM5.0 opcode-subset decode -> SPIR-V codegen (via `rspirv`) -> real
Vulkan dispatch (`open-cuda`'s `opencuda-vulkan`) -> CPU-reference
numeric match, verified on this machine's real NVIDIA GT 730. **This is
not a general SM5.0-to-SPIR-V decoder** — see "Not implemented" below.

- `parse_dxbc` (Phase 0): DXBC container/chunk introspection (RDEF/ISGN/
  OSGN/SHEX presence), unchanged from the original front-end.
- `spirv_gen::translate_vector_add_shader` (Phase 1, new 2026-07-25):
  recognizes the exact narrow opcode sequence `fxc.exe` emits for
  `vector_add.hlsl` (3x `dcl_uav_structured` + `dcl_thread_group` +
  2x `ld_structured` + `add` + `store_structured` + `ret`), rejecting
  anything else via `SpirvGenError::UnsupportedShader` rather than
  silently mistranslating. UAV bind points and thread-group size are
  extracted from the real parsed DXBC, not hardcoded. Emits a real
  SPIR-V module via `rspirv::dr::Builder`.
- `tests/vector_add_real_vulkan.rs`: dispatches the translated SPIR-V
  through `open-cuda`'s real `opencuda-vulkan::VulkanDevice` (`ash`,
  `real-vulkan` feature) and checks the GPU output against a CPU
  reference `a[i]+b[i]` for 256 elements (1e-3 epsilon).

## Build & test

```powershell
cargo build --workspace
cargo test --workspace --release -- --nocapture
```

Actually observed output (2026-07-25, this machine, NVIDIA GeForce GT 730):

```
running 5 tests
test spirv_gen::tests::rejects_garbage_bytes_honestly_instead_of_pretending_to_translate ... ok
test tests::rejects_truncated_dxbc_header ... ok
test tests::parses_real_fxc_compiled_vector_add_dxbc_container ... ok
test tests::rejects_garbage_bytes_that_are_not_a_dxbc_container ... ok
test spirv_gen::tests::translates_real_fxc_compiled_vector_add_dxbc_to_valid_spirv ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 1 test
device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
OK: DXBC(fxc.exe実コンパイル)->SPIR-V(自前生成)->実Vulkan(NVIDIA GT 730)経路が、CPU参照実装(a[i]+b[i])と256要素すべてで数値一致した
c[0]=128, c[255]=255.5
test dxbc_vector_add_matches_cpu_reference_on_real_vulkan_hardware ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s
```

To regenerate the DXBC fixture from HLSL (requires the Windows SDK's
`fxc.exe` — note `dxc.exe` only targets DXIL/SM6+ and cannot produce
DXBC):

```powershell
pwsh tools/compile-dxbc-shaders.ps1
```

## Not implemented (honest scope)

- **General SM5.0 instruction decoding.** Only the exact `vector_add.hlsl`
  opcode shape is handled; any other D3D11 compute shader (different
  resource layout, control flow, other intrinsics) is rejected, not
  mistranslated. Building a real general decoder (or adopting/porting an
  existing one, e.g. studying `dxbc-spirv`/`dxil-spirv`'s approach more
  closely) remains the actual next milestone.
- DXIL (Shader Model 6+, D3D12) parsing/translation — out of scope
  until general SM5.0 decoding is done.
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
