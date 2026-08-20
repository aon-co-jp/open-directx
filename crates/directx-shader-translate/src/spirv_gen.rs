//! DXBC(SM5.0)命令列 -> SPIR-Vの翻訳(バックエンド)。
//!
//! **正直なスコープ(2026-07-25、3回目の一般化後)**: これは汎用SM5.0
//! デコーダではない。以下の4つの実シェーダー(いずれも`fxc.exe`で実際に
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
//! 4. `shaders/vector_div.hlsl` — 演算が除算(`Opcode::Div`)。実SHEX命令列を
//!    ダンプして確認したところ、add/mulと全く同じ命令形状
//!    (`ld_structured`x2 -> 演算 -> `store_structured`)で、オペコードだけが
//!    異なる形だった(fxcは除算専用の特別な最適化をしていない)。
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

use dxbc::shex::{ComponentSelect, Instruction, InstructionKind, Opcode, Operand, OperandIndex, RegisterType};
use dxbc::{scan_dxbc, ChunkData};
use rspirv::binary::Assemble;
use rspirv::dr::{Builder, Operand as DrOperand};
use rspirv::spirv;
use std::collections::HashMap;
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
    /// `div dest, srcA, srcB`(`Opcode::Div`) — `A / B`。add/mulと全く同じ
    /// 命令形状(`ld_structured`x2->演算->`store_structured`)で、オペコード
    /// だけがDivに変わる形を実機出力(`vector_div.dxbc`)で確認した。
    Div,
    /// `mul dest, -srcB, srcA`(いずれか片方のソースオペランドにnegate) —
    /// `A * (-B)` = `-(A*B)`。2026-08-08、`vector_mul_negate.hlsl`
    /// (`Output[i] = A[i] * (-B[i])`)の実fxc.exe出力で、`mul`命令の第1
    /// ソースオペランドに`negate: true`が実際に立つことを確認した——以前の
    /// HANDOFFで「mulのnegateフラグは未検証」としていたケースがこれに
    /// 相当する。両方のソースがnegateされる場合は理論上打ち消し合って
    /// 通常の`Mul`と同じになるが、そのパターンは実シェーダーで確認して
    /// いないため、今回はどちらか片方のみがnegateされているケースに
    /// 限定して対応する(両方negateは`decode_shader_shape`側で拒否)。
    MulNeg,
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
    #[error("このシェーダーは対応スコープ外(vector_add/vector_mul/vector_div/vector_sub_bounded系オペコード列専用): {0}")]
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
                    // 実fxc.exe出力(`vector_mul_negate.dxbc`)で確認した形:
                    // ソースオペランドのどちらか片方に`negate`が立っていれば
                    // `A * (-B)`への最適化(`MulNeg`)、両方に立つ場合は
                    // (打ち消し合うはずだが実シェーダーで未確認のため)対応
                    // スコープ外として拒否する。
                    let src1 = operands.get(1).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("mulの第1ソースオペランドが無い".to_string())
                    })?;
                    let src2 = operands.get(2).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("mulの第2ソースオペランドが無い".to_string())
                    })?;
                    op = Some(match (src1.negate, src2.negate) {
                        (false, false) => BinaryOp::Mul,
                        (true, false) | (false, true) => BinaryOp::MulNeg,
                        (true, true) => {
                            return Err(SpirvGenError::UnsupportedShader(
                                "mulの両方のソースオペランドにnegateが立つケースは未検証のため対応スコープ外".to_string(),
                            ));
                        }
                    });
                }
                Opcode::Div => {
                    op = Some(BinaryOp::Div);
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
    emit_spirv_for_kernel(
        shape.thread_group,
        shape.uav_a,
        shape.uav_b,
        shape.uav_c,
        shape.op,
        shape.bounds_check,
    )
}

/// `emit_spirv`の中身そのもの(DXBC固有の`ShaderShape`型に依存しない、
/// 純粋なパラメータのみを受け取る形)。DXBC(`emit_spirv`経由)とDXIL
/// (`crate::dxil::translate_dxil_vector_add_to_spirv`)の両方の翻訳経路
/// から共有される、実際のSPIR-V組み立てロジック本体。
///
/// **正直な開示**: DXBCとDXILは命令セット・コンテナ形式が全く異なるが、
/// `vector_add`が要求する最終的なSPIR-Vの形(storage buffer 3本読み書き+
/// スレッドID添字)は同一であることを、DXBC側の`ShaderShape`抽出結果と
/// DXIL側の`resolve_vector_add_dxil_calls`解決結果を突き合わせて確認した
/// 上で共通化している(命令セットの共通化ではなく、あくまで出力SPIR-Vの
/// 形の共通化)。
pub(crate) fn emit_spirv_for_kernel(
    thread_group: (u32, u32, u32),
    uav_a: u32,
    uav_b: u32,
    uav_c: u32,
    op: BinaryOp,
    bounds_check: bool,
) -> Vec<u32> {
    let shape = ShaderShape {
        thread_group,
        uav_a,
        uav_b,
        uav_c,
        op,
        bounds_check,
    };
    emit_spirv_impl(&shape)
}

