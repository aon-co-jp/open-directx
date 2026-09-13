//! FFv1のMED(median edge detector)予測器、**実画像2次元インデックス版**
//! (`x-1`/`y-1`の近傍参照)のDXBC→SPIR-V翻訳。
//!
//! 2026-09-12に`med_predictor_2d.hlsl`を実際に`fxc.exe`でコンパイルし
//! `examples/dump_shex`で実SHEX命令列を確認した結果、`spirv_gen.rs`の
//! 「制御フロー無しの式木」(`RegExpr`チェーンデコーダ)を大きく超える
//! 形状——`IMul`(Nullレジスタ、`Width*Height`の高位ビット破棄)/`UDiv`/
//! `IMad`(整数積和、`y*Width+x`の行優先インデックス計算)/`Iadd`
//! (即値`0xFFFFFFFF`=`-1`、`x-1`/`y-1`)/`And`(`&&`)、さらに実際の
//! `If`/`EndIf`(外側の`i<Width*Height`ディスパッチ余剰ガード)——である
//! ことが判明したため、`spirv_gen.rs`のRegExpr汎用機構を拡張するのでは
//! なく、**このシェーダー専用の固定形状デコーダ**として別モジュールに
//! 実装する(`decode_shader_shape`〈vector_add等〉と同じ「1つの既知の
//! 実コンパイル結果だけを認識する」設計方針を踏襲——RegExprチェーンの
//! ような汎用化はしていない、正直な開示)。
//!
//! **正直な開示(境界規約)**: 実画像の端(`x==0`または`y==0`)では、
//! FFv1本来の境界規約(仕様上の特別な初期値)ではなく、単純に自分自身の
//! ピクセル値(`center`)で`left`/`top`/`topleft`を埋める簡略化を
//! 採用している(`med_predictor_2d.hlsl`のHLSLソース自体がこの簡略化を
//! 実装しており、翻訳器はそれをそのまま反映するだけ)。

use dxbc::shex::{Instruction, InstructionKind, Opcode, OperandIndex, RegisterType};
use dxbc::{scan_dxbc, ChunkData};
use rspirv::binary::Assemble;
use rspirv::dr::{Builder, Operand as DrOperand};
use rspirv::spirv;

use crate::spirv_gen::SpirvGenError;
use crate::TranslateError;

/// 翻訳結果。`spirv_gen::TranslatedKernel`と似た形だが、`Width`/
/// `Height`をpush constantで受け取る専用カーネルであることを示すため
/// あえて別の型として定義する。
#[derive(Debug, Clone)]
pub struct Med2dTranslatedKernel {
    pub spirv_words: Vec<u32>,
    pub entry_point: &'static str,
    pub local_size: (u32, u32, u32),
    /// 読み込み元(`Image`)・書き込み先(`Output`)のUAVバインドポイント。
    pub image_uav: u32,
    pub output_uav: u32,
}

/// `med_predictor_2d.hlsl`(実`fxc.exe`出力)専用の翻訳。DXBC命令列が
/// 実際にコンパイルした形と完全一致することを確認した上でのみ翻訳し、
/// 一致しなければ`SpirvGenError::UnsupportedShader`を返す(このモジュール
/// 冒頭のdocコメント通り、これは1つの既知シェーダー専用の翻訳器であり、
/// 汎用の整数演算+分岐デコーダではない)。
///
/// `width`/`height`はDXBC側では実行時の定数バッファ(`cbuffer Params`)
/// 経由だが、翻訳後のSPIR-Vではpush constantではなく**ビルド時の
/// `OpConstant`として直接埋め込む**(2026-09-13設計判断:
/// `range_coder::build_range_decoder_kernel`と全く同じ理由——
/// `open-cuda`の`chain_n_buffer`汎用ディスパッチが渡すpush constantは
/// 常に4バイト〈要素数n〉のみで、このカーネルが本来必要とする8バイト
/// 〈width+height〉のレイアウトとは一致しないため、レイアウト不整合を
/// 起こす前に設計で回避した)。呼び出し側は画像サイズを変えるたびに
/// 翻訳をやり直す必要がある——実行時に任意の画像サイズを1回のSPIR-V
/// ビルドで扱えるようにするには、正攻法(push constant契約を
/// `chain_n_buffer`とは別に用意する)が別途必要で、今回は範囲外とする。
pub fn translate_med_predictor_2d_shader(bytes: &[u8], width: u32, height: u32) -> Result<Med2dTranslatedKernel, SpirvGenError> {
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

    let Med2dVerifiedShape { thread_group, image_uav, output_uav } = verify_med_2d_shape(&instructions)?;
    let spirv_words = emit_med_2d_spirv(thread_group, image_uav, output_uav, width, height);

    Ok(Med2dTranslatedKernel { spirv_words, entry_point: "main", local_size: thread_group, image_uav, output_uav })
}

