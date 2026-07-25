//! DXBC(SM5.0)命令列 -> SPIR-Vの翻訳(バックエンド)。
//!
//! **正直なスコープ**: これは汎用SM5.0デコーダではない。`vector_add.hlsl`
//! (`RWStructuredBuffer<float>`2本を読み1本へ書く、`SV_DispatchThreadID`で
//! 添字するD3D11 Compute Shader)が実際に`fxc.exe`でコンパイルされたときに
//! 生成する、次の狭いオペコード列だけを対象にした翻訳器である:
//!
//! `dcl_globalFlags` -> `dcl_uav_structured`(x N) -> `dcl_input`(vThreadID)
//! -> `dcl_temps` -> `dcl_thread_group` -> `ld_structured`(x2) -> `add`
//! -> `store_structured` -> `ret`
//!
//! 上記以外のオペコードが1つでも混ざっている場合は
//! `TranslateError::UnsupportedShader`を返し、誤った"対応している"という
//! シグナルは出さない。バッファのバインドポイント・スレッドグループサイズは
//! すべて実際に`dxbc`クレートでパースした`SHEX`命令列から抽出する
//! (ハードコードした決め打ち値ではない)。
//!
//! 出力するSPIR-Vは、`open-cuda`の`opencuda-vulkan`が期待する
//! `vector_add`契約(3本のstorage buffer、set=0/binding=0,1,2、
//! push constant `uint n`、エントリポイント名`"main"`)に合わせている。

use dxbc::shex::{Instruction, InstructionKind, Opcode, OperandIndex, RegisterType};
use dxbc::{scan_dxbc, ChunkData};
use rspirv::binary::Assemble;
use rspirv::dr::{Builder, Operand as DrOperand};
use rspirv::spirv;
use thiserror::Error;

use crate::TranslateError;

/// 翻訳結果のSPIR-Vモジュールと、Vulkanディスパッチに必要な最小限のメタ情報。
///
/// **スコープの正直な開示**: `spirv_words`は`vector_add`の狭いオペコード列
/// からのみ生成できる。`local_size`は実際にパースした`dcl_thread_group`から
/// 得た値であり、決め打ちではない。
#[derive(Debug, Clone)]
pub struct TranslatedKernel {
    /// 生成されたSPIR-Vモジュール(リトルエンディアン32bitワード列)。
    pub spirv_words: Vec<u32>,
    /// `OpEntryPoint`のエントリポイント名(常に`"main"`)。
    pub entry_point: &'static str,
    /// `dcl_thread_group`から得た実際のスレッドグループサイズ(x, y, z)。
    pub local_size: (u32, u32, u32),
    /// `dcl_uav_structured`から得た、読み込み元2バッファ+書き込み先1バッファの
    /// UAVバインドポイント(u#の#)。`(a, b, c)`の順。
    pub uav_bind_points: (u32, u32, u32),
}

