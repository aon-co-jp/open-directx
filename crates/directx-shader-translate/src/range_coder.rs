//! FFv1のレンジコーダー(range coder)、CPU参照実装+状態遷移テーブル。
//!
//! **正直な開示(スコープ)**: これはFFv1本体のビットストリーム形式
//! (`put_symbol`/`get_symbol`のコンテキストインデックス選択、実際の
//! ピクセル差分値のシンボル化)を実装するものではない。RFC 9043
//! (`https://www.rfc-editor.org/rfc/rfc9043.txt`、Section 3.8.1.5の
//! `default_state_transition`テーブルと`get_rac`関数)に定義された、
//! **1シンボル(1ビット)を読む`get_rac`本体+その状態遷移テーブル**を、
//! CPU参照実装として、かつ`spirv_gen::build_range_decoder_kernel`で
//! 実GPU上の1invocationによる逐次ループとして、それぞれ実装する。
//! これはFFmpeg本家の実装が32レーンsubgroupで並列化している「32個の
//! コンテキストのlookup/adapt」の**並列化そのものではなく**、まず
//! その土台となる「状態遷移テーブル駆動の適応ロジック本体が正しく
//! GPU上で動くこと」を検証する段階——32レーン並列化は、この後さらに
//! 「同時に処理できる32個の独立したコンテキスト」という構造を導入する
//! 追加のステップとして残る。

/// RFC 9043 Section 3.8.1.5の`default_state_transition`テーブル
/// (256要素)を実際にRFC本文から書き写した値。`state_transition_delta`
/// が存在しない場合(このプロトタイプでは常に存在しない前提)、
/// `one_state[i] = default_state_transition[i]`となる。
#[rustfmt::skip]
pub const DEFAULT_STATE_TRANSITION: [u8; 256] = [
      0,  0,  0,  0,  0,  0,  0,  0, 20, 21, 22, 23, 24, 25, 26, 27,
     28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 37, 38, 39, 40, 41, 42,
     43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 56, 57,
     58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73,
     74, 75, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88,
     89, 90, 91, 92, 93, 94, 94, 95, 96, 97, 98, 99,100,101,102,103,
    104,105,106,107,108,109,110,111,112,113,114,114,115,116,117,118,
    119,120,121,122,123,124,125,126,127,128,129,130,131,132,133,133,
    134,135,136,137,138,139,140,141,142,143,144,145,146,147,148,149,
    150,151,152,152,153,154,155,156,157,158,159,160,161,162,163,164,
    165,166,167,168,169,170,171,171,172,173,174,175,176,177,178,179,
    180,181,182,183,184,185,186,187,188,189,190,190,191,192,194,194,
    195,196,197,198,199,200,201,202,202,204,205,206,207,208,209,209,
    210,211,212,213,215,215,216,217,218,219,220,220,222,223,224,225,
    226,227,227,229,229,230,231,232,234,234,235,236,237,238,239,240,
    241,242,243,244,245,246,247,248,248,  0,  0,  0,  0,  0,  0,  0,
];

/// `one_state[i] = default_state_transition[i] + state_transition_delta[i]`
/// (このプロトタイプでは`state_transition_delta`は常に0、RFC既定の
/// 「カスタム状態遷移テーブルがビットストリームに存在しない」場合)。
pub fn one_state() -> [u8; 256] {
    DEFAULT_STATE_TRANSITION
}

/// `zero_state[i] = 256 - one_state[256-i]`(RFC 9043記載の式)。
/// `i=0`は`256-i=256`が配列範囲外になるため、実装上の慣例
/// (FFmpeg本家 `libavcodec/rangecoder.h`の`ff_build_rac_states`)に
/// 倣い`zero_state[0] = 0`とする。
pub fn zero_state() -> [u8; 256] {
    let one = one_state();
    let mut zero = [0u8; 256];
    for (i, slot) in zero.iter_mut().enumerate().skip(1) {
        let idx = 256 - i;
        *slot = (256u16 - one[idx] as u16) as u8;
    }
    zero
}