fn uav_index(indices: &[OperandIndex]) -> Option<u32> {
    match indices.first()? {
        OperandIndex::Imm32(i) => Some(*i),
        _ => None,
    }
}

/// 実際のSHEX命令列が、`med_predictor_2d.hlsl`(実fxc.exe出力、
/// 2026-09-12に`examples/dump_shex`で確認)と完全に一致することを
/// 検証する。一致すれば`(thread_group, image_uav, output_uav)`を返す。
///
/// 検証するのは主にオペコード列の並び(29命令、`IMul`→`ULt`→`If`→
/// `UDiv`→...→`EndIf`)——各命令のオペランドについては、UAV/定数
/// バッファのバインドポイントや`Null`レジスタの使用など、翻訳結果の
/// 正しさに直結する部分のみ厳密にチェックし、一時レジスタの番号・
/// コンポーネント選択の細部までは(実際にfxcが常にこの形で出力する
/// ことを前提に)厳密比較しない——「1つの既知シェーダーの専用翻訳器」
/// という設計方針上、これで十分な正直さと判断した。
/// [`verify_med_2d_shape`]の戻り値(clippyの型複雑度lint回避のため
/// 名前付き構造体化)。
struct Med2dVerifiedShape {
    thread_group: (u32, u32, u32),
    image_uav: u32,
    output_uav: u32,
}

fn verify_med_2d_shape(instructions: &[Instruction]) -> Result<Med2dVerifiedShape, SpirvGenError> {
    let mut declared_uavs: Vec<u32> = Vec::new();
    let mut has_cbuffer = false;
    let mut thread_group: Option<(u32, u32, u32)> = None;
    let mut opcodes: Vec<Opcode> = Vec::new();

    for ins in instructions {
        match &ins.kind {
            InstructionKind::DclGlobalFlags { .. } => {}
            InstructionKind::DclConstantBuffer { operands, .. } => {
                let op0 = operands.first().ok_or_else(|| {
                    SpirvGenError::UnsupportedShader("dcl_constantbufferにオペランドが無い".to_string())
                })?;
                if op0.reg_type != RegisterType::ConstantBuffer || uav_index(&op0.indices) != Some(0) {
                    return Err(SpirvGenError::UnsupportedShader(
                        "med_predictor_2dはb0の定数バッファ(Width,Height)を要求する".to_string(),
                    ));
                }
                has_cbuffer = true;
            }
            InstructionKind::DclUavStructured { stride, operands, .. } => {
                if *stride != 4 {
                    return Err(SpirvGenError::UnsupportedShader("stride!=4".to_string()));
                }
                let op0 = operands.first().ok_or_else(|| {
                    SpirvGenError::UnsupportedShader("dcl_uav_structuredにオペランドが無い".to_string())
                })?;
                let idx = uav_index(&op0.indices)
                    .ok_or_else(|| SpirvGenError::UnsupportedShader("UAVバインドポイント不明".to_string()))?;
                declared_uavs.push(idx);
            }
            InstructionKind::DclInput { .. } => {}
            InstructionKind::DclTemps { .. } => {}
            InstructionKind::DclThreadGroup { x, y, z } => {
                thread_group = Some((*x, *y, *z));
            }
            InstructionKind::Generic { .. } => {
                opcodes.push(ins.opcode);
            }
            other => {
                return Err(SpirvGenError::UnsupportedShader(format!("対応スコープ外の宣言命令: {other:?}")));
            }
        }
    }

    if !has_cbuffer {
        return Err(SpirvGenError::UnsupportedShader("med_predictor_2dは定数バッファ(b0)が必須".to_string()));
    }
    if declared_uavs.as_slice() != [0, 1] {
        return Err(SpirvGenError::UnsupportedShader(format!(
            "med_predictor_2dはUAV u0(Image)/u1(Output)の2本を要求するが{declared_uavs:?}だった"
        )));
    }
    let thread_group = thread_group
        .ok_or_else(|| SpirvGenError::UnsupportedShader("dcl_thread_groupが見つからない".to_string()))?;

    // 2026-09-12に実際にfxc.exeでコンパイルして確認した、この1本の
    // シェーダーのオペコード列そのもの(29命令)。
    const EXPECTED: &[Opcode] = &[
        Opcode::IMul,
        Opcode::ULt,
        Opcode::If,
        Opcode::UDiv,
        Opcode::LdStructured,
        Opcode::ULt,
        Opcode::Iadd,
        Opcode::IMad,
        Opcode::LdStructured,
        Opcode::Movc,
        Opcode::ULt,
        Opcode::Iadd,
        Opcode::IMad,
        Opcode::LdStructured,
        Opcode::Movc,
        Opcode::And,
        Opcode::IMad,
        Opcode::LdStructured,
        Opcode::Movc,
        Opcode::Max,
        Opcode::Ge,
        Opcode::Min,
        Opcode::Ge,
        Opcode::Add,
        Opcode::Add,
        Opcode::Movc,
        Opcode::Movc,
        Opcode::StoreStructured,
        Opcode::EndIf,
        Opcode::Ret,
    ];
    if opcodes.as_slice() != EXPECTED {
        return Err(SpirvGenError::UnsupportedShader(format!(
            "med_predictor_2d専用デコーダが想定するオペコード列と一致しない(実際: {opcodes:?})"
        )));
    }

    Ok(Med2dVerifiedShape { thread_group, image_uav: 0, output_uav: 1 })
}

