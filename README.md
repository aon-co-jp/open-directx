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

## Current state (2026-07-25, Phase 0 -> Phase 1 vertical slice, in progress)

Only the **DXBC container/chunk parsing front-end** exists so far.
SPIR-V codegen and real Vulkan dispatch are **not implemented yet** —
do not assume they work.

- `crates/directx-shader-translate`: parses real DXBC containers
  (Shader Model <= 5.1, used by D3D9/10/11) produced by `fxc.exe`.
  Reuses the existing [`dxbc`](https://crates.io/crates/dxbc) crate
  (crates.io, MIT, round-trip verified against 1000+ real shaders) for
  the low-level chunk table/RDEF/ISGN/OSGN/SHEX decoding, and wraps it
  into a `ShaderModule` summary intended to feed a future SPIR-V
  code-generation backend.
- `crates/directx-shader-translate/shaders/vector_add.hlsl`: a trivial
  D3D11 compute shader (`RWStructuredBuffer` vector-add, SM5.0) — the
  target for the Phase 1 vertical slice (DXBC -> SPIR-V ->
  `opencuda-vulkan` dispatch -> CPU-reference numeric match). Only the
  DXBC-parsing half is done; translation/dispatch is future work.
- `crates/directx-shader-translate/shaders/vector_add.dxbc`: the real
  compiled DXBC bytes (956 bytes), produced with
  `fxc.exe /T cs_5_0 /E main vector_add.hlsl`, committed as a test
  fixture (see `tools/compile-dxbc-shaders.ps1` to regenerate).

## Build & test

```powershell
cargo build --workspace
cargo test --workspace
```

Actually observed output (2026-07-25):

```
running 3 tests
test tests::rejects_garbage_bytes_that_are_not_a_dxbc_container ... ok
test tests::rejects_truncated_dxbc_header ... ok
test tests::parses_real_fxc_compiled_vector_add_dxbc_container ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

To regenerate the DXBC fixture from HLSL (requires the Windows SDK's
`fxc.exe` — note `dxc.exe` only targets DXIL/SM6+ and cannot produce
DXBC):

```powershell
pwsh tools/compile-dxbc-shaders.ps1
```

## Not implemented (honest scope)

- DXBC -> SPIR-V code generation (the actual instruction-stream
  translation). This is the bulk of the remaining Phase 1 work.
- DXIL (Shader Model 6+, D3D12) parsing/translation — out of scope
  until Phase 1 (D3D11 compute) is proven end-to-end.
- Any wiring to `opencuda-vulkan`'s `GpuDevice`/`KernelSource::SpirV`
  dispatch path — designed conceptually (see `PORTING.md`), not coded.
- Full graphics pipeline (rasterizer, texture sampling, blend state) —
  explicitly out of scope until the compute vertical slice works.
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