fn emit_spirv_impl(shape: &ShaderShape) -> Vec<u32> {
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
            BinaryOp::Div => b.f_div(float_ty, None, val_a, val_b).expect("OpFDiv"),
            BinaryOp::MulNeg => {
                let product = b.f_mul(float_ty, None, val_a, val_b).expect("OpFMul");
                b.f_negate(float_ty, None, product).expect("OpFNegate")
            }
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

// ---------------------------------------------------------------------
// ここから先(「N個の逐次2項演算」パターンクラス)は今回新規に追加した部分。
// ---------------------------------------------------------------------
//
// 既存の`decode_shader_shape`(UAV3本固定・2項演算1回固定、`ShaderShape`)は
// 一切変更していない(既存4形状+境界チェック版の実Vulkanテストへの回帰リスクを
// ゼロにするため)。今回追加したのは、それとは別の、より一般化されたパターン
// クラス`decode_chain_shape`/`ChainShape`である。
//
// **一般化の軸**: `shaders/vector_add_mul_chain.hlsl`
// (`t = InputA[i] + InputB[i]; Output[i] = t * InputA[i];`、UAV3本・
// 2項演算2回)を実際に`fxc.exe /T cs_5_0`でコンパイルし、
// `examples/dump_shex.rs`で実SHEX命令列をダンプして確認したところ、
// **予想に反して`dcl_temps`は1個のまま**だった(`t`という1つの一時変数しか
// HLSL上には無いにもかかわらず、fxcは`InputA[i]`の2回目の参照を「ロード
// し直す」のではなく「最初の`ld_structured`の結果(`r0.y`)を単純に再利用する
// (共通部分式除去/CSE)」という最適化をしていた——2回目の`ld_structured`は
// 実バイト列に存在しない)。そのため一般化の軸は「N+1個の一時レジスタ」でも
// 「N回の`ld_structured`」でもなく、**「一時レジスタの各コンポーネントへ、
// バッファ読み込みまたは2項演算の結果を割り当てていく評価式の木
// (制御フロー無し)」**とした——`store_structured`が最終的に参照する
// コンポーネントから逆算して式木を実際に構築し、そこに含まれる
// `ld_structured`(読み込み元UAV)・2項演算(`add`/`mul`、既存3パターンと同じ
// 検出方式)の数も、同じ一時レジスタコンポーネントが何回再利用されるかも
// 問わない(1回でも2回でもN回でも、fxcがCSEで詰め込んでいても同じロジックで
// 扱える)。これは「シェーダー5個目の形をそのままハードコードする」のでは
// なく、マッチングロジック自体を「N個の逐次2項演算」というパターンクラスへ
// 一般化したものである。
//
// **正直な開示(UAV本数)**: 当初はUAV4本(A/B/C/Out)の式`(a+b)*c`を狙ったが、
// `opencuda-vulkan::VulkanDevice::launch_kernel`が`"vector_add"`/`"matmul"`
// のいずれも厳密に3バッファ固定の引数配線しか持たない(`real.rs`の
// `ensure_vector_add_args`/`ensure_matmul_args`実装で確認済み、
// `open-cuda`側は今回変更しない方針のため)ため、実Vulkan実機テストに乗せる
// 都合上、UAV3本(`InputA`を2回参照する`t = A+B; Out = t*A;`)へ設計を変更した。
// UAV本数自体を増やす一般化ではなく、「同一UAVの多重参照込みの2項演算チェーン」
// という軸に絞った——これも正直な開示の通り、当初想定より小さいが実物の
// 一般化である。
//
// **正直な開示(スコープ、2026-07-27更新)**: 対応する2項演算は
// `add`/`mul`/`sub`(negated-add最適化)/`div`。`vector_sub_div_chain.hlsl`
// (`t = InputA[i] - InputB[i]; Output[i] = t / InputA[i];`)を実際に
// `fxc.exe`でコンパイルし`examples/dump_shex`で実SHEXを確認した上で
// sub/divをこのチェーンクラスへ追加した(当初は「1シェーダーだけでは
// 正しい順序を検証しきれない」として明示的に拒否していたが、実際に
// このシェーダーで検証した)。ただし以下は引き続き未検証のスコープ外:
// - `mul`のnegateフラグが立つケース(このシェーダーでは発生しなかったため
//   未検証、遭遇したら明示的にエラーを返す)。
// - このシェーダー1本(1回のsub+1回のdiv)以外の組み合わせ・オペランド
//   並び(例: 3項以上のチェーンでのsub/divの重複使用)。
// 境界チェック(`ult`/`if`/`endif`)も今回のクラスでは対象外(既存クラス側の
// みが対応)。

/// 一時レジスタコンポーネントが実際に評価する式。`ld_structured`で読み込んだ
/// 値そのもの(`Load`)か、既存の2つの式を入力に取る2項演算(`BinOp`)の
/// いずれか——制御フローを含まない評価式の木。
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RegExpr {
    /// このUAVバインドポイントから読み込んだ値。
    Load(u32),
    /// 2つの部分式に対する2項演算の結果。
    BinOp(BinaryOp, Box<RegExpr>, Box<RegExpr>),
}

/// [`RegExpr`]の木を実際に辿り、含まれる`Load`(読み込み元UAV)を出現順に集める。
pub(crate) fn collect_loads(expr: &RegExpr, out: &mut Vec<u32>) {
    match expr {
        RegExpr::Load(uav) => out.push(*uav),
        RegExpr::BinOp(_, lhs, rhs) => {
            collect_loads(lhs, out);
            collect_loads(rhs, out);
        }
    }
}

/// 検証済みの「N個の逐次2項演算」シェーダー形状。
struct ChainShape {
    thread_group: (u32, u32, u32),
    write_uav: u32,
    /// `store_structured`が最終的に参照する式木(制御フロー無し)。
    root: RegExpr,
    /// `dcl_constantbuffer`(b0) + `ult` + `if`/`endif`による境界チェック
    /// (`vector_add_mul_chain_bounded.hlsl`実コンパイル結果で確認した形、
    /// 2026-08-06追加)。既存クラス側(`ShaderShape::bounds_check`)と同じ
    /// 「全部揃っているか、全く無いか」の規約。
    bounds_check: bool,
}

/// 一時レジスタのオペランドから`(temp_index, component_index)`キーを取り出す
/// (書き込み側は単一コンポーネントの`Mask`、読み込み側は`Scalar`選択のみ対応
/// ——`vector_add_mul_chain.dxbc`の実オペランド形状で確認済み)。
fn temp_key(op: &Operand, want_write: bool) -> Option<(u32, u32)> {
    if op.reg_type != RegisterType::Temp {
        return None;
    }
    let temp_index = match op.indices.first()? {
        OperandIndex::Imm32(i) => *i,
        _ => return None,
    };
    let component = if want_write {
        match op.components {
            ComponentSelect::Mask(m) if m.count_ones() == 1 => m.trailing_zeros(),
            _ => return None,
        }
    } else {
        match op.components {
            ComponentSelect::Scalar(c) => c as u32,
            _ => return None,
        }
    };
    Some((temp_index, component))
}

/// 実際のSHEX命令列を、「N個の逐次2項演算(制御フロー無し)」パターンクラスと
/// 突き合わせる。1命令でも想定外の形状があれば正直に拒否する。
fn decode_chain_shape(instructions: &[Instruction]) -> Result<ChainShape, SpirvGenError> {
    let mut declared_uavs: Vec<u32> = Vec::new();
    let mut thread_group: Option<(u32, u32, u32)> = None;
    let mut reg_map: HashMap<(u32, u32), RegExpr> = HashMap::new();
    let mut store_uav: Option<u32> = None;
    let mut root: Option<RegExpr> = None;
    let mut saw_ret = false;
    let mut has_cbuffer = false;
    let mut saw_ult = false;
    let mut saw_if = false;
    let mut saw_endif = false;

    for ins in instructions {
        match &ins.kind {
            InstructionKind::DclGlobalFlags { .. } => {}
            InstructionKind::DclConstantBuffer { operands, .. } => {
                // 既存クラス側(`decode_shader_shape`)と同じ規約: b0のみ対応。
                let op0 = operands.first().ok_or_else(|| {
                    SpirvGenError::UnsupportedShader("dcl_constantbufferにオペランドが無い".to_string())
                })?;
                if op0.reg_type != RegisterType::ConstantBuffer {
                    return Err(SpirvGenError::UnsupportedShader(
                        "dcl_constantbufferの対象レジスタがcbではない".to_string(),
                    ));
                }
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
                    // 既存クラス側(`decode_shader_shape`)と同じ規約:
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
                    let dest = operands.first().ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("ld_structuredの書き込み先オペランドが無い".to_string())
                    })?;
                    let dest_key = temp_key(dest, true).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader(
                            "ld_structuredの書き込み先が単一コンポーネントの一時レジスタではない".to_string(),
                        )
                    })?;
                    let idx_operand = operands.get(1).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("ld_structuredの添字オペランドが無い".to_string())
                    })?;
                    if idx_operand.reg_type != RegisterType::ThreadID {
                        return Err(SpirvGenError::UnsupportedShader(
                            "対応しているのはvThreadIDによる添字のみ".to_string(),
                        ));
                    }
                    let src_uav = operands.get(3).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("ld_structuredのUAVオペランドが無い".to_string())
                    })?;
                    if src_uav.reg_type != RegisterType::Uav {
                        return Err(SpirvGenError::UnsupportedShader(
                            "ld_structuredの読み込み元がUAVではない".to_string(),
                        ));
                    }
                    let uav = uav_index(&src_uav.indices).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("ld_structuredのUAVバインドポイントを解決できない".to_string())
                    })?;
                    reg_map.insert(dest_key, RegExpr::Load(uav));
                }
                Opcode::Add | Opcode::Mul | Opcode::Div => {
                    // 2026-07-27追加: sub/divをチェーンクラスへ対応
                    // (`vector_sub_div_chain.hlsl`を実際にfxc.exeでコンパイル・
                    // `examples/dump_shex`で実オペランド順序を確認した上での
                    // 実装、以前は"チェーン内でのnegateフラグは対応スコープ外"
                    // として明示的に拒否していた)。
                    //
                    // 実測した規約(t = A - B; Output = t / A;のSHEXダンプより):
                    // - `Add`でsrc1(operands[1])に`negate`が立っていれば
                    //   `dest = src2_val - src1_val`(既存の`decode_shader_shape`
                    //   と同じ「negated-addはsub」規約、チェーン内でも成立する
                    //   ことを実際に確認した)。
                    // - `Mul`はnegateフラグが立つケースは未検証のため引き続き
                    //   拒否する(実際に検証できたのはこのシェーダーのadd/div
                    //   のみ、既存のmulの扱いは変更しない)。
                    // - `Div`は`Add`/`Mul`とはオペランド順序が逆
                    //   (`dest = src1_val / src2_val`、swapしない)——
                    //   このモジュール冒頭のdocコメントで以前から「divは
                    //   オペランド順序がadd/mulと異なる」と記載されていた
                    //   通りであることを、このチェーンクラスでも実際に確認した。
                    let dest = operands.first().ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("add/mul/divの書き込み先オペランドが無い".to_string())
                    })?;
                    let dest_key = temp_key(dest, true).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader(
                            "add/mul/divの書き込み先が単一コンポーネントの一時レジスタではない".to_string(),
                        )
                    })?;
                    let src1 = operands.get(1).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("add/mul/divの第1ソースオペランドが無い".to_string())
                    })?;
                    let src2 = operands.get(2).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("add/mul/divの第2ソースオペランドが無い".to_string())
                    })?;
                    if ins.opcode != Opcode::Add && (src1.negate || src2.negate) {
                        return Err(SpirvGenError::UnsupportedShader(
                            "チェーン内でのnegateフラグはAdd(sub最適化)以外では未検証のため対応スコープ外".to_string(),
                        ));
                    }
                    let src1_key = temp_key(src1, false).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader(
                            "add/mul/divの第1ソースが一時レジスタのスカラー選択ではない".to_string(),
                        )
                    })?;
                    let src2_key = temp_key(src2, false).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader(
                            "add/mul/divの第2ソースが一時レジスタのスカラー選択ではない".to_string(),
                        )
                    })?;
                    let src1_val = reg_map.get(&src1_key).cloned().ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("add/mul/divの第1ソースがまだ定義されていない一時レジスタを参照している".to_string())
                    })?;
                    let src2_val = reg_map.get(&src2_key).cloned().ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("add/mul/divの第2ソースがまだ定義されていない一時レジスタを参照している".to_string())
                    })?;
                    let expr = match ins.opcode {
                        Opcode::Add if src1.negate => RegExpr::BinOp(BinaryOp::Sub, Box::new(src2_val), Box::new(src1_val)),
                        Opcode::Add => RegExpr::BinOp(BinaryOp::Add, Box::new(src2_val), Box::new(src1_val)),
                        Opcode::Mul => RegExpr::BinOp(BinaryOp::Mul, Box::new(src2_val), Box::new(src1_val)),
                        Opcode::Div => RegExpr::BinOp(BinaryOp::Div, Box::new(src1_val), Box::new(src2_val)),
                        _ => unreachable!("match arm limited to Add|Mul|Div above"),
                    };
                    reg_map.insert(dest_key, expr);
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
                    let idx_operand = operands.get(1).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("store_structuredの添字オペランドが無い".to_string())
                    })?;
                    if idx_operand.reg_type != RegisterType::ThreadID {
                        return Err(SpirvGenError::UnsupportedShader(
                            "対応しているのはvThreadIDによる添字のみ".to_string(),
                        ));
                    }
                    let val_operand = operands.get(3).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("store_structuredの書き込み値オペランドが無い".to_string())
                    })?;
                    let val_key = temp_key(val_operand, false).ok_or_else(|| {
                        SpirvGenError::UnsupportedShader(
                            "store_structuredの書き込み値が一時レジスタのスカラー選択ではない".to_string(),
                        )
                    })?;
                    root = Some(reg_map.get(&val_key).cloned().ok_or_else(|| {
                        SpirvGenError::UnsupportedShader("store_structuredがまだ定義されていない一時レジスタを参照している".to_string())
                    })?);
                }
                Opcode::Ret => {
                    saw_ret = true;
                }
                other => {
                    return Err(SpirvGenError::UnsupportedShader(format!(
                        "チェーンクラスの対応スコープ外のオペコード: {other:?}"
                    )));
                }
            },
            other => {
                return Err(SpirvGenError::UnsupportedShader(format!(
                    "チェーンクラスの対応スコープ外の宣言命令: {other:?}"
                )));
            }
        }
    }

    if declared_uavs.len() < 2 {
        return Err(SpirvGenError::UnsupportedShader(format!(
            "チェーンクラスはUAV2本以上(N入力+1出力)を想定するが{}本だった",
            declared_uavs.len()
        )));
    }
    let thread_group = thread_group
        .ok_or_else(|| SpirvGenError::UnsupportedShader("dcl_thread_groupが見つからない".to_string()))?;
    let write_uav = store_uav
        .ok_or_else(|| SpirvGenError::UnsupportedShader("store_structuredが見つからない".to_string()))?;
    let root = root.ok_or_else(|| {
        SpirvGenError::UnsupportedShader("store_structuredが式を書き込んでいない".to_string())
    })?;
    if !saw_ret {
        return Err(SpirvGenError::UnsupportedShader("ret命令が見つからない".to_string()));
    }
    // 少なくとも1回の2項演算(N>=1)を要求する(0回=単純コピーは対象外、
    // 既存の`decode_shader_shape`ともパターンが重ならないようにするため)。
    let mut loads = Vec::new();
    collect_loads(&root, &mut loads);
    if matches!(root, RegExpr::Load(_)) {
        return Err(SpirvGenError::UnsupportedShader(
            "2項演算を1回も含まない(単純コピー)シェーダーは対応スコープ外".to_string(),
        ));
    }
    // 境界チェックは「定数バッファ宣言 + ult + if + endif」が全部揃っている
    // か、全く無いかのどちらかのみ許容する(既存クラス側と同じ規約)。
    let bounds_check = has_cbuffer && saw_ult && saw_if && saw_endif;
    if has_cbuffer != bounds_check || saw_ult != bounds_check || saw_if != bounds_check || saw_endif != bounds_check {
        return Err(SpirvGenError::UnsupportedShader(
            "境界チェック構成(dcl_constantbuffer/ult/if/endif)が不完全".to_string(),
        ));
    }

    Ok(ChainShape { thread_group, write_uav, root, bounds_check })
}

/// [`translate_chain_shader`]が返す翻訳結果。既存の[`TranslatedKernel`]は
/// 読み込みUAVがちょうど2本という前提の3要素タプルを持つため、N本(N>=1)の
/// 読み込みUAVを表現できるよう別の型として定義する。
#[derive(Debug, Clone)]
pub struct ChainTranslatedKernel {
    pub spirv_words: Vec<u32>,
    pub entry_point: &'static str,
    pub local_size: (u32, u32, u32),
    /// 式木を実際に辿って集めた、読み込み元UAVバインドポイントの一覧
    /// (出現順、重複あり得る)。
    pub read_uav_bind_points: Vec<u32>,
    pub write_uav_bind_point: u32,
    /// `dcl_constantbuffer`(b0)+`ult`+`if`/`endif`による境界チェックが実際に
    /// このシェーダーに存在したかどうか(2026-08-06追加)。
    pub bounds_check: bool,
}

/// DXBCバイト列を解析し、「N個の逐次2項演算(制御フロー無し)」パターンクラス
/// (`decode_chain_shape`)に一致すれば実際のSHEX命令列を検証しながらSPIR-Vへ
/// 翻訳する。一致しなければ`SpirvGenError::UnsupportedShader`を返す
/// (既存の`translate_shader`とは独立した、別のエントリポイント)。
pub fn translate_chain_shader(bytes: &[u8]) -> Result<ChainTranslatedKernel, SpirvGenError> {
    let containers = scan_dxbc(bytes);
    let container = containers.into_iter().next().ok_or_else(|| {
        SpirvGenError::Translate(TranslateError::Parse("DXBCコンテナが見つからない".to_string()))
    })?;

    let mut instructions: Option<Vec<Instruction>> = None;
    for chunk in &container.chunks {
        if let ChunkData::Shader(program) = chunk.parse() {
            instructions = Some(program.instructions);
        }
    }
    let instructions = instructions.ok_or(SpirvGenError::Translate(TranslateError::MissingChunk("SHEX")))?;

    let shape = decode_chain_shape(&instructions)?;
    let mut read_uav_bind_points = Vec::new();
    collect_loads(&shape.root, &mut read_uav_bind_points);
    let spirv_words = emit_chain_spirv(&shape);

    Ok(ChainTranslatedKernel {
        spirv_words,
        entry_point: "main",
        local_size: shape.thread_group,
        read_uav_bind_points,
        write_uav_bind_point: shape.write_uav,
        bounds_check: shape.bounds_check,
    })
}

/// [`ChainShape`]から実際にSPIR-Vバイナリを組み立てる。DXBC側の薄いラッパー
/// ——実体は[`emit_chain_spirv_for_kernel`](DXIL側`dxil.rs`の
/// `translate_dxil_chain_to_spirv`とも共有する、DXBC固有の`ChainShape`型に
/// 依存しない部分を切り出したもの)。
fn emit_chain_spirv(shape: &ChainShape) -> Vec<u32> {
    emit_chain_spirv_for_kernel(shape.thread_group, &shape.root, shape.write_uav, shape.bounds_check)
}

