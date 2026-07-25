//! DXBC (D3D9/10/11 シェーダーバイトコード, Shader Model <= 5.1) の
//! コンテナ/チャンク解析。
//!
//! open-directxのPhase 1垂直スライス(D3D11 Compute Shaderを実際に
//! DXBC->SPIR-Vへ翻訳し、`open-cuda`の`opencuda-vulkan`
//! (`GpuDevice`/`KernelSource::SpirV`)経由でVulkan実行し、CPU参照実装と
//! 数値一致することを実証する)の最初の段階。
//!
//! **現状のスコープ(正直な開示)**: このクレートは「DXBCコンテナを解析し、
//! リソースバインディング(RDEF)・入出力シグネチャ(ISGN/OSGN)・
//! シェーダー命令列(SHEX)の存在を検証する」フロントエンドのみを実装する。
//! SPIR-Vコード生成(バックエンド)は未実装(次フェーズ)。
//!
//! コンテナ/チャンクの低レベル解析自体は、車輪の再発明を避けるため
//! 既存の [`dxbc`クレート](https://crates.io/crates/dxbc)(fxc.exe出力での
//! 往復検証済み、MIT)を再利用する。このクレートはその上に、
//! open-directxのSPIR-V翻訳パイプラインが必要とする形へ整理した薄い
//! ラッパーを提供する。

use dxbc::{scan_dxbc as dxbc_scan, ChunkData};
use thiserror::Error;

pub mod spirv_gen;
pub use spirv_gen::{translate_vector_add_shader, SpirvGenError, TranslatedKernel};

#[derive(Debug, Error)]
pub enum TranslateError {
    #[error("DXBCコンテナの解析に失敗した: {0}")]
    Parse(String),
    #[error("必須チャンクが見つからない: {0}")]
    MissingChunk(&'static str),
}

/// DXBCコンテナから抽出した、SPIR-V翻訳に必要な情報の要約。
///
/// フルパースした`RDEF`/`ISGN`/`OSGN`の詳細構造そのものではなく、
/// 「翻訳パイプラインの次段(SPIR-Vコード生成)が必要とする最小限の要約」
/// として整理している。詳細情報が要る場合は`dxbc`クレートを直接使う。
#[derive(Debug, Clone)]
pub struct ShaderModule {
    /// コンテナ内のチャンク数。
    pub chunk_count: usize,
    /// RDEF(リソース定義)チャンクが存在したか。
    pub has_resource_definitions: bool,
    /// ISGN(入力シグネチャ)チャンクが存在したか。
    pub has_input_signature: bool,
    /// OSGN(出力シグネチャ)チャンクが存在したか。
    pub has_output_signature: bool,
    /// SHEX/SHDR(シェーダー本体命令列)チャンクの命令数
    /// (`dxbc`クレートがdisassembleできた場合のみ`Some`)。
    pub instruction_count: Option<usize>,
}

/// DXBCバイト列を解析し、[`ShaderModule`]要約を返す。
///
/// `dxbc`クレートの`scan_dxbc`は「バイト列中に含まれる全DXBCコンテナを
/// スキャンする」設計(単体の`.dxbc`/`.cso`ファイルなら先頭に1個見つかる)。
/// 先頭4バイトが`b"DXBC"`(コンテナマジック)であることを含め、コンテナが
/// 全く見つからない場合はエラーとして扱う。
pub fn parse_dxbc(bytes: &[u8]) -> Result<ShaderModule, TranslateError> {
    let containers = dxbc_scan(bytes);
    let container = containers
        .into_iter()
        .next()
        .ok_or_else(|| TranslateError::Parse("DXBCコンテナが見つからない(マジックバイト不一致または破損)".to_string()))?;

    let mut has_resource_definitions = false;
    let mut has_input_signature = false;
    let mut has_output_signature = false;
    let mut instruction_count = None;

    for chunk in &container.chunks {
        match chunk.parse() {
            ChunkData::Rdef(_) => has_resource_definitions = true,
            ChunkData::InputSignature(_) => has_input_signature = true,
            ChunkData::OutputSignature(_) => has_output_signature = true,
            ChunkData::Shader(program) => {
                instruction_count = Some(program.instructions.len());
            }
            _ => {}
        }
    }

    Ok(ShaderModule {
        chunk_count: container.chunks.len(),
        has_resource_definitions,
        has_input_signature,
        has_output_signature,
        instruction_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `crates/directx-shader-translate/shaders/vector_add.hlsl`を
    /// `fxc.exe /T cs_5_0 /E main`で実際にコンパイルして得た、本物の
    /// DXBCバイト列(D3D11 Compute Shader、SM5.0)。手書きのダミーバイト列
    /// ではない——このマシンの`C:\Program Files (x86)\Windows Kits\10\bin\
    /// 10.0.22621.0\x64\fxc.exe`で生成した実物。
    const VECTOR_ADD_DXBC: &[u8] = include_bytes!("../shaders/vector_add.dxbc");

    #[test]
    fn parses_real_fxc_compiled_vector_add_dxbc_container() {
        let module = parse_dxbc(VECTOR_ADD_DXBC).expect("real fxc-compiled DXBC must parse");
        assert!(module.chunk_count >= 4, "expected at least RDEF/ISGN/OSGN/SHEX chunks, got {}", module.chunk_count);
        assert!(module.has_resource_definitions, "vector_add.hlsl declares 3 UAV resources, RDEF must be present");
        assert!(module.has_output_signature, "compute shader OSGN chunk expected (may be empty-signature but present)");
        assert!(
            module.instruction_count.unwrap_or(0) > 0,
            "SHEX chunk must decode to a non-empty instruction stream"
        );
    }

    #[test]
    fn rejects_garbage_bytes_that_are_not_a_dxbc_container() {
        let garbage = [0u8; 16];
        assert!(parse_dxbc(&garbage).is_err(), "non-DXBC bytes must not parse successfully");
    }

    #[test]
    fn rejects_truncated_dxbc_header() {
        let truncated = &VECTOR_ADD_DXBC[..8];
        assert!(parse_dxbc(truncated).is_err(), "truncated container must fail to parse, not panic silently");
    }
}