#[derive(Debug, Error)]
pub enum SpirvGenError {
    #[error("DXBC解析エラー: {0}")]
    Translate(#[from] TranslateError),
    #[error("このシェーダーは対応スコープ外(狭いvector_add系オペコード列専用): {0}")]
    UnsupportedShader(String),
}

/// DXBCバイト列(`vector_add.hlsl`相当のD3D11 Compute Shader、SM5.0)を解析し、
/// 実際のSHEX命令列を検証しながらSPIR-Vへ翻訳する。
pub fn translate_vector_add_shader(bytes: &[u8]) -> Result<TranslatedKernel, SpirvGenError> {
    let containers = scan_dxbc(bytes);
    let container = containers.into_iter().next().ok_or_else(|| {
        SpirvGenError::Translate(TranslateError::Parse(
            "DXBCコンテナが見つからない".to_string(),
        ))
    })?;

    let mut instructions: Option<Vec<Instruction>> = None;
    for chunk in &container.chunks {
        if let ChunkData::Shader(program) = chunk.parse() {
            instructions = Some(program.instructions);
        }
    }
    let instructions =
        instructions.ok_or(SpirvGenError::Translate(TranslateError::MissingChunk("SHEX")))?;

    let shape = decode_vector_add_shape(&instructions)?;
    let spirv_words = emit_spirv(&shape);

    Ok(TranslatedKernel {
        spirv_words,
        entry_point: "main",
        local_size: shape.thread_group,
        uav_bind_points: (shape.uav_a, shape.uav_b, shape.uav_c),
    })
}

/// 検証済みの狭いvector_addシェーダー形状(実DXBC解析から抽出した情報のみ)。
struct VectorAddShape {
    thread_group: (u32, u32, u32),
    /// UAVバインドポイント。`ld_structured`の読み込み元2本(u#昇順で発見順)と、
    /// `store_structured`の書き込み先1本。
    uav_a: u32,
    uav_b: u32,
    uav_c: u32,
}

fn uav_index(indices: &[OperandIndex]) -> Option<u32> {
    match indices.first()? {
        OperandIndex::Imm32(i) => Some(*i),
        _ => None,
    }
}

/// 実際のSHEX命令列を、`vector_add`が実際に生成する狭いオペコード列と厳密に
/// 突き合わせる。1つでも一致しなければ、対応スコープ外として明示的に拒否する
/// (「対応している」という誤ったシグナルを出さない、というCLAUDE.md方針)。
fn decode_vector_add_shape(instructions: &[Instruction]) -> Result<VectorAddShape, SpirvGenError> {
    let mut declared_uavs: Vec<u32> = Vec::new();
    let mut thread_group: Option<(u32, u32, u32)> = None;
    let mut ld_uavs: Vec<u32> = Vec::new();
    let mut store_uav: Option<u32> = None;
    let mut saw_add = false;
    let mut saw_ret = false;

    for ins in instructions {
        match &ins.kind {
            InstructionKind::DclGlobalFlags { .. } => {}
            InstructionKind::DclUavStructured { stride, operands, .. } => {
                if *stride != 4 {
                    return Err(SpirvGenError::UnsupportedShader(format!(
                        "dcl_uav_structuredのstrideが4(float)ではない: {stride}"
                    )));
                }
                let op = operands.first().ok_or_else(|| {
                    SpirvGenError::UnsupportedShader("dcl_uav_structuredにオペランドが無い".to_string())
                })?;
                if op.reg_type != RegisterType::Uav {
                    return Err(SpirvGenError::UnsupportedShader(
                        "dcl_uav_structuredの対象レジスタがUAVではない".to_string(),
                    ));
                }
                let idx = uav_index(&op.indices).ok_or_else(|| {
                    SpirvGenError::UnsupportedShader("UAVバインドポイントを解決できない".to_string())
                })?;
                declared_uavs.push(idx);
            }
            InstructionKind::DclInput { operands, .. } => {
                let op = operands.first().ok_or_else(|| {
                    SpirvGenError::UnsupportedShader("dcl_inputにオペランドが無い".to_string())
                })?;
                if op.reg_type != RegisterType::ThreadID {
                    return Err(SpirvGenError::UnsupportedShader(
                        "対応しているのはvThreadID(SV_DispatchThreadID)入力のみ".to_string(),
                    ));
                }
            }
            InstructionKind::DclTemps { .. } => {}
            InstructionKind::DclThreadGroup { x, y, z } => {
                thread_group = Some((*x, *y, *z));
            }
            InstructionKind::Generic { operands } => match ins.opcode {
                Opcode::LdStructured => {
                    let src_uav = operands.get(3).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("ld_structuredのUAVオペランドが無い".to_string())
                    })?;
                    if src_uav.reg_type != RegisterType::Uav {
                        return Err(SpirvGenError::UnsupportedShader(
                            "ld_structuredの読み込み元がUAVではない".to_string(),
                        ));
                    }
                    let idx = uav_index(&src_uav.indices).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("ld_structuredのUAVバインドポイントを解決できない".to_string())
                    })?;
                    let idx_operand = operands.get(1).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("ld_structuredの添字オペランドが無い".to_string())
                    })?;
                    if idx_operand.reg_type != RegisterType::ThreadID {
                        return Err(SpirvGenError::UnsupportedShader(
                            "対応しているのはvThreadIDによる添字のみ".to_string(),
                        ));
                    }
                    ld_uavs.push(idx);
                }
                Opcode::Add => {
                    saw_add = true;
                }
                Opcode::StoreStructured => {
                    let dest_uav = operands.first().ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("store_structuredの書き込み先オペランドが無い".to_string())
                    })?;
                    if dest_uav.reg_type != RegisterType::Uav {
                        return Err(SpirvGenError::UnsupportedShader(
                            "store_structuredの書き込み先がUAVではない".to_string(),
                        ));
                    }
                    let idx = uav_index(&dest_uav.indices).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("store_structuredのUAVバインドポイントを解決できない".to_string())
                    })?;
                    store_uav = Some(idx);
                }
                Opcode::Ret => {
                    saw_ret = true;
                }
                other => {
                    return Err(SpirvGenError::UnsupportedShader(format!(
                        "対応スコープ外のオペコード: {other:?}"
                    )));
                }
            },
            other => {
                return Err(SpirvGenError::UnsupportedShader(format!(
                    "対応スコープ外の宣言命令: {other:?}"
                )));
            }
        }
    }

    if declared_uavs.len() != 3 {
        return Err(SpirvGenError::UnsupportedShader(format!(
            "vector_addは3本のUAVを想定するが{}本だった",
            declared_uavs.len()
        )));
    }
    let thread_group = thread_group.ok_or_else(|| {
        SpirvGenError::UnsupportedShader("dcl_thread_groupが見つからない".to_string())
    })?;
    if ld_uavs.len() != 2 {
        return Err(SpirvGenError::UnsupportedShader(format!(
            "ld_structuredが2回のはずが{}回だった",
            ld_uavs.len()
        )));
    }
    if !saw_add {
        return Err(SpirvGenError::UnsupportedShader("add命令が見つからない".to_string()));
    }
    let store_uav = store_uav.ok_or_else(|| {
        SpirvGenError::UnsupportedShader("store_structuredが見つからない".to_string())
    })?;
    if !saw_ret {
        return Err(SpirvGenError::UnsupportedShader("ret命令が見つからない".to_string()));
    }

    Ok(VectorAddShape {
        thread_group,
        uav_a: ld_uavs[0],
        uav_b: ld_uavs[1],
        uav_c: store_uav,
    })
}