/// [`emit_chain_spirv`]の本体。DXBC固有の[`ChainShape`]型に依存しない
/// パラメータのみを取る形へ切り出したもの(`emit_spirv_for_kernel`が
/// `emit_spirv_impl`から切り出されたのと同じパターン)——DXIL側
/// (`dxil.rs`)がDXBC側の`RegExpr`式木構築ロジックを再利用してSPIR-Vを
/// 生成する際にも、このSPIR-V組み立て本体をそのまま呼べるようにするため
/// `pub(crate)`にした(2026-08-05、DXILチェーン対応の一環)。`bounds_check`
/// が真の場合、既存クラス側(`emit_spirv_impl`)と同じpush constant
/// `Params{ uint n }`+`OpSelectionMerge`/`OpBranchConditional`で
/// `id.x < n`の比較を実際にゲートする(2026-08-06追加、DXBC側のみ——DXIL側
/// `dxil.rs`の呼び出しは境界チェック未対応のため常に`false`を渡す)。
pub(crate) fn emit_chain_spirv_for_kernel(
    thread_group: (u32, u32, u32),
    root: &RegExpr,
    write_uav: u32,
    bounds_check: bool,
) -> Vec<u32> {
    let mut b = Builder::new();
    b.set_version(1, 0);
    b.capability(spirv::Capability::Shader);
    b.memory_model(spirv::AddressingModel::Logical, spirv::MemoryModel::GLSL450);

    let void_ty = b.type_void();
    let voidf_ty = b.type_function(void_ty, vec![]);
    let float_ty = b.type_float(32, None);
    let uint_ty = b.type_int(32, 0);
    let uvec3_ty = b.type_vector(uint_ty, 3);

    let rt_array_ty = b.type_runtime_array(float_ty);
    b.decorate(rt_array_ty, spirv::Decoration::ArrayStride, vec![DrOperand::LiteralBit32(4)]);
    let buf_struct_ty = b.type_struct(vec![rt_array_ty]);
    b.decorate(buf_struct_ty, spirv::Decoration::BufferBlock, vec![]);
    b.member_decorate(buf_struct_ty, 0, spirv::Decoration::Offset, vec![DrOperand::LiteralBit32(0)]);
    let buf_ptr_ty = b.type_pointer(None, spirv::StorageClass::Uniform, buf_struct_ty);

    // 実際に式木に登場するUAVバインドポイント(読み込み+書き込み)ごとに、1つの
    // storage bufferバリアブルを作る(重複作成しないよう、既に作った分は
    // 再利用する)。
    let mut buffer_vars: HashMap<u32, u32> = HashMap::new();
    let ensure_buffer_var = |b: &mut Builder, binding: u32, vars: &mut HashMap<u32, u32>| -> u32 {
        *vars.entry(binding).or_insert_with(|| {
            let var = b.variable(buf_ptr_ty, None, spirv::StorageClass::Uniform, None);
            b.decorate(var, spirv::Decoration::DescriptorSet, vec![DrOperand::LiteralBit32(0)]);
            b.decorate(var, spirv::Decoration::Binding, vec![DrOperand::LiteralBit32(binding)]);
            var
        })
    };

    let mut all_uavs = Vec::new();
    collect_loads(root, &mut all_uavs);
    all_uavs.push(write_uav);
    for uav in &all_uavs {
        ensure_buffer_var(&mut b, *uav, &mut buffer_vars);
    }

    let gid_ptr_ty = b.type_pointer(None, spirv::StorageClass::Input, uvec3_ty);
    let var_gid = b.variable(gid_ptr_ty, None, spirv::StorageClass::Input, None);
    b.decorate(var_gid, spirv::Decoration::BuiltIn, vec![DrOperand::BuiltIn(spirv::BuiltIn::GlobalInvocationId)]);

    let float_ptr_uniform_ty = b.type_pointer(None, spirv::StorageClass::Uniform, float_ty);

    // push constant: struct Params { uint n; }(既存クラス側`emit_spirv_impl`
    // と同じレイアウト、`bounds_check`が真の場合のみ実際に使う)。
    let params_struct_ty = b.type_struct(vec![uint_ty]);
    b.decorate(params_struct_ty, spirv::Decoration::Block, vec![]);
    b.member_decorate(params_struct_ty, 0, spirv::Decoration::Offset, vec![DrOperand::LiteralBit32(0)]);
    let params_ptr_ty = b.type_pointer(None, spirv::StorageClass::PushConstant, params_struct_ty);
    let var_params = b.variable(params_ptr_ty, None, spirv::StorageClass::PushConstant, None);
    let uint_ptr_pushconstant_ty = b.type_pointer(None, spirv::StorageClass::PushConstant, uint_ty);
    let bool_ty = b.type_bool();

    let main_fn = b.begin_function(void_ty, None, spirv::FunctionControl::NONE, voidf_ty).expect("OpFunction");
    b.begin_block(None).expect("OpLabel");

    let const_0 = b.constant_bit32(uint_ty, 0);
    let gid_vec = b.load(uvec3_ty, None, var_gid, None, vec![]).expect("OpLoad gid");
    let idx = b.composite_extract(uint_ty, None, gid_vec, vec![0]).expect("OpCompositeExtract .x");

    fn emit_expr(
        b: &mut Builder,
        expr: &RegExpr,
        buffer_vars: &HashMap<u32, u32>,
        float_ptr_uniform_ty: u32,
        float_ty: u32,
        const_0: u32,
        idx: u32,
    ) -> u32 {
        match expr {
            RegExpr::Load(uav) => {
                let var = *buffer_vars.get(uav).expect("buffer var must exist for every referenced UAV");
                let ac = b.access_chain(float_ptr_uniform_ty, None, var, vec![const_0, idx]).expect("OpAccessChain");
                b.load(float_ty, None, ac, None, vec![]).expect("OpLoad")
            }
            RegExpr::BinOp(op, lhs, rhs) => {
                let l = emit_expr(b, lhs, buffer_vars, float_ptr_uniform_ty, float_ty, const_0, idx);
                let r = emit_expr(b, rhs, buffer_vars, float_ptr_uniform_ty, float_ty, const_0, idx);
                match op {
                    BinaryOp::Add => b.f_add(float_ty, None, l, r).expect("OpFAdd"),
                    BinaryOp::Mul => b.f_mul(float_ty, None, l, r).expect("OpFMul"),
                    BinaryOp::Sub => b.f_sub(float_ty, None, l, r).expect("OpFSub"),
                    BinaryOp::Div => b.f_div(float_ty, None, l, r).expect("OpFDiv"),
                    // `decode_chain_shape`は`Add`以外のnegateを明示的に拒否
                    // するため(このモジュール内の該当コメント参照)、チェーン
                    // 内の式木に`MulNeg`が現れることは無い——単独`vector_mul`
                    // シェーダー専用の`decode_shader_shape`のみが生成する。
                    BinaryOp::MulNeg => unreachable!(
                        "decode_chain_shapeはMulNegを生成しない(Add以外のnegateは拒否済み)"
                    ),
                }
            }
        }
    }

    // 本体(式木の評価+書き込み)を組み立てるクロージャ。境界チェック無しの
    // 場合は現在のブロックへ直接、有りの場合は`if`ブロック内へ、それぞれ
    // 同じ命令列を発行する(既存クラス側`emit_spirv_impl`の`emit_body`と
    // 同じパターン)。
    let emit_body = |b: &mut Builder| {
        let result = emit_expr(b, root, &buffer_vars, float_ptr_uniform_ty, float_ty, const_0, idx);
        let write_var = *buffer_vars.get(&write_uav).expect("write buffer var must exist");
        let ac_out =
            b.access_chain(float_ptr_uniform_ty, None, write_var, vec![const_0, idx]).expect("OpAccessChain out");
        b.store(ac_out, result, None, vec![]).expect("OpStore out");
    };

    if bounds_check {
        let n_ptr = b
            .access_chain(uint_ptr_pushconstant_ty, None, var_params, vec![const_0])
            .expect("OpAccessChain params.n");
        let n_val = b.load(uint_ty, None, n_ptr, None, vec![]).expect("OpLoad n");
        let cond = b.u_less_than(bool_ty, None, idx, n_val).expect("OpULessThan idx < n");

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
        [thread_group.0, thread_group.1, thread_group.2],
    );

    let module = b.module();
    module.assemble()
}

// ---------------------------------------------------------------------
// ここから先(D3D11グラフィックスパイプライン: VS/PS向けSPIR-V生成)は今回
// 新規に追加した部分。Compute Shader向けの上記コード(`decode_shader_shape`/
// `decode_chain_shape`とその周辺)は一切変更していない。
// ---------------------------------------------------------------------
//
// **スコープの正直な開示**: これは汎用SM5.0頂点/ピクセルシェーダーデコーダ
// ではない。以下の2つの実シェーダー(いずれも`fxc.exe /T vs_5_0`・
// `/T ps_5_0`で実際にコンパイルし、`examples/dump_shex.rs`で実SHEX命令列を
// 確認した上でサポートを追加したもの)だけを対象にした翻訳器である:
//
// 1. `shaders/triangle_vs.hlsl` — `POSITION`(v0, xyz)・`COLOR`(v1, xyzw)を
//    入力に取り、`SV_POSITION`(o0, xyzw、ただしwは`mov`で1.0固定)・
//    `COLOR`(o1, xyzw)をパススルーで出力する頂点シェーダー。実SHEX命令列
//    (`examples/dump_shex.rs`で実際にダンプして確認、番号は0始まり):
//    `dcl_globalFlags` -> `dcl_input`(v0, mask=7=xyz) ->
//    `dcl_input`(v1, mask=15=xyzw) -> `dcl_output_siv`(o0, mask=15, SV_POSITION) ->
//    `dcl_output`(o1, mask=15) -> `mov o0.xyz, v0.xyzx` -> `mov o0.w, l(1.0)` ->
//    `mov o1.xyzw, v1.xyzw` -> `ret`。
// 2. `shaders/triangle_ps.hlsl` — `COLOR`(v1, `linear`補間, xyzw)を入力に
//    取り、`SV_TARGET`(o0, xyzw)へパススルーで出力するピクセルシェーダー。
//    実SHEX命令列: `dcl_globalFlags` -> `dcl_input_ps`(linear, v1, mask=15) ->
//    `dcl_output`(o0, mask=15) -> `mov o0.xyzw, v1.xyzw` -> `ret`。
//
// 上記2パターン以外の命令列が1つでも混ざっている場合は
// `SpirvGenError::UnsupportedShader`を返し、誤った"対応している"という
// シグナルは出さない(既存のCompute Shader側と同じ方針)。この2シェーダーは
// 命令列が完全に固定(パラメータ化できる可変要素が実質無い——UAVバインド
// ポイントのような抽出対象すら存在しない、単純なパススルー)なので、
// `decode_*_shape`は「この通りの命令列か」を検証するだけの関数であり、
// 抽出した値を使ってSPIR-Vを組み立てるのではなく、検証が通った場合にのみ
// 対応する固定のSPIR-Vモジュールを返す。
//
// 出力するSPIR-Vは、Compute Shader側(`emit_spirv_impl`)とは根本的に異なる
// SPIR-V実行モデルを使う: `OpEntryPoint Vertex`/`OpEntryPoint Fragment`
// (`GLCompute`ではない)、`Input`/`Output`ストレージクラスの変数
// (storage bufferではない)、頂点シェーダーの`SV_POSITION`出力には
// `BuiltIn Position`デコレーション、それ以外の入出力には`Location`
// デコレーションを付与する。フラグメントシェーダーには
// `OpExecutionMode ... OriginUpperLeft`が必須(Vulkanの規約)。
//
// **`opencuda-vulkan`との配線について(正直な開示)**: `opencuda-vulkan`の
// `VulkanDevice`はCompute専用(`launch_kernel`のみ)で、グラフィックス
// パイプライン(`VkGraphicsPipelineCreateInfo`・レンダーパス・ラスタライザ・
// フレームバッファ)を一切持たない。本セクションはSPIR-V生成
// (`rspirv`の構造検証+`spirv-val`による外部検証)までを対象とし、実際に
// Vulkanへディスパッチして三角形を描画する処理は含まない(下記HANDOFF
// エントリに詳細な理由を記載)。

/// 頂点シェーダー(`triangle_vs.hlsl`)の入出力に対応するSPIR-V IDの集合。
struct VsIds {
    var_in_pos: u32,
    var_in_color: u32,
    var_out_position: u32,
    var_out_color: u32,
}