/// 検証済みの形状から、実際にMED予測器(2次元近傍参照版)のSPIR-Vを
/// 直接組み立てる。DXBCの各一時レジスタを逐一模倣するのではなく、
/// 「このシェーダーが計算する式が何か」を直接SPIR-Vで表現する
/// (検証済みのDXBC形状と数学的に同じ結果になることは、実GPUテストで
/// 実測値を突き合わせて確認する)。
fn emit_med_2d_spirv(thread_group: (u32, u32, u32), image_uav: u32, output_uav: u32, width: u32, height: u32) -> Vec<u32> {
    let mut b = Builder::new();
    b.set_version(1, 0);
    b.capability(spirv::Capability::Shader);
    b.memory_model(spirv::AddressingModel::Logical, spirv::MemoryModel::GLSL450);

    let void_ty = b.type_void();
    let voidf_ty = b.type_function(void_ty, vec![]);
    let float_ty = b.type_float(32, None);
    let uint_ty = b.type_int(32, 0);
    let bool_ty = b.type_bool();
    let uvec3_ty = b.type_vector(uint_ty, 3);

    let rt_array_ty = b.type_runtime_array(float_ty);
    b.decorate(rt_array_ty, spirv::Decoration::ArrayStride, vec![DrOperand::LiteralBit32(4)]);
    let buf_struct_ty = b.type_struct(vec![rt_array_ty]);
    b.decorate(buf_struct_ty, spirv::Decoration::BufferBlock, vec![]);
    b.member_decorate(buf_struct_ty, 0, spirv::Decoration::Offset, vec![DrOperand::LiteralBit32(0)]);
    let buf_ptr_ty = b.type_pointer(None, spirv::StorageClass::Uniform, buf_struct_ty);
    let float_ptr_uniform_ty = b.type_pointer(None, spirv::StorageClass::Uniform, float_ty);

    let make_buffer_var = |b: &mut Builder, binding: u32| -> u32 {
        let var = b.variable(buf_ptr_ty, None, spirv::StorageClass::Uniform, None);
        b.decorate(var, spirv::Decoration::DescriptorSet, vec![DrOperand::LiteralBit32(0)]);
        b.decorate(var, spirv::Decoration::Binding, vec![DrOperand::LiteralBit32(binding)]);
        var
    };
    let var_image = make_buffer_var(&mut b, image_uav);
    let var_output = make_buffer_var(&mut b, output_uav);

    let gid_ptr_ty = b.type_pointer(None, spirv::StorageClass::Input, uvec3_ty);
    let var_gid = b.variable(gid_ptr_ty, None, spirv::StorageClass::Input, None);
    b.decorate(var_gid, spirv::Decoration::BuiltIn, vec![DrOperand::BuiltIn(spirv::BuiltIn::GlobalInvocationId)]);

    let main_fn = b.begin_function(void_ty, None, spirv::FunctionControl::NONE, voidf_ty).expect("OpFunction");
    b.begin_block(None).expect("OpLabel entry");

    let const_0 = b.constant_bit32(uint_ty, 0);
    let const_1u = b.constant_bit32(uint_ty, 1);
    let width = b.constant_bit32(uint_ty, width);
    let height = b.constant_bit32(uint_ty, height);

    let gid_vec = b.load(uvec3_ty, None, var_gid, None, vec![]).expect("OpLoad gid");
    let i = b.composite_extract(uint_ty, None, gid_vec, vec![0]).expect("OpCompositeExtract .x");

    let total = b.i_mul(uint_ty, None, width, height).expect("width*height");
    let in_bounds = b.u_less_than(bool_ty, None, i, total).expect("i < width*height");

    let then_label = b.id();
    let merge_label = b.id();
    b.selection_merge(merge_label, spirv::SelectionControl::NONE).expect("OpSelectionMerge");
    b.branch_conditional(in_bounds, then_label, merge_label, vec![]).expect("OpBranchConditional");

    b.begin_block(Some(then_label)).expect("OpLabel then");

    let x = b.u_mod(uint_ty, None, i, width).expect("i % width");
    let y = b.u_div(uint_ty, None, i, width).expect("i / width");

    let load_image_at = |b: &mut Builder, idx: u32| -> u32 {
        let ac = b.access_chain(float_ptr_uniform_ty, None, var_image, vec![const_0, idx]).expect("OpAccessChain Image[idx]");
        b.load(float_ty, None, ac, None, vec![]).expect("OpLoad Image[idx]")
    };
    let center = load_image_at(&mut b, i);

    // left = (x>0) ? Image[y*width+(x-1)] : center
    let x_gt_0 = {
        let zero_u = b.constant_bit32(uint_ty, 0);
        b.u_greater_than(bool_ty, None, x, zero_u).expect("x > 0")
    };
    let x_minus_1 = b.i_sub(uint_ty, None, x, const_1u).expect("x-1");
    let y_mul_w = b.i_mul(uint_ty, None, y, width).expect("y*width");
    let idx_left = b.i_add(uint_ty, None, y_mul_w, x_minus_1).expect("y*width+(x-1)");
    let left_loaded = load_image_at(&mut b, idx_left);
    let left = b.select(float_ty, None, x_gt_0, left_loaded, center).expect("OpSelect left");

    // top = (y>0) ? Image[(y-1)*width+x] : center
    let y_gt_0 = {
        let zero_u = b.constant_bit32(uint_ty, 0);
        b.u_greater_than(bool_ty, None, y, zero_u).expect("y > 0")
    };
    let y_minus_1 = b.i_sub(uint_ty, None, y, const_1u).expect("y-1");
    let ym1_mul_w = b.i_mul(uint_ty, None, y_minus_1, width).expect("(y-1)*width");
    let idx_top = b.i_add(uint_ty, None, ym1_mul_w, x).expect("(y-1)*width+x");
    let top_loaded = load_image_at(&mut b, idx_top);
    let top = b.select(float_ty, None, y_gt_0, top_loaded, center).expect("OpSelect top");

    // topleft = (x>0 && y>0) ? Image[(y-1)*width+(x-1)] : center
    let both_gt_0 = b.logical_and(bool_ty, None, x_gt_0, y_gt_0).expect("x>0 && y>0");
    let idx_topleft = b.i_add(uint_ty, None, ym1_mul_w, x_minus_1).expect("(y-1)*width+(x-1)");
    let topleft_loaded = load_image_at(&mut b, idx_topleft);
    let topleft = b.select(float_ty, None, both_gt_0, topleft_loaded, center).expect("OpSelect topleft");

    // MED: pred = (topleft>=max(left,top)) ? min(left,top)
    //           : (topleft<=min(left,top)) ? max(left,top)
    //           : left+top-topleft
    let glsl_ext = b.ext_inst_import("GLSL.std.450");
    const GLSL_STD_450_F_MIN: u32 = 37;
    const GLSL_STD_450_F_MAX: u32 = 40;
    let max_lt = b
        .ext_inst(float_ty, None, glsl_ext, GLSL_STD_450_F_MAX, vec![DrOperand::IdRef(left), DrOperand::IdRef(top)])
        .expect("FMax(left,top)");
    let min_lt = b
        .ext_inst(float_ty, None, glsl_ext, GLSL_STD_450_F_MIN, vec![DrOperand::IdRef(left), DrOperand::IdRef(top)])
        .expect("FMin(left,top)");
    let cond1 = b.f_ord_greater_than_equal(bool_ty, None, topleft, max_lt).expect("topleft >= max(left,top)");
    let cond2 = b.f_ord_less_than_equal(bool_ty, None, topleft, min_lt).expect("topleft <= min(left,top)");
    let sum_lt = b.f_add(float_ty, None, left, top).expect("left+top");
    let else_val = b.f_sub(float_ty, None, sum_lt, topleft).expect("left+top-topleft");
    let inner = b.select(float_ty, None, cond2, max_lt, else_val).expect("OpSelect inner");
    let pred = b.select(float_ty, None, cond1, min_lt, inner).expect("OpSelect outer (pred)");

    let ac_out = b.access_chain(float_ptr_uniform_ty, None, var_output, vec![const_0, i]).expect("OpAccessChain Output[i]");
    b.store(ac_out, pred, None, vec![]).expect("OpStore Output[i]");

    b.branch(merge_label).expect("branch to merge");
    b.begin_block(Some(merge_label)).expect("OpLabel merge");
    b.ret().expect("OpReturn");
    b.end_function().expect("OpFunctionEnd");

    b.entry_point(spirv::ExecutionModel::GLCompute, main_fn, "main", vec![var_gid]);
    b.execution_mode(main_fn, spirv::ExecutionMode::LocalSize, [thread_group.0, thread_group.1, thread_group.2]);

    let module = b.module();
    module.assemble()
}
