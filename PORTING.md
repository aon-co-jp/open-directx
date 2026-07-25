# PORTING.md — what's reusable, by whom, and how

## `crates/directx-shader-translate`

**Reusable by**: any Rust project that needs to inspect DXBC (D3D9/10/11
shader bytecode, Shader Model <= 5.1) containers — e.g. a future D3D9/10/11
graphics-pipeline layer in this same repo, a shader-cache/asset-pipeline
tool, or a completely unrelated project that needs to read `.cso`/`.fxc`
shader blobs.

**How**: path dependency, same convention used elsewhere in this
ecosystem (e.g. `aruaru-llm/Cargo.toml`'s
`opencuda-core = { path = "../open-cuda/crates/opencuda-core" }`, or
`aruaru-db`'s `rust-json = { path = "../RS-JSON" }`):

```toml
[dependencies]
directx-shader-translate = { path = "../open-directx/crates/directx-shader-translate" }
```

Public API surface (current, Phase 0 front-end only):

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
(e.g. actual resource-binding slot numbers for SPIR-V descriptor-set
generation) should depend on the `dxbc` crate directly — this crate does
not hide it, it re-exports nothing exclusive.

## What is NOT yet reusable (honest gaps)

- **No SPIR-V codegen.** There is no `translate_to_spirv()` function of
  any kind yet. A consumer wanting actual GPU dispatch today gets
  nothing runnable — only container introspection.
- **No DXIL (SM6+) support in this crate.** The underlying `dxbc` crate
  parses the `DXIL` chunk as an opaque LLVM-bitcode blob (see its
  `ChunkData::Dxil` variant) but does not decode it; this project has
  not built anything on top of that yet either. D3D12 support is Phase
  3+ per `CLAUDE.md`'s roadmap, after DXBC (D3D9/10/11) and DXIL are
  both handled by a real backend.
- **No `opencuda-vulkan` wiring.** The intended integration point is:

  ```rust
  // conceptual, NOT implemented:
  let module = directx_shader_translate::parse_dxbc(&dxbc_bytes)?;
  let spirv: Vec<u8> = translate_to_spirv(&module)?; // <- does not exist yet
  let kernel = opencuda_core::CompiledKernel::spirv("vector_add", "main", spirv);
  device.launch_kernel(&kernel, &cfg, &args)?; // opencuda_core::GpuDevice
  ```

  `opencuda_core::KernelSource::SpirV(Vec<u8>)` and
  `CompiledKernel::spirv(name, entry, bytes)` already exist and are
  stable (see `open-cuda/crates/opencuda-core/src/kernel.rs`) — the gap
  is entirely on this project's side (producing the `Vec<u8>` of valid
  SPIR-V), not on `open-cuda`'s side.

## Path-dependency convention used in this ecosystem (for reference)

Confirmed by reading sibling repos before adding any dependency here:

- `aruaru-llm/Cargo.toml`: `opencuda-core`, `opencuda-cpu`,
  `opencuda-blas`, `opencuda-bert` all as `{ path = "../open-cuda/crates/<name>" }`.
- `aruaru-db/Cargo.toml` (workspace root): `rust-json = { path = "../RS-JSON" }`
  under `[workspace.dependencies]`, with a comment explaining the
  sibling-repo-under-`F:\runo` convention.

This project follows the same pattern (documented above) for the
not-yet-added `opencuda-vulkan`/`opencuda-core` integration, to be wired
once SPIR-V codegen actually exists — adding the path dependency before
there is real code to use it would be premature.