/// 実SHEX命令列を、`triangle_vs.hlsl`が実際に生成する固定の命令列と厳密に
/// 突き合わせる。1命令でも一致しなければ対応スコープ外として拒否する。
fn decode_vertex_shader_shape(instructions: &[Instruction]) -> Result<(), SpirvGenError> {
    let reject = |msg: &str| Err(SpirvGenError::UnsupportedShader(format!("VS: {msg}")));

    let mut it = instructions.iter();
    let mut next = |what: &str| -> Result<&Instruction, SpirvGenError> {
        it.next()
            .ok_or_else(|| SpirvGenError::UnsupportedShader(format!("VS: 命令列が短すぎる({what}が見つからない)")))
    };

    match &next("dcl_globalFlags")?.kind {
        InstructionKind::DclGlobalFlags { .. } => {}
        _ => return reject("先頭はdcl_globalFlagsのはず"),
    }

    // dcl_input v0 (POSITION, mask=xyz=7)
    match &next("dcl_input v0")?.kind {
        InstructionKind::DclInput { operands, .. } => {
            let op0 = operands.first().ok_or_else(|| SpirvGenError::UnsupportedShader("VS: dcl_inputにオペランドが無い".to_string()))?;
            if op0.reg_type != RegisterType::Input || uav_index(&op0.indices) != Some(0) || op0.components != ComponentSelect::Mask(7) {
                return reject("dcl_inputの1つ目はv0・mask=7(xyz)のはず");
            }
        }
        _ => return reject("2番目はdcl_inputのはず"),
    }

    // dcl_input v1 (COLOR, mask=xyzw=15)
    match &next("dcl_input v1")?.kind {
        InstructionKind::DclInput { operands, .. } => {
            let op0 = operands.first().ok_or_else(|| SpirvGenError::UnsupportedShader("VS: dcl_inputにオペランドが無い".to_string()))?;
            if op0.reg_type != RegisterType::Input || uav_index(&op0.indices) != Some(1) || op0.components != ComponentSelect::Mask(15) {
                return reject("dcl_inputの2つ目はv1・mask=15(xyzw)のはず");
            }
        }
        _ => return reject("3番目はdcl_inputのはず"),
    }

    // dcl_output_siv o0, SV_POSITION
    let ins = next("dcl_output_siv o0")?;
    match &ins.kind {
        InstructionKind::DclOutput { system_value, operands } => {
            let op0 = operands.first().ok_or_else(|| SpirvGenError::UnsupportedShader("VS: dcl_output_sivにオペランドが無い".to_string()))?;
            if ins.opcode != Opcode::DclOutputSiv
                || *system_value != Some("position")
                || op0.reg_type != RegisterType::Output
                || uav_index(&op0.indices) != Some(0)
            {
                return reject("4番目はdcl_output_siv o0(SV_POSITION)のはず");
            }
        }
        _ => return reject("4番目はdcl_outputのはず"),
    }

    // dcl_output o1 (COLOR)
    match &next("dcl_output o1")?.kind {
        InstructionKind::DclOutput { system_value, operands } => {
            let op0 = operands.first().ok_or_else(|| SpirvGenError::UnsupportedShader("VS: dcl_outputにオペランドが無い".to_string()))?;
            if system_value.is_some() || op0.reg_type != RegisterType::Output || uav_index(&op0.indices) != Some(1) {
                return reject("5番目はdcl_output o1(COLOR、SVでない)のはず");
            }
        }
        _ => return reject("5番目はdcl_outputのはず"),
    }

    // mov o0.xyz, v0.xyzx
    match &next("mov o0.xyz, v0")?.kind {
        InstructionKind::Generic { operands } if operands.len() == 2 => {
            let dst = &operands[0];
            let src = &operands[1];
            if dst.reg_type != RegisterType::Output || uav_index(&dst.indices) != Some(0) || dst.components != ComponentSelect::Mask(7) {
                return reject("6番目のmovの書き込み先がo0.xyzではない");
            }
            if src.reg_type != RegisterType::Input || uav_index(&src.indices) != Some(0) {
                return reject("6番目のmovの読み込み元がv0ではない");
            }
        }
        _ => return reject("6番目はmovのはず"),
    }

    // mov o0.w, l(1.0)
    match &next("mov o0.w, l(1.0)")?.kind {
        InstructionKind::Generic { operands } if operands.len() == 2 => {
            let dst = &operands[0];
            let src = &operands[1];
            if dst.reg_type != RegisterType::Output || uav_index(&dst.indices) != Some(0) || dst.components != ComponentSelect::Mask(8) {
                return reject("7番目のmovの書き込み先がo0.wではない");
            }
            if src.reg_type != RegisterType::Immediate32 || src.immediate_values.first() != Some(&1065353216) {
                return reject("7番目のmovの読み込み元が定数1.0fではない");
            }
        }
        _ => return reject("7番目はmovのはず"),
    }

    // mov o1.xyzw, v1.xyzw
    match &next("mov o1.xyzw, v1")?.kind {
        InstructionKind::Generic { operands } if operands.len() == 2 => {
            let dst = &operands[0];
            let src = &operands[1];
            if dst.reg_type != RegisterType::Output || uav_index(&dst.indices) != Some(1) || dst.components != ComponentSelect::Mask(15) {
                return reject("8番目のmovの書き込み先がo1.xyzwではない");
            }
            if src.reg_type != RegisterType::Input || uav_index(&src.indices) != Some(1) {
                return reject("8番目のmovの読み込み元がv1ではない");
            }
        }
        _ => return reject("8番目はmovのはず"),
    }

    match &next("ret")?.kind {
        InstructionKind::Generic { operands } if operands.is_empty() => {}
        _ => return reject("9番目はretのはず"),
    }

    if it.next().is_some() {
        return reject("想定より命令が多い(9命令ちょうどのはず)");
    }

    Ok(())
}

/// 実SHEX命令列を、`triangle_ps.hlsl`が実際に生成する固定の命令列と厳密に
/// 突き合わせる。1命令でも一致しなければ対応スコープ外として拒否する。
fn decode_pixel_shader_shape(instructions: &[Instruction]) -> Result<(), SpirvGenError> {
    let reject = |msg: &str| Err(SpirvGenError::UnsupportedShader(format!("PS: {msg}")));

    let mut it = instructions.iter();
    let mut next = |what: &str| -> Result<&Instruction, SpirvGenError> {
        it.next()
            .ok_or_else(|| SpirvGenError::UnsupportedShader(format!("PS: 命令列が短すぎる({what}が見つからない)")))
    };

    match &next("dcl_globalFlags")?.kind {
        InstructionKind::DclGlobalFlags { .. } => {}
        _ => return reject("先頭はdcl_globalFlagsのはず"),
    }

    // dcl_input_ps (linear) v1, COLOR
    let ins = next("dcl_input_ps v1")?;
    match &ins.kind {
        InstructionKind::DclInput { interpolation, operands, .. } => {
            let op0 = operands.first().ok_or_else(|| SpirvGenError::UnsupportedShader("PS: dcl_input_psにオペランドが無い".to_string()))?;
            if *interpolation != Some("linear")
                || op0.reg_type != RegisterType::Input
                || uav_index(&op0.indices) != Some(1)
                || op0.components != ComponentSelect::Mask(15)
            {
                return reject("2番目はdcl_input_ps(linear) v1・mask=15のはず");
            }
        }
        _ => return reject("2番目はdcl_inputのはず"),
    }

    // dcl_output o0, SV_TARGET (no explicit system_value in this decoder's output)
    match &next("dcl_output o0")?.kind {
        InstructionKind::DclOutput { operands, .. } => {
            let op0 = operands.first().ok_or_else(|| SpirvGenError::UnsupportedShader("PS: dcl_outputにオペランドが無い".to_string()))?;
            if op0.reg_type != RegisterType::Output || uav_index(&op0.indices) != Some(0) || op0.components != ComponentSelect::Mask(15) {
                return reject("3番目はdcl_output o0・mask=15のはず");
            }
        }
        _ => return reject("3番目はdcl_outputのはず"),
    }

    // mov o0.xyzw, v1.xyzw
    match &next("mov o0, v1")?.kind {
        InstructionKind::Generic { operands } if operands.len() == 2 => {
            let dst = &operands[0];
            let src = &operands[1];
            if dst.reg_type != RegisterType::Output || uav_index(&dst.indices) != Some(0) || dst.components != ComponentSelect::Mask(15) {
                return reject("4番目のmovの書き込み先がo0.xyzwではない");
            }
            if src.reg_type != RegisterType::Input || uav_index(&src.indices) != Some(1) {
                return reject("4番目のmovの読み込み元がv1ではない");
            }
        }
        _ => return reject("4番目はmovのはず"),
    }

    match &next("ret")?.kind {
        InstructionKind::Generic { operands } if operands.is_empty() => {}
        _ => return reject("5番目はretのはず"),
    }

    if it.next().is_some() {
        return reject("想定より命令が多い(5命令ちょうどのはず)");
    }

    Ok(())
}

/// VS/PS共通のSPIR-Vモジュール組み立て結果。`local_size`もUAVバインド
/// ポイントも持たない(グラフィックスシェーダーにその概念が無いため、
/// Compute向けの`TranslatedKernel`とはフィールドが異なる別の型として定義)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderStage {
    Vertex,
    Fragment,
}

#[derive(Debug, Clone)]
pub struct GraphicsTranslatedKernel {
    pub spirv_words: Vec<u32>,
    pub entry_point: &'static str,
    pub stage: ShaderStage,
}

/// DXBCバイト列(`triangle_vs.hlsl`相当のD3D11頂点シェーダー、SM5.0)を解析し、
/// 実際のSHEX命令列を検証しながらSPIR-Vへ翻訳する。一致しなければ
/// `SpirvGenError::UnsupportedShader`を返す。
pub fn translate_vertex_shader(bytes: &[u8]) -> Result<GraphicsTranslatedKernel, SpirvGenError> {
    let instructions = shex_instructions(bytes)?;
    decode_vertex_shader_shape(&instructions)?;
    Ok(GraphicsTranslatedKernel {
        spirv_words: emit_vertex_spirv(),
        entry_point: "main",
        stage: ShaderStage::Vertex,
    })
}

/// DXBCバイト列(`triangle_ps.hlsl`相当のD3D11ピクセルシェーダー、SM5.0)を
/// 解析し、実際のSHEX命令列を検証しながらSPIR-Vへ翻訳する。一致しなければ
/// `SpirvGenError::UnsupportedShader`を返す。
pub fn translate_pixel_shader(bytes: &[u8]) -> Result<GraphicsTranslatedKernel, SpirvGenError> {
    let instructions = shex_instructions(bytes)?;
    decode_pixel_shader_shape(&instructions)?;
    Ok(GraphicsTranslatedKernel {
        spirv_words: emit_pixel_spirv(),
        entry_point: "main",
        stage: ShaderStage::Fragment,
    })
}

/// 共通のDXBC->SHEX命令列取り出し処理(`translate_shader`/`translate_chain_shader`
/// と同じロジックだが、VS/PSはこれら2つとは別のエントリポイント関数から
/// 呼ばれるため、小さな共有ヘルパーとして切り出した)。
fn shex_instructions(bytes: &[u8]) -> Result<Vec<Instruction>, SpirvGenError> {
    let containers = scan_dxbc(bytes);
    let container = containers.into_iter().next().ok_or_else(|| {
        SpirvGenError::Translate(TranslateError::Parse("DXBCコンテナが見つからない".to_string()))
    })?;
    let mut instructions: Option<Vec<Instruction>> = None;
    for chunk in &container.chunks {
        if let ChunkData::Shader(program) = chunk.parse() {
            instructions = Some(program.instructions);
        }
    }
    instructions.ok_or(SpirvGenError::Translate(TranslateError::MissingChunk("SHEX")))
}

/// `triangle_vs.hlsl`が検証を通った場合にのみ返す、固定のSPIR-Vモジュール。
/// `OpEntryPoint Vertex`、入力2本(`Location 0`=POSITION vec3, `Location 1`=
/// COLOR vec4)、出力2本(`BuiltIn Position` vec4, `Location 0`=COLOR vec4)。
fn emit_vertex_spirv() -> Vec<u32> {
    let mut b = Builder::new();
    b.set_version(1, 0);
    b.capability(spirv::Capability::Shader);
    b.memory_model(spirv::AddressingModel::Logical, spirv::MemoryModel::GLSL450);

    let void_ty = b.type_void();
    let voidf_ty = b.type_function(void_ty, vec![]);
    let float_ty = b.type_float(32, None);
    let vec3_ty = b.type_vector(float_ty, 3);
    let vec4_ty = b.type_vector(float_ty, 4);

    let ids = {
        let in_pos_ptr_ty = b.type_pointer(None, spirv::StorageClass::Input, vec3_ty);
        let var_in_pos = b.variable(in_pos_ptr_ty, None, spirv::StorageClass::Input, None);
        b.decorate(var_in_pos, spirv::Decoration::Location, vec![DrOperand::LiteralBit32(0)]);

        let in_color_ptr_ty = b.type_pointer(None, spirv::StorageClass::Input, vec4_ty);
        let var_in_color = b.variable(in_color_ptr_ty, None, spirv::StorageClass::Input, None);
        b.decorate(var_in_color, spirv::Decoration::Location, vec![DrOperand::LiteralBit32(1)]);

        let out_position_ptr_ty = b.type_pointer(None, spirv::StorageClass::Output, vec4_ty);
        let var_out_position = b.variable(out_position_ptr_ty, None, spirv::StorageClass::Output, None);
        b.decorate(var_out_position, spirv::Decoration::BuiltIn, vec![DrOperand::BuiltIn(spirv::BuiltIn::Position)]);

        let out_color_ptr_ty = b.type_pointer(None, spirv::StorageClass::Output, vec4_ty);
        let var_out_color = b.variable(out_color_ptr_ty, None, spirv::StorageClass::Output, None);
        b.decorate(var_out_color, spirv::Decoration::Location, vec![DrOperand::LiteralBit32(0)]);

        VsIds { var_in_pos, var_in_color, var_out_position, var_out_color }
    };

    let main_fn = b.begin_function(void_ty, None, spirv::FunctionControl::NONE, voidf_ty).expect("OpFunction");
    b.begin_block(None).expect("OpLabel");

    // o0.xyz = v0.xyz (POSITION passthrough)
    let pos_in = b.load(vec3_ty, None, ids.var_in_pos, None, vec![]).expect("OpLoad pos");
    let px = b.composite_extract(float_ty, None, pos_in, vec![0]).expect("extract x");
    let py = b.composite_extract(float_ty, None, pos_in, vec![1]).expect("extract y");
    let pz = b.composite_extract(float_ty, None, pos_in, vec![2]).expect("extract z");
    let one = b.constant_bit32(float_ty, 1.0f32.to_bits());
    // o0.w = 1.0 (mov o0.w, l(1.0))
    let pos_out = b.composite_construct(vec4_ty, None, vec![px, py, pz, one]).expect("construct SV_POSITION");
    b.store(ids.var_out_position, pos_out, None, vec![]).expect("OpStore SV_POSITION");

    // o1 = v1 (COLOR passthrough)
    let color_in = b.load(vec4_ty, None, ids.var_in_color, None, vec![]).expect("OpLoad color");
    b.store(ids.var_out_color, color_in, None, vec![]).expect("OpStore COLOR");

    b.ret().expect("OpReturn");
    b.end_function().expect("OpFunctionEnd");

    b.entry_point(
        spirv::ExecutionModel::Vertex,
        main_fn,
        "main",
        vec![ids.var_in_pos, ids.var_in_color, ids.var_out_position, ids.var_out_color],
    );

    let module = b.module();
    module.assemble()
}

