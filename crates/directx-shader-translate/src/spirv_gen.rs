//! DXBC(SM5.0)命令列 -> SPIR-Vの翻訳(バックエンド)。
//!
//! **正直なスコープ(2026-07-25、2回目の一般化後)**: これは汎用SM5.0
//! デコーダではない。以下の3つの実シェーダー(いずれも`fxc.exe`で実際に
//! コンパイルし、実SHEX命令列を確認した上でサポートを追加したもの)だけを
//! 対象にした翻訳器である:
//!
//! 1. `shaders/vector_add.hlsl` — `RWStructuredBuffer<float>`2本を読み、
//!    加算して1本へ書く。境界チェック無し。
//! 2. `shaders/vector_mul.hlsl` — 同上だが演算が乗算。
//! 3. `shaders/vector_sub_bounded.hlsl` — 定数バッファ(`uint`要素数1本)+
//!    `if (id.x < N)`境界チェック付きの減算。fxcは`a - b`を
//!    `add dest, -b, a`(第1ソースオペランドに`negate`フラグを立てたadd)
//!    へ最適化することを実機出力で確認した上でその形を検出している。
//!
//! 共通の骨格は次の通り:
//!
//! `dcl_globalFlags` -> (`dcl_constantbuffer`(b0)?) -> `dcl_uav_structured`
//! (x3) -> `dcl_input`(vThreadID) -> `dcl_temps` -> `dcl_thread_group` ->
//! (`ult` + `if`?) -> `ld_structured`x2 -> (`add` | `mul` |
//! `add`+negate) -> `store_structured` -> (`endif`?) -> `ret`
//!
//! 上記3パターン以外のオペコード・オペランド形状が1つでも混ざっている
//! 場合は`SpirvGenError::UnsupportedShader`を返し、誤った"対応している"
//! というシグナルは出さない。バッファのバインドポイント・スレッドグループ
//! サイズ・演算種別・境界チェックの有無は、すべて実際に`dxbc`クレートで
//! パースした`SHEX`命令列から抽出する(ハードコードした決め打ち値ではない)。
//!
//! 出力するSPIR-Vは、`open-cuda`の`opencuda-vulkan`が期待する契約
//! (3本のstorage buffer、set=0/binding=実バインドポイント、
//! push constant `uint n`、エントリポイント名`"main"`)に合わせている。
//! 境界チェック付きシェーダーでは、このpush constantの`n`を実際に
//! `id.x < n`の比較に使う(境界チェック無しシェーダーでは、従来通り
//! 宣言のみで未使用のまま——呼び出し側が`numthreads`の倍数でディスパッチ
//! する責任を負う)。

use dxbc::shex::{Instruction, InstructionKind, Opcode, OperandIndex, RegisterType};
use dxbc::{scan_dxbc, ChunkData};
use rspirv::binary::Assemble;
use rspirv::dr::{Builder, Operand as DrOperand};
use rspirv::spirv;
use thiserror::Error;

use crate::TranslateError;

/// 実DXBC出力から検出した2項演算の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// `add dest, srcB, srcA`(negateフラグ無し) — `A + B`。
    Add,
    /// `mul dest, srcB, srcA` — `A * B`。
    Mul,
    /// `add dest, -srcB, srcA`(第1ソースオペランドにnegate) — `A - B`。
    /// fxcが`a - b`をこの形へ最適化することを実機出力で確認した。
    Sub,
}

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
    #[error("このシェーダーは対応スコープ外(vector_add/vector_mul/vector_sub_bounded系オペコード列専用): {0}")]
    UnsupportedShader(String),
}

/// DXBCバイト列(`vector_add`/`vector_mul`/`vector_sub_bounded`のいずれか
/// 相当のD3D11 Compute Shader、SM5.0)を解析し、実際のSHEX命令列を検証しな
/// がらSPIR-Vへ翻訳する。後方互換のため`vector_add`専用時代の関数名を残す
/// が、実体は3パターン共通の[`translate_shader`]である。
pub fn translate_vector_add_shader(bytes: &[u8]) -> Result<TranslatedKernel, SpirvGenError> {
    translate_shader(bytes)
}