/// `get_rac`が読み進める`RangeCoder`の状態(RFC 9043 Section 3.8.1.1
/// の初期化: `low = 先頭2バイトをビッグエンディアンでu16化`、
/// `range = 0xFF00`)。
pub struct RangeDecoderCpu<'a> {
    bytes: &'a [u8],
    pos: usize,
    low: u32,
    range: u32,
}

impl<'a> RangeDecoderCpu<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        let low = ((bytes[0] as u32) << 8) | (bytes[1] as u32);
        Self { bytes, pos: 2, low, range: 0xFF00 }
    }

    fn refill(&mut self) {
        if self.range < 256 {
            self.range *= 256;
            self.low *= 256;
            if self.pos < self.bytes.len() {
                self.low += self.bytes[self.pos] as u32;
                self.pos += 1;
            }
            // 正直な開示: RFC本文の`refill`はバイト列が尽きた場合
            // `end`フラグを立てて以後0を足す(実装は簡略化のため
            // バイトが尽きたら単に加算しない=0を足したのと同じ扱い)。
        }
    }

    /// RFC 9043 Section 3.8.1.1の`get_rac`をそのまま実装する。
    /// `state`は呼び出し側が保持するコンテキストの現在状態
    /// (呼び出しごとに更新される)。戻り値は復号された1ビット(0か1)。
    pub fn get_rac(&mut self, state: &mut u8, one: &[u8; 256], zero: &[u8; 256]) -> u32 {
        let rangeoff = (self.range * (*state as u32)) / 256;
        let range_after = self.range - rangeoff;
        let bit = if self.low < range_after {
            self.range = range_after;
            *state = zero[*state as usize];
            0
        } else {
            self.low -= range_after;
            self.range = rangeoff;
            *state = one[*state as usize];
            1
        };
        self.refill();
        bit
    }
}

// ---------------------------------------------------------------------
// ここから先: 上記CPU参照実装と同じ`get_rac`ロジックを、実GPU上の
// 単一invocationによる逐次ループとして実装する(rspirvで直接SPIR-Vを
// 組み立てる——`spirv_gen::build_subgroup_shuffle_xor1_kernel`と同じ理由
// で、これもDXBC/SM5.0からの翻訳ではない。DXBCの`loop`/`endloop`命令
// 自体はSM5.0に存在するが、このクレートの`decode_chain_shape`は
// 「制御フロー無しの式木」専用に設計されており、ループを含む形は
// 現時点でのデコーダのスコープ外——将来DXBCのloop命令からこのSPIR-V
// ループ構造への翻訳を追加する余地はあるが、今回は直接構築で検証を
// 優先した)。
//
// **正直な開示(このカーネルがまだ検証しない部分)**: これは1本の
// invocationが`num_symbols`回`get_rac`を逐次実行するだけであり、
// FFmpeg本家が行う「32レーンが並列に32個の異なるコンテキストの
// lookup/adaptを行う」という並列化そのものはまだ実装していない
// (`spirv_gen::build_subgroup_shuffle_xor1_kernel`で検証済みの
// subgroup shuffleの土台と、ここで検証する状態遷移テーブル駆動の
// 適応ロジック本体を、実際に組み合わせて32レーン並列化する作業が
// 次の課題として残る)。

use rspirv::binary::Assemble;
use rspirv::dr::{Builder, Operand as DrOperand};
use rspirv::spirv;

/// [`build_range_decoder_kernel`]が返すカーネル情報
/// (`spirv_gen::SubgroupShuffleKernel`と同型だが、レンジコーダー専用の
/// 別モジュールであることを示すためあえて別の型として定義する)。
#[derive(Debug, Clone)]
pub struct RangeDecoderKernel {
    pub spirv_words: Vec<u32>,
    pub entry_point: &'static str,
    /// 1(単一invocationが状態を逐次持ち回るため、並列化していない
    /// 現段階ではこれで正しい——`local_size`を増やしても複数
    /// invocationが同じ処理を重複して行うだけになる)。
    pub local_size: (u32, u32, u32),
}