/// `triangle_ps.hlsl`が検証を通った場合にのみ返す、固定のSPIR-Vモジュール。
/// `OpEntryPoint Fragment`、入力1本(`Location 0`=COLOR vec4)、出力1本
/// (`Location 0`=SV_TARGET vec4)。`OpExecutionMode ... OriginUpperLeft`は
/// Vulkanのフラグメントシェーダーで必須のため付与する。
fn emit_pixel_spirv() -> Vec<u32> {
    let mut b = Builder::new();
    b.set_version(1, 0);
    b.capability(spirv::Capability::Shader);
    b.memory_model(spirv::AddressingModel::Logical, spirv::MemoryModel::GLSL450);

    let void_ty = b.type_void();
    let voidf_ty = b.type_function(void_ty, vec![]);
    let float_ty = b.type_float(32, None);
    let vec4_ty = b.type_vector(float_ty, 4);

    let in_color_ptr_ty = b.type_pointer(None, spirv::StorageClass::Input, vec4_ty);
    let var_in_color = b.variable(in_color_ptr_ty, None, spirv::StorageClass::Input, None);
    b.decorate(var_in_color, spirv::Decoration::Location, vec![DrOperand::LiteralBit32(0)]);

    let out_color_ptr_ty = b.type_pointer(None, spirv::StorageClass::Output, vec4_ty);
    let var_out_color = b.variable(out_color_ptr_ty, None, spirv::StorageClass::Output, None);
    b.decorate(var_out_color, spirv::Decoration::Location, vec![DrOperand::LiteralBit32(0)]);

    let main_fn = b.begin_function(void_ty, None, spirv::FunctionControl::NONE, voidf_ty).expect("OpFunction");
    b.begin_block(None).expect("OpLabel");

    let color = b.load(vec4_ty, None, var_in_color, None, vec![]).expect("OpLoad color");
    b.store(var_out_color, color, None, vec![]).expect("OpStore SV_TARGET");

    b.ret().expect("OpReturn");
    b.end_function().expect("OpFunctionEnd");

    b.entry_point(spirv::ExecutionModel::Fragment, main_fn, "main", vec![var_in_color, var_out_color]);
    b.execution_mode(main_fn, spirv::ExecutionMode::OriginUpperLeft, []);

    let module = b.module();
    module.assemble()
}

/// 実SHEX命令列を、`sprite_vs.hlsl`が実際に生成する固定の命令列と厳密に
/// 突き合わせる(2026-08-08、2Dスプライト描画プロトタイプの第一歩)。
/// `triangle_vs.hlsl`(COLORパススルー、mask=15)とほぼ同じ骨格だが、
/// COLORの代わりにTEXCOORD(mask=3、xyの2成分)をパススルーする点のみ
/// 異なる——実`fxc.exe`出力(`sprite_vs.dxbc`)を`examples/dump_shex`で
/// ダンプして確認した上で実装した。
fn decode_sprite_vertex_shader_shape(instructions: &[Instruction]) -> Result<(), SpirvGenError> {
    let reject = |msg: &str| Err(SpirvGenError::UnsupportedShader(format!("Sprite VS: {msg}")));

    let mut it = instructions.iter();
    let mut next = |what: &str| -> Result<&Instruction, SpirvGenError> {
        it.next()
            .ok_or_else(|| SpirvGenError::UnsupportedShader(format!("Sprite VS: 命令列が短すぎる({what}が見つからない)")))
    };

    match &next("dcl_globalFlags")?.kind {
        InstructionKind::DclGlobalFlags { .. } => {}
        _ => return reject("先頭はdcl_globalFlagsのはず"),
    }

    // dcl_input v0 (POSITION, mask=xyz=7)
    match &next("dcl_input v0")?.kind {
        InstructionKind::DclInput { operands, .. } => {
            let op0 = operands.first().ok_or_else(|| SpirvGenError::UnsupportedShader("Sprite VS: dcl_inputにオペランドが無い".to_string()))?;
            if op0.reg_type != RegisterType::Input || uav_index(&op0.indices) != Some(0) || op0.components != ComponentSelect::Mask(7) {
                return reject("dcl_inputの1つ目はv0・mask=7(xyz)のはず");
            }
        }
        _ => return reject("2番目はdcl_inputのはず"),
    }

    // dcl_input v1 (TEXCOORD, mask=xy=3)
    match &next("dcl_input v1")?.kind {
        InstructionKind::DclInput { operands, .. } => {
            let op0 = operands.first().ok_or_else(|| SpirvGenError::UnsupportedShader("Sprite VS: dcl_inputにオペランドが無い".to_string()))?;
            if op0.reg_type != RegisterType::Input || uav_index(&op0.indices) != Some(1) || op0.components != ComponentSelect::Mask(3) {
                return reject("dcl_inputの2つ目はv1・mask=3(xy)のはず");
            }
        }
        _ => return reject("3番目はdcl_inputのはず"),
    }

    // dcl_output_siv o0, SV_POSITION
    match &next("dcl_output_siv o0")?.kind {
        InstructionKind::DclOutput { system_value, operands } => {
            let op0 = operands.first().ok_or_else(|| SpirvGenError::UnsupportedShader("Sprite VS: dcl_output_sivにオペランドが無い".to_string()))?;
            if *system_value != Some("position") || op0.reg_type != RegisterType::Output || uav_index(&op0.indices) != Some(0) {
                return reject("4番目はdcl_output_siv o0(SV_POSITION)のはず");
            }
        }
        _ => return reject("4番目はdcl_outputのはず"),
    }

    // dcl_output o1 (TEXCOORD, mask=xy=3)
    match &next("dcl_output o1")?.kind {
        InstructionKind::DclOutput { system_value, operands } => {
            let op0 = operands.first().ok_or_else(|| SpirvGenError::UnsupportedShader("Sprite VS: dcl_outputにオペランドが無い".to_string()))?;
            if system_value.is_some() || op0.reg_type != RegisterType::Output || uav_index(&op0.indices) != Some(1) || op0.components != ComponentSelect::Mask(3) {
                return reject("5番目はdcl_output o1(TEXCOORD、mask=3、SVでない)のはず");
            }
        }
        _ => return reject("5番目はdcl_outputのはず"),
    }

    // mov o0.xyz, v0.xyzx
    match &next("mov o0.xyz, v0")?.kind {
        InstructionKind::Generic { operands } if operands.len() == 2 => {
            let dst = &operands[0];
            let src = &operands[1];
            if dst.reg_type != RegisterType::Output || uav_index(&dst.indices) != Some(0) || dst.components != ComponentSelect::Mask(7) {
                return reject("6番目のmovの書き込み先がo0.xyzではない");
            }
            if src.reg_type != RegisterType::Input || uav_index(&src.indices) != Some(0) {
                return reject("6番目のmovの読み込み元がv0ではない");
            }
        }
        _ => return reject("6番目はmovのはず"),
    }

    // mov o0.w, l(1.0)
    match &next("mov o0.w, l(1.0)")?.kind {
        InstructionKind::Generic { operands } if operands.len() == 2 => {
            let dst = &operands[0];
            let src = &operands[1];
            if dst.reg_type != RegisterType::Output || uav_index(&dst.indices) != Some(0) || dst.components != ComponentSelect::Mask(8) {
                return reject("7番目のmovの書き込み先がo0.wではない");
            }
            if src.reg_type != RegisterType::Immediate32 || src.immediate_values.first() != Some(&1065353216) {
                return reject("7番目のmovの読み込み元が定数1.0fではない");
            }
        }
        _ => return reject("7番目はmovのはず"),
    }

    // mov o1.xy, v1.xy
    match &next("mov o1.xy, v1")?.kind {
        InstructionKind::Generic { operands } if operands.len() == 2 => {
            let dst = &operands[0];
            let src = &operands[1];
            if dst.reg_type != RegisterType::Output || uav_index(&dst.indices) != Some(1) || dst.components != ComponentSelect::Mask(3) {
                return reject("8番目のmovの書き込み先がo1.xyではない");
            }
            if src.reg_type != RegisterType::Input || uav_index(&src.indices) != Some(1) {
                return reject("8番目のmovの読み込み元がv1ではない");
            }
        }
        _ => return reject("8番目はmovのはず"),
    }

    match &next("ret")?.kind {
        InstructionKind::Generic { operands } if operands.is_empty() => {}
        _ => return reject("9番目はretのはず"),
    }

    if it.next().is_some() {
        return reject("想定より命令が多い(9命令ちょうどのはず)");
    }

    Ok(())
}

/// 実SHEX命令列を、`sprite_ps.hlsl`が実際に生成する固定の命令列と厳密に
/// 突き合わせる(2026-08-08)。`triangle_ps.hlsl`とは異なり、テクスチャ
/// サンプリング(`dcl_sampler`/`dcl_resource`/`sample`)を含む——本
/// リポジトリで初めてテクスチャサンプリングに対応した増分。
fn decode_sprite_pixel_shader_shape(instructions: &[Instruction]) -> Result<(), SpirvGenError> {
    let reject = |msg: &str| Err(SpirvGenError::UnsupportedShader(format!("Sprite PS: {msg}")));

    let mut it = instructions.iter();
    let mut next = |what: &str| -> Result<&Instruction, SpirvGenError> {
        it.next()
            .ok_or_else(|| SpirvGenError::UnsupportedShader(format!("Sprite PS: 命令列が短すぎる({what}が見つからない)")))
    };

    match &next("dcl_globalFlags")?.kind {
        InstructionKind::DclGlobalFlags { .. } => {}
        _ => return reject("先頭はdcl_globalFlagsのはず"),
    }

    // dcl_sampler s0 (default mode)
    match &next("dcl_sampler s0")?.kind {
        InstructionKind::DclSampler { mode, operands } => {
            let op0 = operands.first().ok_or_else(|| SpirvGenError::UnsupportedShader("Sprite PS: dcl_samplerにオペランドが無い".to_string()))?;
            if *mode != "default" || op0.reg_type != RegisterType::Sampler || uav_index(&op0.indices) != Some(0) {
                return reject("2番目はdcl_sampler s0(defaultモード)のはず");
            }
        }
        _ => return reject("2番目はdcl_samplerのはず"),
    }

    // dcl_resource_texture2d t0, float4
    match &next("dcl_resource t0")?.kind {
        InstructionKind::DclResource { dimension, return_type, operands, .. } => {
            let op0 = operands.first().ok_or_else(|| SpirvGenError::UnsupportedShader("Sprite PS: dcl_resourceにオペランドが無い".to_string()))?;
            let all_float = return_type.len() == 4 && return_type.iter().all(|t| format!("{t:?}") == "Float");
            if *dimension != "texture2d" || !all_float || op0.reg_type != RegisterType::Resource || uav_index(&op0.indices) != Some(0) {
                return reject("3番目はdcl_resource_texture2d t0(float4)のはず");
            }
        }
        _ => return reject("3番目はdcl_resourceのはず"),
    }

    // dcl_input_ps (linear) v1, TEXCOORD (mask=xy=3)
    match &next("dcl_input_ps v1")?.kind {
        InstructionKind::DclInput { interpolation, operands, .. } => {
            let op0 = operands.first().ok_or_else(|| SpirvGenError::UnsupportedShader("Sprite PS: dcl_input_psにオペランドが無い".to_string()))?;
            if *interpolation != Some("linear")
                || op0.reg_type != RegisterType::Input
                || uav_index(&op0.indices) != Some(1)
                || op0.components != ComponentSelect::Mask(3)
            {
                return reject("4番目はdcl_input_ps(linear) v1・mask=3(xy)のはず");
            }
        }
        _ => return reject("4番目はdcl_inputのはず"),
    }

    // dcl_output o0, mask=xyzw=15
    match &next("dcl_output o0")?.kind {
        InstructionKind::DclOutput { operands, .. } => {
            let op0 = operands.first().ok_or_else(|| SpirvGenError::UnsupportedShader("Sprite PS: dcl_outputにオペランドが無い".to_string()))?;
            if op0.reg_type != RegisterType::Output || uav_index(&op0.indices) != Some(0) || op0.components != ComponentSelect::Mask(15) {
                return reject("5番目はdcl_output o0・mask=15のはず");
            }
        }
        _ => return reject("5番目はdcl_outputのはず"),
    }

    // sample o0.xyzw, v1.xy, t0.xyzw, s0
    match &next("sample o0, v1, t0, s0")?.kind {
        InstructionKind::Generic { operands } if operands.len() == 4 => {
            let dst = &operands[0];
            let coord = &operands[1];
            let resource = &operands[2];
            let sampler = &operands[3];
            if dst.reg_type != RegisterType::Output || uav_index(&dst.indices) != Some(0) {
                return reject("6番目のsampleの書き込み先がo0ではない");
            }
            if coord.reg_type != RegisterType::Input || uav_index(&coord.indices) != Some(1) {
                return reject("6番目のsampleの座標がv1ではない");
            }
            if resource.reg_type != RegisterType::Resource || uav_index(&resource.indices) != Some(0) {
                return reject("6番目のsampleのリソースがt0ではない");
            }
            if sampler.reg_type != RegisterType::Sampler || uav_index(&sampler.indices) != Some(0) {
                return reject("6番目のsampleのサンプラーがs0ではない");
            }
        }
        _ => return reject("6番目はsampleのはず"),
    }

    match &next("ret")?.kind {
        InstructionKind::Generic { operands } if operands.is_empty() => {}
        _ => return reject("7番目はretのはず"),
    }

    if it.next().is_some() {
        return reject("想定より命令が多い(7命令ちょうどのはず)");
    }

    Ok(())
}