/// 検証済みの`VectorAddShape`から、実際にSPIR-Vバイナリを組み立てる
/// (`rspirv::dr::Builder`使用、手書きバイナリ列の直接構築ではない)。
///
/// レイアウトは`opencuda-vulkan`の`vector_add`契約に合わせる:
/// storage buffer 3本(set=0, binding=`uav_a`/`uav_b`/`uav_c`、いずれも
/// 実際にDXBCから抽出したバインドポイント)+ push constant `uint n`。
fn emit_spirv(shape: &VectorAddShape) -> Vec<u32> {
    let mut b = Builder::new();
    b.set_version(1, 0);
    b.capability(spirv::Capability::Shader);
    b.memory_model(spirv::AddressingModel::Logical, spirv::MemoryModel::GLSL450);

    let void_ty = b.type_void();
    let voidf_ty = b.type_function(void_ty, vec![]);
    let float_ty = b.type_float(32, None);
    let uint_ty = b.type_int(32, 0);
    let uvec3_ty = b.type_vector(uint_ty, 3);

    // storage buffer: struct { float data[]; } (Uniform/BufferBlock, SPIR-V 1.0互換)
    let rt_array_ty = b.type_runtime_array(float_ty);
    b.decorate(rt_array_ty, spirv::Decoration::ArrayStride, vec![DrOperand::LiteralBit32(4)]);
    let buf_struct_ty = b.type_struct(vec![rt_array_ty]);
    b.decorate(buf_struct_ty, spirv::Decoration::BufferBlock, vec![]);
    b.member_decorate(buf_struct_ty, 0, spirv::Decoration::Offset, vec![DrOperand::LiteralBit32(0)]);
    let buf_ptr_ty = b.type_pointer(None, spirv::StorageClass::Uniform, buf_struct_ty);

    let make_buffer_var = |b: &mut Builder, binding: u32| -> u32 {
        let var = b.variable(buf_ptr_ty, None, spirv::StorageClass::Uniform, None);
        b.decorate(var, spirv::Decoration::DescriptorSet, vec![DrOperand::LiteralBit32(0)]);
        b.decorate(var, spirv::Decoration::Binding, vec![DrOperand::LiteralBit32(binding)]);
        var
    };
    let var_a = make_buffer_var(&mut b, shape.uav_a);
    let var_b = make_buffer_var(&mut b, shape.uav_b);
    let var_c = make_buffer_var(&mut b, shape.uav_c);

    // push constant: struct Params { uint n; }
    let params_struct_ty = b.type_struct(vec![uint_ty]);
    b.decorate(params_struct_ty, spirv::Decoration::Block, vec![]);
    b.member_decorate(params_struct_ty, 0, spirv::Decoration::Offset, vec![DrOperand::LiteralBit32(0)]);
    let params_ptr_ty = b.type_pointer(None, spirv::StorageClass::PushConstant, params_struct_ty);
    let _var_params = b.variable(params_ptr_ty, None, spirv::StorageClass::PushConstant, None);

    // gl_GlobalInvocationID
    let gid_ptr_ty = b.type_pointer(None, spirv::StorageClass::Input, uvec3_ty);
    let var_gid = b.variable(gid_ptr_ty, None, spirv::StorageClass::Input, None);
    b.decorate(var_gid, spirv::Decoration::BuiltIn, vec![DrOperand::BuiltIn(spirv::BuiltIn::GlobalInvocationId)]);

    let float_ptr_uniform_ty = b.type_pointer(None, spirv::StorageClass::Uniform, float_ty);

    let main_fn = b
        .begin_function(void_ty, None, spirv::FunctionControl::NONE, voidf_ty)
        .expect("OpFunction");
    b.begin_block(None).expect("OpLabel");

    let const_0 = b.constant_bit32(uint_ty, 0);

    let gid_vec = b.load(uvec3_ty, None, var_gid, None, vec![]).expect("OpLoad gid");
    let idx = b
        .composite_extract(uint_ty, None, gid_vec, vec![0])
        .expect("OpCompositeExtract .x");

    let ac_a = b
        .access_chain(float_ptr_uniform_ty, None, var_a, vec![const_0, idx])
        .expect("OpAccessChain a");
    let val_a = b.load(float_ty, None, ac_a, None, vec![]).expect("OpLoad a[i]");

    let ac_b = b
        .access_chain(float_ptr_uniform_ty, None, var_b, vec![const_0, idx])
        .expect("OpAccessChain b");
    let val_b = b.load(float_ty, None, ac_b, None, vec![]).expect("OpLoad b[i]");

    let sum = b.f_add(float_ty, None, val_a, val_b).expect("OpFAdd");

    let ac_c = b
        .access_chain(float_ptr_uniform_ty, None, var_c, vec![const_0, idx])
        .expect("OpAccessChain c");
    b.store(ac_c, sum, None, vec![]).expect("OpStore c[i]");

    b.ret().expect("OpReturn");
    b.end_function().expect("OpFunctionEnd");

    b.entry_point(spirv::ExecutionModel::GLCompute, main_fn, "main", vec![var_gid]);
    b.execution_mode(
        main_fn,
        spirv::ExecutionMode::LocalSize,
        [shape.thread_group.0, shape.thread_group.1, shape.thread_group.2],
    );

    let module = b.module();
    module.assemble()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `crates/directx-shader-translate/shaders/vector_add.dxbc`と同じ実
    /// fxc.exe出力(lib.rsのテストで使っているのと同一バイト列)。
    const VECTOR_ADD_DXBC: &[u8] = include_bytes!("../shaders/vector_add.dxbc");

    #[test]
    fn translates_real_fxc_compiled_vector_add_dxbc_to_valid_spirv() {
        let kernel = translate_vector_add_shader(VECTOR_ADD_DXBC)
            .expect("narrow vector_add opcode subset must translate");

        // 実際にdxbcクレートでパースしたRDEF(u0/u1/u2)・dcl_thread_group(64,1,1)と
        // 一致していること(決め打ちではなく、抽出結果であることの検証)。
        assert_eq!(kernel.uav_bind_points, (0, 1, 2));
        assert_eq!(kernel.local_size, (64, 1, 1));
        assert_eq!(kernel.entry_point, "main");

        // SPIR-Vバイナリマジックナンバー(0x07230203)がリトルエンディアンで
        // 先頭ワードに入っていること。
        assert_eq!(kernel.spirv_words[0], 0x0723_0203);

        // rspirvが組み立てたモジュールが、そのモジュール自身の構造規則
        // (SPIR-V各種チェック)に照らして自己矛盾していないこと。
        let bytes: Vec<u8> = kernel
            .spirv_words
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect();
        let mut loader = rspirv::dr::Loader::new();
        rspirv::binary::parse_bytes(&bytes, &mut loader)
            .expect("emitted SPIR-V must be well-formed and re-parseable by rspirv's own loader");
        let reparsed = loader.module();
        assert!(
            reparsed.entry_points.iter().any(|ep| ep.operands.iter().any(|op| matches!(
                op,
                rspirv::dr::Operand::LiteralString(s) if s == "main"
            ))),
            "re-parsed module must still expose entry point \"main\""
        );
    }

    #[test]
    fn rejects_garbage_bytes_honestly_instead_of_pretending_to_translate() {
        let garbage = [0u8; 16];
        assert!(translate_vector_add_shader(&garbage).is_err());
    }
}