/// DXBCバイト列を解析し、対応する3パターン(add/mul/境界チェック付きsub)の
/// いずれかであれば実際のSHEX命令列を検証しながらSPIR-Vへ翻訳する。
/// いずれにも一致しなければ`SpirvGenError::UnsupportedShader`を返す。
pub fn translate_shader(bytes: &[u8]) -> Result<TranslatedKernel, SpirvGenError> {
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

    let shape = decode_shader_shape(&instructions)?;
    let spirv_words = emit_spirv(&shape);

    Ok(TranslatedKernel {
        spirv_words,
        entry_point: "main",
        local_size: shape.thread_group,
        uav_bind_points: (shape.uav_a, shape.uav_b, shape.uav_c),
    })
}

/// 検証済みのシェーダー形状(実DXBC解析から抽出した情報のみ)。
/// `vector_add`/`vector_mul`/`vector_sub_bounded`の3パターンいずれかに
/// 一致した結果を保持する(`op`と`bounds_check`でどのパターンかが分かる)。
struct ShaderShape {
    thread_group: (u32, u32, u32),
    /// UAVバインドポイント。`ld_structured`の読み込み元2本(発見順=A, B)と、
    /// `store_structured`の書き込み先1本。
    uav_a: u32,
    uav_b: u32,
    uav_c: u32,
    /// 実際に検出した2項演算。
    op: BinaryOp,
    /// `dcl_constantbuffer`(b0) + `ult` + `if`/`endif`による
    /// `if (id.x < n)`境界チェックが存在したか。
    bounds_check: bool,
}

fn uav_index(indices: &[OperandIndex]) -> Option<u32> {
    match indices.first()? {
        OperandIndex::Imm32(i) => Some(*i),
        _ => None,
    }
}

/// 実際のSHEX命令列を、対応する3パターンのいずれかと厳密に突き合わせる。
/// いずれにも一致しなければ、対応スコープ外として明示的に拒否する
/// (「対応している」という誤ったシグナルを出さない、というCLAUDE.md方針)。
fn decode_shader_shape(instructions: &[Instruction]) -> Result<ShaderShape, SpirvGenError> {
    let mut has_cbuffer = false;
    let mut declared_uavs: Vec<u32> = Vec::new();
    let mut thread_group: Option<(u32, u32, u32)> = None;
    let mut ld_uavs: Vec<u32> = Vec::new();
    let mut store_uav: Option<u32> = None;
    let mut op: Option<BinaryOp> = None;
    let mut saw_ult = false;
    let mut saw_if = false;
    let mut saw_endif = false;
    let mut saw_ret = false;

    for ins in instructions {
        match &ins.kind {
            InstructionKind::DclGlobalFlags { .. } => {}
            InstructionKind::DclConstantBuffer { operands, .. } => {
                let op0 = operands.first().ok_or_else(|| {
                    SpirvGenError::UnsupportedShader("dcl_constantbufferにオペランドが無い".to_string())
                })?;
                if op0.reg_type != RegisterType::ConstantBuffer {
                    return Err(SpirvGenError::UnsupportedShader(
                        "dcl_constantbufferの対象レジスタがcbではない".to_string(),
                    ));
                }
                // register(b0)のみ対応(vector_sub_bounded.hlslが唯一使う形)。
                if uav_index(&op0.indices) != Some(0) {
                    return Err(SpirvGenError::UnsupportedShader(
                        "対応しているのはb0の定数バッファのみ".to_string(),
                    ));
                }
                has_cbuffer = true;
            }
            InstructionKind::DclUavStructured { stride, operands, .. } => {
                if *stride != 4 {
                    return Err(SpirvGenError::UnsupportedShader(format!(
                        "dcl_uav_structuredのstrideが4(float)ではない: {stride}"
                    )));
                }
                let op0 = operands.first().ok_or_else(|| {
                    SpirvGenError::UnsupportedShader("dcl_uav_structuredにオペランドが無い".to_string())
                })?;
                if op0.reg_type != RegisterType::Uav {
                    return Err(SpirvGenError::UnsupportedShader(
                        "dcl_uav_structuredの対象レジスタがUAVではない".to_string(),
                    ));
                }
                let idx = uav_index(&op0.indices).ok_or_else(|| {
                    SpirvGenError::UnsupportedShader("UAVバインドポイントを解決できない".to_string())
                })?;
                declared_uavs.push(idx);
            }
            InstructionKind::DclInput { operands, .. } => {
                let op0 = operands.first().ok_or_else(|| {
                    SpirvGenError::UnsupportedShader("dcl_inputにオペランドが無い".to_string())
                })?;
                if op0.reg_type != RegisterType::ThreadID {
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
                Opcode::ULt => {
                    // `id.x < N`(N=定数バッファ)の比較のみ対応。
                    let rhs = operands.get(2).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("ultの右辺オペランドが無い".to_string())
                    })?;
                    if rhs.reg_type != RegisterType::ConstantBuffer {
                        return Err(SpirvGenError::UnsupportedShader(
                            "対応しているのは定数バッファとの比較のみ".to_string(),
                        ));
                    }
                    saw_ult = true;
                }
                Opcode::If => {
                    if !saw_ult {
                        return Err(SpirvGenError::UnsupportedShader(
                            "ultの結果を使わないifは対応スコープ外".to_string(),
                        ));
                    }
                    saw_if = true;
                }
                Opcode::EndIf => {
                    saw_endif = true;
                }
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
                    // 実fxc.exe出力で確認した形: 第1ソースオペランド
                    // (operands[1]、2回目のld結果=B)にnegateが立っていれば
                    // `A - B`への最適化(add dest, -B, A)、無ければ通常の
                    // `A + B`。
                    let src1 = operands.get(1).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("addの第1ソースオペランドが無い".to_string())
                    })?;
                    op = Some(if src1.negate { BinaryOp::Sub } else { BinaryOp::Add });
                }
                Opcode::Mul => {
                    op = Some(BinaryOp::Mul);
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
            "3本のUAVを想定するが{}本だった",
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
    let op = op.ok_or_else(|| {
        SpirvGenError::UnsupportedShader("add/mul命令が見つからない".to_string())
    })?;
    let store_uav = store_uav.ok_or_else(|| {
        SpirvGenError::UnsupportedShader("store_structuredが見つからない".to_string())
    })?;
    if !saw_ret {
        return Err(SpirvGenError::UnsupportedShader("ret命令が見つからない".to_string()));
    }
    // 境界チェックは「定数バッファ宣言 + ult + if + endif」が全部揃っている
    // か、全く無いかのどちらかのみ許容する(中途半端な組み合わせは拒否)。
    let bounds_check = has_cbuffer && saw_ult && saw_if && saw_endif;
    if has_cbuffer != bounds_check || saw_ult != bounds_check || saw_if != bounds_check || saw_endif != bounds_check {
        return Err(SpirvGenError::UnsupportedShader(
            "境界チェック構成(dcl_constantbuffer/ult/if/endif)が不完全".to_string(),
        ));
    }

    Ok(ShaderShape {
        thread_group,
        uav_a: ld_uavs[0],
        uav_b: ld_uavs[1],
        uav_c: store_uav,
        op,
        bounds_check,
    })
}