/// DXBCバイト列(`sprite_vs.hlsl`相当のD3D11頂点シェーダー、SM5.0)を解析し、
/// 実際のSHEX命令列を検証しながらSPIR-Vへ翻訳する。一致しなければ
/// `SpirvGenError::UnsupportedShader`を返す。
pub fn translate_sprite_vertex_shader(bytes: &[u8]) -> Result<GraphicsTranslatedKernel, SpirvGenError> {
    let instructions = shex_instructions(bytes)?;
    decode_sprite_vertex_shader_shape(&instructions)?;
    Ok(GraphicsTranslatedKernel {
        spirv_words: emit_sprite_vertex_spirv(),
        entry_point: "main",
        stage: ShaderStage::Vertex,
    })
}

/// DXBCバイト列(`sprite_ps.hlsl`相当のD3D11ピクセルシェーダー、SM5.0)を
/// 解析し、実際のSHEX命令列を検証しながらSPIR-Vへ翻訳する。一致しなければ
/// `SpirvGenError::UnsupportedShader`を返す。テクスチャサンプリングに
/// 対応した初めてのシェーダー(2026-08-08)。
pub fn translate_sprite_pixel_shader(bytes: &[u8]) -> Result<GraphicsTranslatedKernel, SpirvGenError> {
    let instructions = shex_instructions(bytes)?;
    decode_sprite_pixel_shader_shape(&instructions)?;
    Ok(GraphicsTranslatedKernel {
        spirv_words: emit_sprite_pixel_spirv(),
        entry_point: "main",
        stage: ShaderStage::Fragment,
    })
}

/// `sprite_vs.hlsl`が検証を通った場合にのみ返す、固定のSPIR-Vモジュール。
/// `triangle_vs`用`emit_vertex_spirv`とほぼ同じだが、COLOR(vec4)の代わりに
/// TEXCOORD(vec2)をパススルーする。
fn emit_sprite_vertex_spirv() -> Vec<u32> {
    let mut b = Builder::new();
    b.set_version(1, 0);
    b.capability(spirv::Capability::Shader);
    b.memory_model(spirv::AddressingModel::Logical, spirv::MemoryModel::GLSL450);

    let void_ty = b.type_void();
    let voidf_ty = b.type_function(void_ty, vec![]);
    let float_ty = b.type_float(32, None);
    let vec2_ty = b.type_vector(float_ty, 2);
    let vec3_ty = b.type_vector(float_ty, 3);
    let vec4_ty = b.type_vector(float_ty, 4);

    let in_pos_ptr_ty = b.type_pointer(None, spirv::StorageClass::Input, vec3_ty);
    let var_in_pos = b.variable(in_pos_ptr_ty, None, spirv::StorageClass::Input, None);
    b.decorate(var_in_pos, spirv::Decoration::Location, vec![DrOperand::LiteralBit32(0)]);

    let in_uv_ptr_ty = b.type_pointer(None, spirv::StorageClass::Input, vec2_ty);
    let var_in_uv = b.variable(in_uv_ptr_ty, None, spirv::StorageClass::Input, None);
    b.decorate(var_in_uv, spirv::Decoration::Location, vec![DrOperand::LiteralBit32(1)]);

    let out_position_ptr_ty = b.type_pointer(None, spirv::StorageClass::Output, vec4_ty);
    let var_out_position = b.variable(out_position_ptr_ty, None, spirv::StorageClass::Output, None);
    b.decorate(var_out_position, spirv::Decoration::BuiltIn, vec![DrOperand::BuiltIn(spirv::BuiltIn::Position)]);

    let out_uv_ptr_ty = b.type_pointer(None, spirv::StorageClass::Output, vec2_ty);
    let var_out_uv = b.variable(out_uv_ptr_ty, None, spirv::StorageClass::Output, None);
    b.decorate(var_out_uv, spirv::Decoration::Location, vec![DrOperand::LiteralBit32(0)]);

    let main_fn = b.begin_function(void_ty, None, spirv::FunctionControl::NONE, voidf_ty).expect("OpFunction");
    b.begin_block(None).expect("OpLabel");

    let pos_in = b.load(vec3_ty, None, var_in_pos, None, vec![]).expect("OpLoad pos");
    let px = b.composite_extract(float_ty, None, pos_in, vec![0]).expect("extract x");
    let py = b.composite_extract(float_ty, None, pos_in, vec![1]).expect("extract y");
    let pz = b.composite_extract(float_ty, None, pos_in, vec![2]).expect("extract z");
    let one = b.constant_bit32(float_ty, 1.0f32.to_bits());
    let pos_out = b.composite_construct(vec4_ty, None, vec![px, py, pz, one]).expect("construct SV_POSITION");
    b.store(var_out_position, pos_out, None, vec![]).expect("OpStore SV_POSITION");

    let uv_in = b.load(vec2_ty, None, var_in_uv, None, vec![]).expect("OpLoad uv");
    b.store(var_out_uv, uv_in, None, vec![]).expect("OpStore TEXCOORD");

    b.ret().expect("OpReturn");
    b.end_function().expect("OpFunctionEnd");

    b.entry_point(
        spirv::ExecutionModel::Vertex,
        main_fn,
        "main",
        vec![var_in_pos, var_in_uv, var_out_position, var_out_uv],
    );

    let module = b.module();
    module.assemble()
}

/// `sprite_ps.hlsl`が検証を通った場合にのみ返す、固定のSPIR-Vモジュール。
/// `set=0, binding=0`のcombined image sampler(`OpTypeSampledImage`)から
/// `OpImageSampleImplicitLod`でサンプルし、そのままSV_TARGETへ出力する。
fn emit_sprite_pixel_spirv() -> Vec<u32> {
    let mut b = Builder::new();
    b.set_version(1, 0);
    b.capability(spirv::Capability::Shader);
    b.memory_model(spirv::AddressingModel::Logical, spirv::MemoryModel::GLSL450);

    let void_ty = b.type_void();
    let voidf_ty = b.type_function(void_ty, vec![]);
    let float_ty = b.type_float(32, None);
    let vec2_ty = b.type_vector(float_ty, 2);
    let vec4_ty = b.type_vector(float_ty, 4);

    let in_uv_ptr_ty = b.type_pointer(None, spirv::StorageClass::Input, vec2_ty);
    let var_in_uv = b.variable(in_uv_ptr_ty, None, spirv::StorageClass::Input, None);
    b.decorate(var_in_uv, spirv::Decoration::Location, vec![DrOperand::LiteralBit32(0)]);

    let out_color_ptr_ty = b.type_pointer(None, spirv::StorageClass::Output, vec4_ty);
    let var_out_color = b.variable(out_color_ptr_ty, None, spirv::StorageClass::Output, None);
    b.decorate(var_out_color, spirv::Decoration::Location, vec![DrOperand::LiteralBit32(0)]);

    // combined image sampler: `Texture2D SpriteTex : register(t0)` +
    // `SamplerState SpriteSampler : register(s0)`をHLSLコンパイラが暗黙に
    // 対で扱う慣行に合わせ、Vulkan側もset=0/binding=0の単一combined image
    // samplerとして表現する(D3D側のt0/s0という別レジスタ空間をVulkan側の
    // 単一デスクリプタへ畳み込む、DXVK等でも一般的なマッピング)。
    let image_ty = b.type_image(
        float_ty,
        spirv::Dim::Dim2D,
        0,
        0,
        0,
        1,
        spirv::ImageFormat::Unknown,
        None,
    );
    let sampled_image_ty = b.type_sampled_image(image_ty);
    let sampler_ptr_ty = b.type_pointer(None, spirv::StorageClass::UniformConstant, sampled_image_ty);
    let var_sampler = b.variable(sampler_ptr_ty, None, spirv::StorageClass::UniformConstant, None);
    b.decorate(var_sampler, spirv::Decoration::DescriptorSet, vec![DrOperand::LiteralBit32(0)]);
    b.decorate(var_sampler, spirv::Decoration::Binding, vec![DrOperand::LiteralBit32(0)]);

    let main_fn = b.begin_function(void_ty, None, spirv::FunctionControl::NONE, voidf_ty).expect("OpFunction");
    b.begin_block(None).expect("OpLabel");

    let uv = b.load(vec2_ty, None, var_in_uv, None, vec![]).expect("OpLoad uv");
    let sampled_image = b.load(sampled_image_ty, None, var_sampler, None, vec![]).expect("OpLoad sampledImage");
    let color = b
        .image_sample_implicit_lod(vec4_ty, None, sampled_image, uv, None, vec![])
        .expect("OpImageSampleImplicitLod");
    b.store(var_out_color, color, None, vec![]).expect("OpStore SV_TARGET");

    b.ret().expect("OpReturn");
    b.end_function().expect("OpFunctionEnd");

    b.entry_point(spirv::ExecutionModel::Fragment, main_fn, "main", vec![var_in_uv, var_out_color]);
    b.execution_mode(main_fn, spirv::ExecutionMode::OriginUpperLeft, []);

    let module = b.module();
    module.assemble()
}

// ---------------------------------------------------------------------
// GEMM(行列積)垂直スライス: `shaders/gemm2x2.hlsl`専用の翻訳経路。
// ---------------------------------------------------------------------
//
// **正直なスコープ**: 上記の`decode_shader_shape`/`decode_chain_shape`は
// いずれも「同じ添字(スレッドID)で読み書きする要素ごとの演算」専用の
// デコーダであり、GEMM(行列積、C[i][j]=Σ_k A[i][k]*B[k][j])が要求する
// 「読み込み添字自体をスレッドIDから算術演算(乗算・加算)で計算する」
// 形には対応していない。CLAUDE.mdのPhase 0が要求する「1つのシンプルな
// コンピュートシェーダーが実際にDXBC→SPIR-Vへ翻訳されVulkan実行される」
// という垂直スライスを、動的ループ(`loop`/`endloop`命令、本クレートの
// 既存デコーダ群がいずれも扱っていない制御フロー)を避けたまま実証する
// ため、**固定サイズ2x2×2x2=2x2のGEMMをK=2で完全アンロールした
// `shaders/gemm2x2.hlsl`(`numthreads(2,2,1)`、境界チェック不要——
// ディスパッチグリッドと出力サイズが厳密に一致するため)専用のデコーダ**
// として新規に追加した。一般のM×K×N GEMM(可変サイズ、ループを伴う)への
// 一般化は今回のスコープ外(次フェーズの課題として正直に残す)。
//
// 実際に`fxc.exe /T cs_5_0`でコンパイルし`examples/dump_shex`で確認した
// 実SHEX命令列(19命令、制御フロー無し)は次の通り:
// `dcl_globalFlags -> dcl_uav_structured(u0,u1,u2) -> dcl_input(vThreadID)
// -> dcl_temps(1) -> dcl_thread_group(2,2,1) -> imad -> ld_structured(u0)
// -> iadd -> ld_structured(u1) -> mul -> ld_structured(u1) -> ishl
// -> ld_structured(u0) -> iadd -> mad -> store_structured(u2) -> ret`
// (`C[i][j] = A[i][0]*B[0][j] + A[i][1]*B[1][j]`、`i=threadID.y`,
// `j=threadID.x`)。このデコーダは、レジスタ単位の式木を汎用的に評価する
// のではなく、**この既知の命令列と一致するかどうかだけを実際に検証する**
// (オペコード列・UAVバインドポイント・スレッドグループサイズを実際に
// パースした`SHEX`から読み取り、1つでも食い違えば
// `SpirvGenError::UnsupportedShader`を返す——「対応している」という
// 誤ったシグナルを出さない、というCLAUDE.md方針の継続)。

/// [`translate_gemm2x2_shader`]の結果。固定2x2×2x2GEMM専用のため、
/// 汎用[`TranslatedKernel`]と異なりM/K/Nパラメータは持たない
/// (常に2x2固定、`shape.uav_bind_points`のみ実DXBCから抽出する)。
#[derive(Debug, Clone)]
pub struct Gemm2x2TranslatedKernel {
    /// 生成されたSPIR-Vモジュール(リトルエンディアン32bitワード列)。
    pub spirv_words: Vec<u32>,
    /// `OpEntryPoint`のエントリポイント名(常に`"main"`)。
    pub entry_point: &'static str,
    /// `dcl_thread_group`から得た実際のスレッドグループサイズ(常に`(2,2,1)`
    /// になるはずだが、決め打ちにせず実DXBCから抽出した値をそのまま返す)。
    pub local_size: (u32, u32, u32),
    /// `dcl_uav_structured`から得た、A/B/C(この順)のUAVバインドポイント。
    pub uav_bind_points: (u32, u32, u32),
}