/// `get_rac`を`num_symbols`回繰り返す実行するカーネルを直接組み立てる。
///
/// バッファ配線(いずれも`u32`配列、1要素1バイト/1状態を表す——実際の
/// バイトパッキングは行わず、簡潔さのため1バイトにつき1`u32`要素を
/// 使う正直な簡略化):
/// - binding 0: `bytestream`(先頭2要素が初期`low`、以降が`refill`で
///   1バイトずつ消費される)
/// - binding 1: `one_state`(256要素)
/// - binding 2: `zero_state`(256要素)
/// - binding 3: `output`(`num_symbols`要素、復号されたビット列)
///
/// `initial_state`/`num_symbols`はpush constantではなく、ビルド時に
/// `OpConstant`として直接埋め込む(2026-09-12設計変更: 当初push
/// constant案だったが、ディスパッチ側`open-cuda`の`chain_n_buffer`
/// カーネル名が渡すpush constantは常に4バイト〈要素数nのみ〉であり、
/// この関数が求める8バイト〈initial_state+num_symbols〉のpush constant
/// レイアウトとは一致しない——パイプラインレイアウトとシェーダー側の
/// 宣言が食い違うと未定義動作になるため、push constantを使わずビルド時
/// 定数にすることでこの不整合自体を無くした)。
pub fn build_range_decoder_kernel(initial_state: u32, num_symbols: u32) -> RangeDecoderKernel {
    let mut b = Builder::new();
    b.set_version(1, 0);
    b.capability(spirv::Capability::Shader);
    b.memory_model(spirv::AddressingModel::Logical, spirv::MemoryModel::GLSL450);

    let void_ty = b.type_void();
    let voidf_ty = b.type_function(void_ty, vec![]);
    let uint_ty = b.type_int(32, 0);
    let bool_ty = b.type_bool();

    let rt_array_ty = b.type_runtime_array(uint_ty);
    b.decorate(rt_array_ty, spirv::Decoration::ArrayStride, vec![DrOperand::LiteralBit32(4)]);
    let buf_struct_ty = b.type_struct(vec![rt_array_ty]);
    b.decorate(buf_struct_ty, spirv::Decoration::BufferBlock, vec![]);
    b.member_decorate(buf_struct_ty, 0, spirv::Decoration::Offset, vec![DrOperand::LiteralBit32(0)]);
    let buf_ptr_ty = b.type_pointer(None, spirv::StorageClass::Uniform, buf_struct_ty);
    let uint_ptr_uniform_ty = b.type_pointer(None, spirv::StorageClass::Uniform, uint_ty);

    let make_buffer_var = |b: &mut Builder, binding: u32| -> u32 {
        let var = b.variable(buf_ptr_ty, None, spirv::StorageClass::Uniform, None);
        b.decorate(var, spirv::Decoration::DescriptorSet, vec![DrOperand::LiteralBit32(0)]);
        b.decorate(var, spirv::Decoration::Binding, vec![DrOperand::LiteralBit32(binding)]);
        var
    };
    let var_bytestream = make_buffer_var(&mut b, 0);
    let var_one_state = make_buffer_var(&mut b, 1);
    let var_zero_state = make_buffer_var(&mut b, 2);
    let var_output = make_buffer_var(&mut b, 3);

    let uint_ptr_function_ty = b.type_pointer(None, spirv::StorageClass::Function, uint_ty);

    let main_fn = b.begin_function(void_ty, None, spirv::FunctionControl::NONE, voidf_ty).expect("OpFunction");
    b.begin_block(None).expect("OpLabel entry");

    // 定数(モジュール内で複数回使うのでここで一度だけ作る)。
    let const_0 = b.constant_bit32(uint_ty, 0);
    let const_1 = b.constant_bit32(uint_ty, 1);
    let const_2 = b.constant_bit32(uint_ty, 2);
    let const_256 = b.constant_bit32(uint_ty, 256);
    let const_0xff00 = b.constant_bit32(uint_ty, 0xFF00);

    // Function storage classのローカル変数は、そのOpFunctionの
    // エントリブロックの先頭にまとめて置く、というSPIR-Vの規約に従う
    // (このBuilderは今entry_labelブロックの中にいるので、ここで
    // まとめてOpVariableを発行してから、他の命令へ進む)。
    let var_i = b.variable(uint_ptr_function_ty, None, spirv::StorageClass::Function, None);
    let var_state = b.variable(uint_ptr_function_ty, None, spirv::StorageClass::Function, None);
    let var_low = b.variable(uint_ptr_function_ty, None, spirv::StorageClass::Function, None);
    let var_range = b.variable(uint_ptr_function_ty, None, spirv::StorageClass::Function, None);
    let var_pos = b.variable(uint_ptr_function_ty, None, spirv::StorageClass::Function, None);

    // 初期化: low = (bytestream[0]<<8) | bytestream[1]; range=0xFF00;
    // pos=2; i=0; state=initial_state(push constant)。
    let ac_b0 = b.access_chain(uint_ptr_uniform_ty, None, var_bytestream, vec![const_0, const_0]).expect("byte[0]");
    let b0 = b.load(uint_ty, None, ac_b0, None, vec![]).expect("load byte[0]");
    let ac_b1 = b.access_chain(uint_ptr_uniform_ty, None, var_bytestream, vec![const_0, const_1]).expect("byte[1]");
    let b1 = b.load(uint_ty, None, ac_b1, None, vec![]).expect("load byte[1]");
    let const_8 = b.constant_bit32(uint_ty, 8);
    let b0_shifted = b.shift_left_logical(uint_ty, None, b0, const_8).expect("byte[0]<<8");
    let low_init = b.bitwise_or(uint_ty, None, b0_shifted, b1).expect("(byte[0]<<8)|byte[1]");
    b.store(var_low, low_init, None, vec![]).expect("store low init");
    b.store(var_range, const_0xff00, None, vec![]).expect("store range init");
    b.store(var_pos, const_2, None, vec![]).expect("store pos init");
    b.store(var_i, const_0, None, vec![]).expect("store i init");
    let const_initial_state = b.constant_bit32(uint_ty, initial_state);
    b.store(var_state, const_initial_state, None, vec![]).expect("store state init");

    let loop_header = b.id();
    let loop_cond = b.id();
    let loop_body = b.id();
    let loop_continue = b.id();
    let loop_merge = b.id();
    b.branch(loop_header).expect("branch to loop header");

    b.begin_block(Some(loop_header)).expect("OpLabel loop_header");
    b.loop_merge(loop_merge, loop_continue, spirv::LoopControl::NONE, vec![]).expect("OpLoopMerge");
    b.branch(loop_cond).expect("branch to loop cond");

    b.begin_block(Some(loop_cond)).expect("OpLabel loop_cond");
    let i_val = b.load(uint_ty, None, var_i, None, vec![]).expect("load i");
    let const_num_symbols = b.constant_bit32(uint_ty, num_symbols);
    let cond = b.u_less_than(bool_ty, None, i_val, const_num_symbols).expect("i < num_symbols");
    b.branch_conditional(cond, loop_body, loop_merge, vec![]).expect("OpBranchConditional loop");

    b.begin_block(Some(loop_body)).expect("OpLabel loop_body");
    // rangeoff = (range * state) / 256; range_after = range - rangeoff;
    let state_val = b.load(uint_ty, None, var_state, None, vec![]).expect("load state");
    let range_val = b.load(uint_ty, None, var_range, None, vec![]).expect("load range");
    let low_val = b.load(uint_ty, None, var_low, None, vec![]).expect("load low");
    let range_times_state = b.i_mul(uint_ty, None, range_val, state_val).expect("range*state");
    let rangeoff = b.u_div(uint_ty, None, range_times_state, const_256).expect("(range*state)/256");
    let range_after = b.i_sub(uint_ty, None, range_val, rangeoff).expect("range-rangeoff");
    let bit_is_zero = b.u_less_than(bool_ty, None, low_val, range_after).expect("low < range_after");

    let branch_zero = b.id();
    let branch_one = b.id();
    let branch_merge = b.id();
    b.selection_merge(branch_merge, spirv::SelectionControl::NONE).expect("OpSelectionMerge bit");
    b.branch_conditional(bit_is_zero, branch_zero, branch_one, vec![]).expect("OpBranchConditional bit");

    // bit==0: range=range_after; state=zero_state[state]; output[i]=0
    b.begin_block(Some(branch_zero)).expect("OpLabel branch_zero");
    b.store(var_range, range_after, None, vec![]).expect("store range (bit=0)");
    let ac_zero_next = b
        .access_chain(uint_ptr_uniform_ty, None, var_zero_state, vec![const_0, state_val])
        .expect("zero_state[state]");
    let zero_next_state = b.load(uint_ty, None, ac_zero_next, None, vec![]).expect("load zero_state[state]");
    b.store(var_state, zero_next_state, None, vec![]).expect("store state (bit=0)");
    let ac_out0 = b.access_chain(uint_ptr_uniform_ty, None, var_output, vec![const_0, i_val]).expect("output[i] (bit=0)");
    b.store(ac_out0, const_0, None, vec![]).expect("store output[i]=0");
    b.branch(branch_merge).expect("branch_zero -> branch_merge");

    // bit==1: low -= range_after; range=rangeoff; state=one_state[state]; output[i]=1
    b.begin_block(Some(branch_one)).expect("OpLabel branch_one");
    let low_after = b.i_sub(uint_ty, None, low_val, range_after).expect("low-range_after");
    b.store(var_low, low_after, None, vec![]).expect("store low (bit=1)");
    b.store(var_range, rangeoff, None, vec![]).expect("store range (bit=1)");
    let ac_one_next = b
        .access_chain(uint_ptr_uniform_ty, None, var_one_state, vec![const_0, state_val])
        .expect("one_state[state]");
    let one_next_state = b.load(uint_ty, None, ac_one_next, None, vec![]).expect("load one_state[state]");
    b.store(var_state, one_next_state, None, vec![]).expect("store state (bit=1)");
    let ac_out1 = b.access_chain(uint_ptr_uniform_ty, None, var_output, vec![const_0, i_val]).expect("output[i] (bit=1)");
    b.store(ac_out1, const_1, None, vec![]).expect("store output[i]=1");
    b.branch(branch_merge).expect("branch_one -> branch_merge");

    b.begin_block(Some(branch_merge)).expect("OpLabel branch_merge");
    // refill(): if (range < 256) { range*=256; low*=256; low+=bytestream[pos]; pos++; }
    let range_val2 = b.load(uint_ty, None, var_range, None, vec![]).expect("load range (refill)");
    let need_refill = b.u_less_than(bool_ty, None, range_val2, const_256).expect("range < 256");
    let refill_then = b.id();
    let refill_merge = b.id();
    b.selection_merge(refill_merge, spirv::SelectionControl::NONE).expect("OpSelectionMerge refill");
    b.branch_conditional(need_refill, refill_then, refill_merge, vec![]).expect("OpBranchConditional refill");

    b.begin_block(Some(refill_then)).expect("OpLabel refill_then");
    let range_val3 = b.i_mul(uint_ty, None, range_val2, const_256).expect("range*256");
    b.store(var_range, range_val3, None, vec![]).expect("store range (refill)");
    let low_val2 = b.load(uint_ty, None, var_low, None, vec![]).expect("load low (refill)");
    let low_val3 = b.i_mul(uint_ty, None, low_val2, const_256).expect("low*256");
    let pos_val = b.load(uint_ty, None, var_pos, None, vec![]).expect("load pos");
    let ac_next_byte =
        b.access_chain(uint_ptr_uniform_ty, None, var_bytestream, vec![const_0, pos_val]).expect("bytestream[pos]");
    let next_byte = b.load(uint_ty, None, ac_next_byte, None, vec![]).expect("load bytestream[pos]");
    let low_val4 = b.i_add(uint_ty, None, low_val3, next_byte).expect("low*256+byte");
    b.store(var_low, low_val4, None, vec![]).expect("store low (refill)");
    let pos_val2 = b.i_add(uint_ty, None, pos_val, const_1).expect("pos+1");
    b.store(var_pos, pos_val2, None, vec![]).expect("store pos (refill)");
    b.branch(refill_merge).expect("refill_then -> refill_merge");

    b.begin_block(Some(refill_merge)).expect("OpLabel refill_merge");
    b.branch(loop_continue).expect("branch to loop_continue");

    b.begin_block(Some(loop_continue)).expect("OpLabel loop_continue");
    let i_val2 = b.load(uint_ty, None, var_i, None, vec![]).expect("load i (continue)");
    let i_val3 = b.i_add(uint_ty, None, i_val2, const_1).expect("i+1");
    b.store(var_i, i_val3, None, vec![]).expect("store i (continue)");
    b.branch(loop_header).expect("branch back to loop_header");

    b.begin_block(Some(loop_merge)).expect("OpLabel loop_merge");
    b.ret().expect("OpReturn");
    b.end_function().expect("OpFunctionEnd");

    b.entry_point(spirv::ExecutionModel::GLCompute, main_fn, "main", vec![]);
    b.execution_mode(main_fn, spirv::ExecutionMode::LocalSize, [1, 1, 1]);

    let module = b.module();
    let spirv_words = module.assemble();
    RangeDecoderKernel { spirv_words, entry_point: "main", local_size: (1, 1, 1) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_state_and_one_state_satisfy_the_rfc_symmetry_relation() {
        let one = one_state();
        let zero = zero_state();
        for i in 1..256usize {
            // Cの`uint8_t`配列演算と同じく、`256 - one[256-i]`が256になる
            // (one[256-i]==0の)場合はu8へラップして0になる——本家FFmpeg
            // 実装のテーブルもuint8_t配列である以上、この意味論で一致する。
            let expected = (256u16 - one[256 - i] as u16) as u8;
            assert_eq!(zero[i], expected, "mismatch at i={i}");
        }
        assert_eq!(zero[0], 0);
    }

    #[test]
    fn get_rac_produces_a_deterministic_bit_sequence_from_a_fixed_byte_stream() {
        // 実際の圧縮ビットストリームではなく合成データだが、CPU参照実装
        // 自体が決定的に(何度実行しても同じ結果を返す)動作することを
        // まず確認する——このテスト自体がGPU版と数値を突き合わせる基準になる。
        let bytes = [0x5Au8, 0xA3, 0x12, 0x9F, 0x00, 0xFF, 0x77, 0x88, 0x3C, 0x64];
        let one = one_state();
        let zero = zero_state();
        let mut dec = RangeDecoderCpu::new(&bytes);
        let mut state = 128u8;
        let mut bits = Vec::new();
        for _ in 0..16 {
            bits.push(dec.get_rac(&mut state, &one, &zero));
        }
        let mut dec2 = RangeDecoderCpu::new(&bytes);
        let mut state2 = 128u8;
        let mut bits2 = Vec::new();
        for _ in 0..16 {
            bits2.push(dec2.get_rac(&mut state2, &one, &zero));
        }
        assert_eq!(bits, bits2, "get_racは同じ入力に対して決定的であるべき");
    }
}