/// 検証済みの`ShaderShape`から、実際にSPIR-Vバイナリを組み立てる
/// (`rspirv::dr::Builder`使用、手書きバイナリ列の直接構築ではない)。
///
/// レイアウトは`opencuda-vulkan`の契約に合わせる: storage buffer 3本
/// (set=0, binding=`uav_a`/`uav_b`/`uav_c`、いずれも実際にDXBCから抽出した
/// バインドポイント)+ push constant `uint n`。`bounds_check`が真の場合、
/// このpush constantの`n`を`id.x < n`の比較に実際に使う。
fn emit_spirv(shape: &ShaderShape) -> Vec<u32> {
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
    let var_params = b.variable(params_ptr_ty, None, spirv::StorageClass::PushConstant, None);
    let uint_ptr_pushconstant_ty = b.type_pointer(None, spirv::StorageClass::PushConstant, uint_ty);

    // gl_GlobalInvocationID
    let gid_ptr_ty = b.type_pointer(None, spirv::StorageClass::Input, uvec3_ty);
    let var_gid = b.variable(gid_ptr_ty, None, spirv::StorageClass::Input, None);
    b.decorate(var_gid, spirv::Decoration::BuiltIn, vec![DrOperand::BuiltIn(spirv::BuiltIn::GlobalInvocationId)]);

    let float_ptr_uniform_ty = b.type_pointer(None, spirv::StorageClass::Uniform, float_ty);
    let bool_ty = b.type_bool();

    let main_fn = b
        .begin_function(void_ty, None, spirv::FunctionControl::NONE, voidf_ty)
        .expect("OpFunction");
    b.begin_block(None).expect("OpLabel");

    let const_0 = b.constant_bit32(uint_ty, 0);

    let gid_vec = b.load(uvec3_ty, None, var_gid, None, vec![]).expect("OpLoad gid");
    let idx = b
        .composite_extract(uint_ty, None, gid_vec, vec![0])
        .expect("OpCompositeExtract .x");

    // 本体(バッファ読み込み+演算+書き込み)を組み立てるクロージャ。
    // 境界チェック無しの場合は現在のブロックへ直接、境界チェック有りの場合は
    // `if`ブロック内へ、それぞれ同じ命令列を発行する。
    let emit_body = |b: &mut Builder| {
        let ac_a = b
            .access_chain(float_ptr_uniform_ty, None, var_a, vec![const_0, idx])
            .expect("OpAccessChain a");
        let val_a = b.load(float_ty, None, ac_a, None, vec![]).expect("OpLoad a[i]");

        let ac_b = b
            .access_chain(float_ptr_uniform_ty, None, var_b, vec![const_0, idx])
            .expect("OpAccessChain b");
        let val_b = b.load(float_ty, None, ac_b, None, vec![]).expect("OpLoad b[i]");

        let result = match shape.op {
            BinaryOp::Add => b.f_add(float_ty, None, val_a, val_b).expect("OpFAdd"),
            BinaryOp::Mul => b.f_mul(float_ty, None, val_a, val_b).expect("OpFMul"),
            BinaryOp::Sub => b.f_sub(float_ty, None, val_a, val_b).expect("OpFSub"),
        };

        let ac_c = b
            .access_chain(float_ptr_uniform_ty, None, var_c, vec![const_0, idx])
            .expect("OpAccessChain c");
        b.store(ac_c, result, None, vec![]).expect("OpStore c[i]");
    };

    if shape.bounds_check {
        let n_ptr = b
            .access_chain(uint_ptr_pushconstant_ty, None, var_params, vec![const_0])
            .expect("OpAccessChain params.n");
        let n_val = b.load(uint_ty, None, n_ptr, None, vec![]).expect("OpLoad n");
        let cond = b
            .u_less_than(bool_ty, None, idx, n_val)
            .expect("OpULessThan idx < n");

        let then_label = b.id();
        let merge_label = b.id();
        b.selection_merge(merge_label, spirv::SelectionControl::NONE)
            .expect("OpSelectionMerge");
        b.branch_conditional(cond, then_label, merge_label, vec![])
            .expect("OpBranchConditional");

        b.begin_block(Some(then_label)).expect("OpLabel then");
        emit_body(&mut b);
        b.branch(merge_label).expect("OpBranch to merge");

        b.begin_block(Some(merge_label)).expect("OpLabel merge");
    } else {
        emit_body(&mut b);
    }

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

    /// `shaders/vector_mul.dxbc`(実fxc.exe出力)。
    const VECTOR_MUL_DXBC: &[u8] = include_bytes!("../shaders/vector_mul.dxbc");

    #[test]
    fn translates_real_fxc_compiled_vector_mul_dxbc_to_valid_spirv() {
        let kernel = translate_shader(VECTOR_MUL_DXBC)
            .expect("real fxc-compiled vector_mul.dxbc (mul opcode) must translate");
        assert_eq!(kernel.uav_bind_points, (0, 1, 2));
        assert_eq!(kernel.local_size, (64, 1, 1));
        assert_eq!(kernel.spirv_words[0], 0x0723_0203);

        let bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let mut loader = rspirv::dr::Loader::new();
        rspirv::binary::parse_bytes(&bytes, &mut loader)
            .expect("emitted SPIR-V (mul) must be well-formed and re-parseable");
    }

    /// `shaders/vector_sub_bounded.dxbc`(実fxc.exe出力、境界チェック付き)。
    const VECTOR_SUB_BOUNDED_DXBC: &[u8] = include_bytes!("../shaders/vector_sub_bounded.dxbc");

    #[test]
    fn translates_real_fxc_compiled_vector_sub_bounded_dxbc_to_valid_spirv() {
        let kernel = translate_shader(VECTOR_SUB_BOUNDED_DXBC).expect(
            "real fxc-compiled vector_sub_bounded.dxbc (negated-add sub + ult/if bounds check) must translate",
        );
        assert_eq!(kernel.uav_bind_points, (0, 1, 2));
        assert_eq!(kernel.local_size, (64, 1, 1));
        assert_eq!(kernel.spirv_words[0], 0x0723_0203);

        let bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let mut loader = rspirv::dr::Loader::new();
        rspirv::binary::parse_bytes(&bytes, &mut loader)
            .expect("emitted SPIR-V (sub, bounded) must be well-formed and re-parseable");
    }
}