/// DXBCバイト列(`shaders/gemm2x2.hlsl`相当のD3D11 Compute Shader、
/// SM5.0)を解析し、実際のSHEX命令列が既知の固定2x2GEMM形状と一致するか
/// 検証しながらSPIR-Vへ翻訳する。一致しなければ
/// `SpirvGenError::UnsupportedShader`を返す(黙って的外れなSPIR-Vを
/// 生成しない)。
pub fn translate_gemm2x2_shader(bytes: &[u8]) -> Result<Gemm2x2TranslatedKernel, SpirvGenError> {
    let containers = scan_dxbc(bytes);
    let container = containers.into_iter().next().ok_or_else(|| {
        SpirvGenError::Translate(TranslateError::Parse("DXBCコンテナが見つからない".to_string()))
    })?;

    let mut instructions: Option<Vec<Instruction>> = None;
    for chunk in &container.chunks {
        if let ChunkData::Shader(program) = chunk.parse() {
            instructions = Some(program.instructions);
        }
    }
    let instructions = instructions.ok_or(SpirvGenError::Translate(TranslateError::MissingChunk("SHEX")))?;

    let (uav_a, uav_b, uav_c, thread_group) = decode_gemm2x2_shape(&instructions)?;
    let spirv_words = emit_gemm2x2_spirv(uav_a, uav_b, uav_c);

    Ok(Gemm2x2TranslatedKernel {
        spirv_words,
        entry_point: "main",
        local_size: thread_group,
        uav_bind_points: (uav_a, uav_b, uav_c),
    })
}

/// 実SHEX命令列が、`shaders/gemm2x2.hlsl`の既知の固定形状(上記コメント
/// 参照)と一致するかを検証する。オペコード列自体は実際に`fxc.exe`出力を
/// ダンプして確認したものと突き合わせる——推測でオペコードを並べたもの
/// ではない。戻り値は`(uav_a, uav_b, uav_c, thread_group)`。
type Gemm2x2Shape = (u32, u32, u32, (u32, u32, u32));

fn decode_gemm2x2_shape(instructions: &[Instruction]) -> Result<Gemm2x2Shape, SpirvGenError> {
    let mut uavs: Vec<u32> = Vec::new();
    let mut thread_group: Option<(u32, u32, u32)> = None;
    let mut opcodes: Vec<Opcode> = Vec::new();

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
                let bind = uav_index(&op.indices).ok_or_else(|| {
                    SpirvGenError::UnsupportedShader("dcl_uav_structuredのUAVバインドポイントを解決できない".to_string())
                })?;
                uavs.push(bind);
            }
            InstructionKind::DclInput { .. } => {}
            InstructionKind::DclTemps { .. } => {}
            InstructionKind::DclThreadGroup { x, y, z } => {
                thread_group = Some((*x, *y, *z));
            }
            InstructionKind::Generic { .. } => {
                opcodes.push(ins.opcode);
            }
            _ => {
                return Err(SpirvGenError::UnsupportedShader(format!(
                    "gemm2x2の想定外の宣言命令: {:?}",
                    ins.kind
                )));
            }
        }
    }

    if uavs.len() != 3 {
        return Err(SpirvGenError::UnsupportedShader(format!(
            "gemm2x2はUAV3本(A/B/C)を要求するが{}本だった",
            uavs.len()
        )));
    }
    let thread_group = thread_group
        .ok_or_else(|| SpirvGenError::UnsupportedShader("dcl_thread_groupが見つからない".to_string()))?;
    if thread_group != (2, 2, 1) {
        return Err(SpirvGenError::UnsupportedShader(format!(
            "gemm2x2はnumthreads(2,2,1)を要求するが{thread_group:?}だった"
        )));
    }

    // 実fxc.exe出力(examples/dump_shexで確認済み)と一致する固定オペコード列。
    const EXPECTED: &[Opcode] = &[
        Opcode::IMad,
        Opcode::LdStructured,
        Opcode::Iadd,
        Opcode::LdStructured,
        Opcode::Mul,
        Opcode::LdStructured,
        Opcode::Ishl,
        Opcode::LdStructured,
        Opcode::Iadd,
        Opcode::Mad,
        Opcode::StoreStructured,
        Opcode::Ret,
    ];
    if opcodes != EXPECTED {
        return Err(SpirvGenError::UnsupportedShader(format!(
            "gemm2x2の想定オペコード列と一致しない: got {opcodes:?}"
        )));
    }

    Ok((uavs[0], uavs[1], uavs[2], thread_group))
}

