# PORTING.md — what's reusable, by whom, and how

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

## Known-shader SPIR-V translation (`spirv_gen` module, added 2026-07-25)

```rust
pub struct TranslatedKernel {
    pub spirv_words: Vec<u32>,
    pub entry_point: &'static str,        // always "main"
    pub local_size: (u32, u32, u32),      // extracted from dcl_thread_group
    pub uav_bind_points: (u32, u32, u32), // extracted from RDEF/ld_structured/store_structured
}

pub fn translate_vector_add_shader(bytes: &[u8]) -> Result<TranslatedKernel, SpirvGenError>;
```

**Honest scope**: this is *not* a general SM5.0 decoder. It recognizes
exactly the narrow opcode sequence that `fxc.exe` actually emits for
`shaders/vector_add.hlsl` (`dcl_globalFlags` -> `dcl_uav_structured`x3 ->
`dcl_input`(vThreadID) -> `dcl_temps` -> `dcl_thread_group` ->
`ld_structured`x2 -> `add` -> `store_structured` -> `ret`). Any other
opcode, or a different shape (e.g. more than 2 reads, a different
register class), is rejected with `SpirvGenError::UnsupportedShader`
rather than silently mistranslated. The UAV bind points and thread-group
size embedded in the emitted SPIR-V are **not hardcoded** — they come
from actually parsing the real DXBC container's `RDEF`/`SHEX` chunks via
the `dxbc` crate. The SPIR-V binary itself is assembled with `rspirv`
(not hand-rolled binary bytes), and its self-consistency is validated by
re-parsing it with `rspirv::binary::parse_bytes` in the test suite.

**Verified end-to-end (2026-07-25)**: `crates/directx-shader-translate/tests/vector_add_real_vulkan.rs`
parses the real `fxc.exe`-compiled `vector_add.dxbc` fixture, translates
it with `translate_vector_add_shader`, dispatches the resulting SPIR-V
through `open-cuda`'s real `opencuda-vulkan::VulkanDevice` (`ash`-based,
`real-vulkan` feature) on this machine's NVIDIA GeForce GT 730, and
confirms the GPU output matches a CPU reference `a[i]+b[i]` for all 256
elements within 1e-3. See `CLAUDE.md`'s HANDOFF (2026-07-25 continuation
entry) for the exact `cargo test` output.

```rust
// actual, working (not conceptual) — see tests/vector_add_real_vulkan.rs
let kernel = directx_shader_translate::translate_vector_add_shader(&dxbc_bytes)?;
let spirv_bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
let compiled = opencuda_core::CompiledKernel::spirv("vector_add", kernel.entry_point, spirv_bytes);
device.launch_kernel(&compiled, &cfg, &args)?; // opencuda_vulkan::VulkanDevice, real hardware
```

## What is NOT yet reusable (honest gaps)

- **No general SM5.0 instruction decoder.** Only the exact
  `vector_add.hlsl` opcode shape above is handled. A different D3D11
  compute shader (different resource count/types, control flow,
  intrinsics beyond `SV_DispatchThreadID` indexing, etc.) will be
  rejected by `decode_vector_add_shape`, not silently mistranslated.
- **No DXIL (SM6+) support in this crate.** The underlying `dxbc` crate
  parses the `DXIL` chunk as an opaque LLVM-bitcode blob (see its
  `ChunkData::Dxil` variant) but does not decode it. D3D12 support
  remains Phase 3+ per `CLAUDE.md`'s roadmap.
- **No D3D11 graphics pipeline (vertex/pixel shaders, rasterizer, etc.).**
  Compute-only, per the vertical-slice scope in `CLAUDE.md`.

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