/// `decode_gemm2x2_shape`で検証済みのUAVバインドポイントから、固定2x2GEMM
/// (`C[i][j] = A[i][0]*B[0][j] + A[i][1]*B[1][j]`、`i=gl_GlobalInvocationID.y`,
/// `j=gl_GlobalInvocationID.x`)を実行するSPIR-Vを直接組み立てる
/// (DXBC命令列を1対1で逐次変換するのではなく、`decode_gemm2x2_shape`が
/// 実際に検証した固定形状に対して、数学的に同値なSPIR-Vを直接発行する
/// ——既存の`emit_spirv_impl`/`emit_chain_spirv_for_kernel`と同じ
/// 「検証済みの形に対して直接SPIR-Vを組む」設計方針)。
/// `opencuda-vulkan`の`"vector_add"`カーネル契約(3ストレージバッファ+
/// push constant `uint n`)に合わせるため、未使用のpush constantも
/// 宣言する(呼び出し側は`n=4`等ダミー値を渡せばよい、シェーダー側は
/// 参照しない)。
fn emit_gemm2x2_spirv(uav_a: u32, uav_b: u32, uav_c: u32) -> Vec<u32> {
    let mut b = Builder::new();
    b.set_version(1, 0);
    b.capability(spirv::Capability::Shader);
    b.memory_model(spirv::AddressingModel::Logical, spirv::MemoryModel::GLSL450);

    let void_ty = b.type_void();
    let voidf_ty = b.type_function(void_ty, vec![]);
    let float_ty = b.type_float(32, None);
    let uint_ty = b.type_int(32, 0);
    let uvec3_ty = b.type_vector(uint_ty, 3);

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
    let var_a = make_buffer_var(&mut b, uav_a);
    let var_b = make_buffer_var(&mut b, uav_b);
    let var_c = make_buffer_var(&mut b, uav_c);

    // push constant: struct Params { uint n; } (opencuda-vulkanの
    // "vector_add"カーネル契約に合わせるためだけの未使用フィールド)。
    let params_struct_ty = b.type_struct(vec![uint_ty]);
    b.decorate(params_struct_ty, spirv::Decoration::Block, vec![]);
    b.member_decorate(params_struct_ty, 0, spirv::Decoration::Offset, vec![DrOperand::LiteralBit32(0)]);
    let params_ptr_ty = b.type_pointer(None, spirv::StorageClass::PushConstant, params_struct_ty);
    let _var_params = b.variable(params_ptr_ty, None, spirv::StorageClass::PushConstant, None);

    let gid_ptr_ty = b.type_pointer(None, spirv::StorageClass::Input, uvec3_ty);
    let var_gid = b.variable(gid_ptr_ty, None, spirv::StorageClass::Input, None);
    b.decorate(var_gid, spirv::Decoration::BuiltIn, vec![DrOperand::BuiltIn(spirv::BuiltIn::GlobalInvocationId)]);

    let float_ptr_uniform_ty = b.type_pointer(None, spirv::StorageClass::Uniform, float_ty);

    let main_fn = b
        .begin_function(void_ty, None, spirv::FunctionControl::NONE, voidf_ty)
        .expect("OpFunction");
    b.begin_block(None).expect("OpLabel");

    let const_0 = b.constant_bit32(uint_ty, 0);
    let const_1 = b.constant_bit32(uint_ty, 1);
    let const_2 = b.constant_bit32(uint_ty, 2);

    let gid_vec = b.load(uvec3_ty, None, var_gid, None, vec![]).expect("OpLoad gid");
    let j = b.composite_extract(uint_ty, None, gid_vec, vec![0]).expect("j = gid.x");
    let i = b.composite_extract(uint_ty, None, gid_vec, vec![1]).expect("i = gid.y");

    // idx_a0 = i*2 + 0, idx_a1 = i*2 + 1
    let i2 = b.i_mul(uint_ty, None, i, const_2).expect("i*2");
    let idx_a1 = b.i_add(uint_ty, None, i2, const_1).expect("i*2+1");

    let load_f32 = |b: &mut Builder, var: u32, idx: u32| -> u32 {
        let ac = b.access_chain(float_ptr_uniform_ty, None, var, vec![const_0, idx]).expect("OpAccessChain");
        b.load(float_ty, None, ac, None, vec![]).expect("OpLoad")
    };

    let a0 = load_f32(&mut b, var_a, i2);
    let a1 = load_f32(&mut b, var_a, idx_a1);

    // idx_b0 = 0*2 + j = j, idx_b1 = 1*2 + j
    let idx_b1 = b.i_add(uint_ty, None, const_2, j).expect("2+j");
    let b0 = load_f32(&mut b, var_b, j);
    let b1 = load_f32(&mut b, var_b, idx_b1);

    let t0 = b.f_mul(float_ty, None, a0, b0).expect("a0*b0");
    let t1 = b.f_mul(float_ty, None, a1, b1).expect("a1*b1");
    let result = b.f_add(float_ty, None, t0, t1).expect("a0*b0 + a1*b1");

    // idx_c = i*2 + j
    let idx_c = b.i_add(uint_ty, None, i2, j).expect("i*2+j");
    let ac_c = b
        .access_chain(float_ptr_uniform_ty, None, var_c, vec![const_0, idx_c])
        .expect("OpAccessChain c[idx_c]");
    b.store(ac_c, result, None, vec![]).expect("OpStore c[idx_c]");

    b.ret().expect("OpReturn");
    b.end_function().expect("OpFunctionEnd");

    b.entry_point(spirv::ExecutionModel::GLCompute, main_fn, "main", vec![var_gid]);
    b.execution_mode(main_fn, spirv::ExecutionMode::LocalSize, [2, 2, 1]);

    let module = b.module();
    module.assemble()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `crates/directx-shader-translate/shaders/vector_add.dxbc`と同じ実
    /// fxc.exe出力(lib.rsのテストで使っているのと同一バイト列)。
    const VECTOR_ADD_DXBC: &[u8] = include_bytes!("../shaders/vector_add.dxbc");
    const GEMM2X2_DXBC: &[u8] = include_bytes!("../shaders/gemm2x2.dxbc");

    #[test]
    fn translates_real_fxc_compiled_gemm2x2_dxbc_to_valid_spirv() {
        let kernel = translate_gemm2x2_shader(GEMM2X2_DXBC)
            .expect("real fxc-compiled gemm2x2.dxbc (fixed 2x2 unrolled GEMM) must translate");

        assert_eq!(kernel.uav_bind_points, (0, 1, 2));
        assert_eq!(kernel.local_size, (2, 2, 1));
        assert_eq!(kernel.entry_point, "main");
        assert_eq!(kernel.spirv_words[0], 0x0723_0203);

        assert_valid_spirv(&kernel.spirv_words);
    }

    #[test]
    fn gemm2x2_translator_honestly_rejects_garbage_bytes() {
        let garbage = [0u8; 16];
        assert!(translate_gemm2x2_shader(&garbage).is_err(), "non-DXBC bytes must not translate successfully");
    }

    #[test]
    fn gemm2x2_translator_honestly_rejects_the_unrelated_vector_add_shader() {
        // gemm2x2専用デコーダは、たまたまUAV3本を持つだけの別形状
        // (vector_add: ld_structured x2 -> add -> store_structured、
        // ループ/算術添字なし)を誤って受理しないことを確認する。
        let result = translate_gemm2x2_shader(VECTOR_ADD_DXBC);
        assert!(
            result.is_err(),
            "vector_add's opcode sequence does not match gemm2x2's fixed shape, so translation must be honestly rejected"
        );
    }

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

    /// `shaders/vector_div.dxbc`(実fxc.exe出力、除算)。
    const VECTOR_DIV_DXBC: &[u8] = include_bytes!("../shaders/vector_div.dxbc");

    #[test]
    fn translates_real_fxc_compiled_vector_div_dxbc_to_valid_spirv() {
        let kernel = translate_shader(VECTOR_DIV_DXBC)
            .expect("real fxc-compiled vector_div.dxbc (div opcode) must translate");
        assert_eq!(kernel.uav_bind_points, (0, 1, 2));
        assert_eq!(kernel.local_size, (64, 1, 1));
        assert_eq!(kernel.spirv_words[0], 0x0723_0203);

        let bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let mut loader = rspirv::dr::Loader::new();
        rspirv::binary::parse_bytes(&bytes, &mut loader)
            .expect("emitted SPIR-V (div) must be well-formed and re-parseable");
    }

    /// `shaders/vector_add_mul_chain.dxbc`(実fxc.exe出力、UAV3本・
    /// `t = A[i]+B[i]; Out[i] = t*A[i];`という2項演算2回のチェーン、
    /// `A`を2回参照するがfxcがCSEで2回目のロードを省略する実バイト列)。
    /// 既存の`translate_shader`(単一演算・UAV3本固定)ではなく、新設した
    /// `translate_chain_shader`(N個の逐次2項演算パターンクラス)を使う。
    const VECTOR_ADD_MUL_CHAIN_DXBC: &[u8] = include_bytes!("../shaders/vector_add_mul_chain.dxbc");
    /// `shaders/vector_add_mul_chain_bounded.dxbc`(実fxc.exe出力、
    /// `t = A[i]+B[i]; if (i < N) { Out[i] = t*A[i]; }`——2026-08-06追加、
    /// 「境界チェック付きチェーン」パターン(既存のどのクラスにも当たら
    /// なかった組み合わせ)。
    const VECTOR_ADD_MUL_CHAIN_BOUNDED_DXBC: &[u8] = include_bytes!("../shaders/vector_add_mul_chain_bounded.dxbc");

    #[test]
    fn translates_real_fxc_compiled_vector_add_mul_chain_dxbc_to_valid_spirv() {
        let kernel = translate_chain_shader(VECTOR_ADD_MUL_CHAIN_DXBC)
            .expect("real fxc-compiled 2-op chain (add then mul, 3 UAVs, CSE'd reload) must translate");
        // 実際にDXBCから抽出した値であることの検証(決め打ちではない):
        // 式木は`(A+B)*A`なので、木を辿って集めた読み込み順は[A(u0), B(u1), A(u0)]
        // (fxcが2回目のA読み込みを`ld_structured`として出さずCSEで再利用したため、
        // ここでの「2回出現」は式木そのものの構造から来ている、決め打ちの重複ではない)。
        assert_eq!(kernel.read_uav_bind_points, vec![0, 1, 0]);
        assert_eq!(kernel.write_uav_bind_point, 2);
        assert_eq!(kernel.local_size, (64, 1, 1));
        assert_eq!(kernel.spirv_words[0], 0x0723_0203);

        let bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let mut loader = rspirv::dr::Loader::new();
        rspirv::binary::parse_bytes(&bytes, &mut loader)
            .expect("emitted SPIR-V (2-op chain) must be well-formed and re-parseable");
    }

    /// `shaders/vector_add_mul_div_chain3.dxbc`(実fxc.exe出力、UAV3本、
    /// `t1 = A[i]+B[i]; t2 = t1*A[i]; Out[i] = t2/B[i];`という2項演算
    /// **3回**のチェーン——2026-08-05増分、`decode_chain_shape`が2回の
    /// チェーンだけでなく実際に3回のチェーンも(コード変更無しで)正しく
    /// 扱えることを実バイト列で検証する。`examples/dump_shex`で実際に
    /// ダンプした結果、命令列は`ld_structured`x2 -> `add`(dest=temp.z) ->
    /// `mul`(dest=temp.x、temp.xを上書き) -> `div`(dest=temp.x、再度上書き)
    /// -> `store_structured`(temp.xを参照)という形で、`decode_chain_shape`
    /// が最初から持っていた「命令を1つずつ走査してreg_mapを更新する」という
    /// 一般的なロジックがそのまま(N=2専用のコードパスを分岐させることなく)
    /// N=3にも対応することを裏付けた。
    const VECTOR_ADD_MUL_DIV_CHAIN3_DXBC: &[u8] = include_bytes!("../shaders/vector_add_mul_div_chain3.dxbc");

    #[test]
    fn translates_real_fxc_compiled_3op_chain_dxbc_to_valid_spirv() {
        let kernel = translate_chain_shader(VECTOR_ADD_MUL_DIV_CHAIN3_DXBC)
            .expect("real fxc-compiled 3-op chain (add, mul, div; 3 UAVs) must translate");
        // 式木は`(A+B)*A/B`なので、読み込み順は[A(u0), B(u1), A(u0), B(u1)]。
        assert_eq!(kernel.read_uav_bind_points, vec![0, 1, 0, 1]);
        assert_eq!(kernel.write_uav_bind_point, 2);
        assert_eq!(kernel.local_size, (64, 1, 1));

        let bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let mut loader = rspirv::dr::Loader::new();
        rspirv::binary::parse_bytes(&bytes, &mut loader)
            .expect("emitted SPIR-V (3-op chain) must be well-formed and re-parseable");
    }

    #[test]
    fn chain_translator_honestly_rejects_garbage_bytes() {
        let garbage = [0u8; 16];
        assert!(translate_chain_shader(&garbage).is_err());
    }

    /// **既存4形状+境界チェック版に回帰が無いことの確認**: 新設した
    /// `translate_chain_shader`(N個の逐次2項演算)へ、既存の単一演算専用
    /// シェーダー(`vector_add.dxbc`)を渡すと、`decode_chain_shape`の
    /// 「2本以上のUAV(N入力+1出力)」等の要件自体は満たすものの、実際には
    /// 単一演算のみでチェーンを構成しないため——このシェーダー自体はチェーン
    /// クラスの要件(2項演算1回以上)も満たしてしまう。したがって`translate_shader`
    /// (既存)と`translate_chain_shader`(新設)は同じ入力を「両方とも」正しく
    /// 翻訳できてよい(排他的である必要はない、単に別のパターンクラスとして
    /// 共存するだけ)ことを確認する——既存側の挙動に手を入れていないことの
    /// 追加確認。
    #[test]
    fn chain_translator_also_accepts_the_pre_existing_single_op_vector_add_shader() {
        let kernel = translate_chain_shader(VECTOR_ADD_DXBC)
            .expect("a single add is a valid (trivial, N=1) instance of the chain pattern class too");
        assert_eq!(kernel.read_uav_bind_points, vec![0, 1]);
        assert_eq!(kernel.write_uav_bind_point, 2);
        assert!(!kernel.bounds_check, "vector_add.dxbcには境界チェックが無い");
    }

    #[test]
    /// 2026-08-06追加: 境界チェック(`dcl_constantbuffer`/`ult`/`if`/`endif`)
    /// 付きのチェーン(既存の`decode_chain_shape`のどのテストにも無かった
    /// 組み合わせ)を実際にfxc.exeでコンパイルしたバイト列から検証する。
    fn translates_real_fxc_compiled_bounded_chain_dxbc_to_valid_spirv_with_bounds_check_flag_set() {
        let kernel = translate_chain_shader(VECTOR_ADD_MUL_CHAIN_BOUNDED_DXBC)
            .expect("real fxc-compiled bounded 2-op chain (add then mul, cbuffer+ult+if+endif) must translate");
        assert_eq!(kernel.read_uav_bind_points, vec![0, 1, 0]);
        assert_eq!(kernel.write_uav_bind_point, 2);
        assert_eq!(kernel.local_size, (64, 1, 1));
        assert!(kernel.bounds_check, "cbuffer+ult+if+endifが実際に揃っているシェーダーなのでtrueのはず");

        let bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let mut loader = rspirv::dr::Loader::new();
        rspirv::binary::parse_bytes(&bytes, &mut loader)
            .expect("emitted SPIR-V (bounded chain) must be well-formed and re-parseable");
    }

    /// `shaders/triangle_vs.dxbc`/`shaders/triangle_ps.dxbc`(実fxc.exe出力、
    /// `vs_5_0`/`ps_5_0`)。D3D11グラフィックスパイプライン向けSPIR-V生成の
    /// 実データ。
    const TRIANGLE_VS_DXBC: &[u8] = include_bytes!("../shaders/triangle_vs.dxbc");
    const TRIANGLE_PS_DXBC: &[u8] = include_bytes!("../shaders/triangle_ps.dxbc");

    fn assert_valid_spirv(words: &[u32]) -> rspirv::dr::Module {
        assert_eq!(words[0], 0x0723_0203, "SPIR-Vマジックナンバーが先頭に無い");
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let mut loader = rspirv::dr::Loader::new();
        rspirv::binary::parse_bytes(&bytes, &mut loader)
            .expect("emitted SPIR-V must be well-formed and re-parseable by rspirv's own loader");
        loader.module()
    }

    #[test]
    fn translates_real_fxc_compiled_triangle_vs_dxbc_to_valid_vertex_spirv() {
        let kernel = translate_vertex_shader(TRIANGLE_VS_DXBC)
            .expect("real fxc-compiled triangle_vs.dxbc (dcl_input x2/dcl_output_siv/dcl_output/mov x3/ret) must translate");
        assert_eq!(kernel.entry_point, "main");
        assert_eq!(kernel.stage, ShaderStage::Vertex);
        let module = assert_valid_spirv(&kernel.spirv_words);
        assert!(
            module.entry_points.iter().any(|ep| ep.operands.iter().any(
                |op| matches!(op, rspirv::dr::Operand::ExecutionModel(rspirv::spirv::ExecutionModel::Vertex))
            )),
            "re-parsed module must declare OpEntryPoint Vertex"
        );
    }

    #[test]
    fn translates_real_fxc_compiled_triangle_ps_dxbc_to_valid_fragment_spirv() {
        let kernel = translate_pixel_shader(TRIANGLE_PS_DXBC)
            .expect("real fxc-compiled triangle_ps.dxbc (dcl_input_ps/dcl_output/mov/ret) must translate");
        assert_eq!(kernel.entry_point, "main");
        assert_eq!(kernel.stage, ShaderStage::Fragment);
        let module = assert_valid_spirv(&kernel.spirv_words);
        assert!(
            module.entry_points.iter().any(|ep| ep.operands.iter().any(
                |op| matches!(op, rspirv::dr::Operand::ExecutionModel(rspirv::spirv::ExecutionModel::Fragment))
            )),
            "re-parsed module must declare OpEntryPoint Fragment"
        );
    }

    #[test]
    fn vertex_translator_honestly_rejects_the_pixel_shader_and_vice_versa() {
        // VS用の検証ロジックにPSのDXBCを渡す(逆も同様)と、命令列の形状が
        // 一致しないため正しく拒否されることを確認する(「対応している」
        // という誤ったシグナルを出さないことの継続的な保証)。
        assert!(translate_vertex_shader(TRIANGLE_PS_DXBC).is_err());
        assert!(translate_pixel_shader(TRIANGLE_VS_DXBC).is_err());
    }

    #[test]
    fn graphics_translators_honestly_reject_garbage_bytes() {
        let garbage = [0u8; 16];
        assert!(translate_vertex_shader(&garbage).is_err());
        assert!(translate_pixel_shader(&garbage).is_err());
    }

    /// 既存のCompute Shader専用デコーダ(`translate_shader`/`translate_chain_shader`)
    /// にVS/PSのDXBCを渡すと、引き続き`SpirvGenError::UnsupportedShader`で
    /// 正しく拒否されることの回帰確認(今回の追加が既存の「誤ったシグナルを
    /// 出さない」保証を壊していないことの確認)。
    #[test]
    fn compute_translators_still_honestly_reject_graphics_shaders() {
        assert!(translate_shader(TRIANGLE_VS_DXBC).is_err());
        assert!(translate_chain_shader(TRIANGLE_VS_DXBC).is_err());
        assert!(translate_shader(TRIANGLE_PS_DXBC).is_err());
        assert!(translate_chain_shader(TRIANGLE_PS_DXBC).is_err());
    }

    const SPRITE_VS_DXBC: &[u8] = include_bytes!("../shaders/sprite_vs.dxbc");
    const SPRITE_PS_DXBC: &[u8] = include_bytes!("../shaders/sprite_ps.dxbc");

    #[test]
    fn translates_real_fxc_compiled_sprite_vs_dxbc_to_valid_vertex_spirv() {
        let kernel = translate_sprite_vertex_shader(SPRITE_VS_DXBC)
            .expect("real fxc-compiled sprite_vs.dxbc (POSITION/TEXCOORD passthrough) must translate to SPIR-V");
        assert_eq!(kernel.stage, ShaderStage::Vertex);
        assert_eq!(kernel.spirv_words[0], 0x0723_0203);

        let bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let module = rspirv::dr::load_bytes(&bytes).expect("re-parse generated sprite VS SPIR-V");
        assert!(
            module.entry_points.iter().any(|ep| matches!(
                ep.operands.first(),
                Some(DrOperand::ExecutionModel(spirv::ExecutionModel::Vertex))
            )),
            "re-parsed module must declare OpEntryPoint Vertex"
        );
    }

    #[test]
    fn translates_real_fxc_compiled_sprite_ps_dxbc_to_valid_fragment_spirv_with_texture_sampling() {
        let kernel = translate_sprite_pixel_shader(SPRITE_PS_DXBC).expect(
            "real fxc-compiled sprite_ps.dxbc (Texture2D.Sample) must translate to SPIR-V",
        );
        assert_eq!(kernel.stage, ShaderStage::Fragment);
        assert_eq!(kernel.spirv_words[0], 0x0723_0203);

        let bytes: Vec<u8> = kernel.spirv_words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let module = rspirv::dr::load_bytes(&bytes).expect("re-parse generated sprite PS SPIR-V");
        assert!(
            module.entry_points.iter().any(|ep| matches!(
                ep.operands.first(),
                Some(DrOperand::ExecutionModel(spirv::ExecutionModel::Fragment))
            )),
            "re-parsed module must declare OpEntryPoint Fragment"
        );
    }

    #[test]
    fn sprite_translators_honestly_reject_the_non_sprite_triangle_shaders_and_vice_versa() {
        // 既存のtriangle_vs/ps用デコーダとは独立した別パターンクラスである
        // ことの確認(「対応している」という誤ったシグナルを出さない、
        // 既存方針の継続)。
        assert!(translate_sprite_vertex_shader(TRIANGLE_VS_DXBC).is_err());
        assert!(translate_sprite_pixel_shader(TRIANGLE_PS_DXBC).is_err());
        assert!(translate_vertex_shader(SPRITE_VS_DXBC).is_err());
        assert!(translate_pixel_shader(SPRITE_PS_DXBC).is_err());
    }
}
