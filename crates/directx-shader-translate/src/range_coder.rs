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

// ---------------------------------------------------------------------
// ここから先: 32レーン並列化版。FFmpeg本家の実ソース
// (`https://github.com/FFmpeg/FFmpeg/blob/master/libavcodec/vulkan/
// rangecoder.glsl`、2026-09-13に実際にfetchして読んだ)を確認したところ、
// **当初の想定(subgroup shuffle)は誤りだった**——実際の機構は
// `shared`(GLSLのworkgroup共有メモリ、SPIR-Vの`Workgroup`ストレージ
// クラス)+`barrier()`(`OpControlBarrier`)であり、`subgroupShuffle`
// (`OpGroupNonUniformShuffle`)は一切使われていない。
//
// 実際のFFmpegソースの構造(`rangecoder.glsl`より):
// ```glsl
// shared RangeCoder rc;                       // ワークグループ共有、単一
// shared uint8_t rc_state[NB_CONTEXTS*32];     // ワークグループ共有、32要素
// bool get_rac_state(uint idx) {               // 呼び出し側(1invocation)が
//     return rc_data[idx] = get_rac_internal(rc.range * rc_state[idx] >> 8);
// }
// ```
// つまり「32個のコンテキストのlookup/adapt」は、32本のinvocationが
// `rc_state[gl_LocalInvocationIndex] = ...`という形で**それぞれ自分の
// 担当インデックスへ並列に書き込み**、`barrier()`で同期した後、
// 1本のinvocation(通常invocation 0)だけが`rc_state[]`を順番に読んで
// 実際のレンジコーダー逐次更新(`rc.low`/`rc.range`)を行う、という
// **共有メモリ+バリア**方式だった——`subgroupShuffle`のような
// レーン間直接データ交換命令は不要だった。
//
// 前回実装した`subgroup_shuffle_real_vulkan.rs`のsubgroup shuffle検証
// 自体は無駄ではない(GT730が`OpGroupNonUniformShuffle`を実際にサポート
// することを実証した、独立して価値のある結果)が、**FFv1のレンジ
// コーダーが実際に使う機構ではなかった**、という正直な訂正を
// `PORTING.md`/`CLAUDE.md`にも記録する。
//
// 以下は、この実際の機構(共有メモリ+バリア)に忠実な32レーン並列化
// カーネル。`build_range_decoder_kernel`(1invocation逐次版)と
// `build_subgroup_shuffle_xor1_kernel`で検証済みの個別要素を、実際の
// FFmpeg設計に合わせて組み合わせ直したもの。

/// [`build_range_decoder_parallel_kernel`]が返すカーネル情報。
#[derive(Debug, Clone)]
pub struct ParallelRangeDecoderKernel {
    pub spirv_words: Vec<u32>,
    pub entry_point: &'static str,
    /// `(context_size, 1, 1)`——呼び出し側が指定した`context_size`
    /// (FFmpeg実装での定数名`CONTEXT_SIZE`、本家は32固定)をそのまま
    /// 使う。ワークグループ共有メモリ+バリアのみに依存する設計のため、
    /// GPUのsubgroup幅(GT730では32)を超える値でも正しく動く
    /// (2026-09-13、64での実機検証済み)。
    pub local_size: (u32, u32, u32),
}

/// FFmpeg本家`rangecoder.glsl`の`shared`+`barrier()`方式に倣った、
/// `context_size`個のコンテキスト分の`get_rac`を1回のディスパッチで
/// 処理するカーネル。
///
/// バッファ配線(いずれも`u32`配列):
/// - binding 0: `bytestream`(先頭2要素が初期`low`)
/// - binding 1: `zero_one_state`(512要素——`[0..256)`が`zero_state`、
///   `[256..512)`が`one_state`、FFmpeg実ソースと同じ1本化レイアウト)
/// - binding 2: `context_states`(`context_size`要素、各コンテキストの
///   現在状態。読み込み+このディスパッチ後の状態で上書き)
/// - binding 3: `output`(`context_size`要素、復号されたビット列)
///
/// アルゴリズム(`context_size`個のinvocationで1ワークグループ、
/// `local_size=(context_size,1,1)`):
/// 1. 各invocationが自分の`gl_LocalInvocationIndex`に対応する
///    `context_states[lane]`を読み、`shared`配列の同じインデックスへ
///    書く(**並列のlookup**、FFmpegの`rc_state[idx]=...`相当)。
/// 2. `OpControlBarrier`(Workgroupスコープ)で同期。
/// 3. `lane==0`のinvocationだけが、`shared`配列を`i=0..context_size-1`
///    の順で読み、実際の`get_rac`逐次更新(`low`/`range`/`pos`、
///    `RangeDecoderCpu`と全く同じ式)を行い、更新後の状態を同じ
///    `shared`配列へ書き戻し、復号ビットを`output[i]`へ書く
///    (**1レーンだけが実際の直列処理**、FFmpegの「1invocationが実
///    エンコード/デコードを行う」設計と対応)。
/// 4. 再度`OpControlBarrier`。
/// 5. 各invocationが`shared`配列の自分のインデックスを読み、
///    `context_states[lane]`へ書き戻す(**並列の書き戻し**)。
///
/// **2026-09-13追加(64コンテキスト版の検証)**: FFmpeg本家の
/// `CONTEXT_SIZE`は32(GT730のsubgroupSizeと同じ)固定だが、この
/// カーネルが実際に使っているのは`Workgroup`共有メモリ+
/// `OpControlBarrier`のみで、subgroup幅に依存する命令
/// (`OpGroupNonUniformShuffle`等)は一切使っていない——ワークグループ
/// バリアはワークグループ内の全invocationを対象にでき、GPUのsubgroup幅
/// (32)を超えるワークグループサイズ(例: 64、内部的に2 subgroup分)でも
/// 正しく機能する。そのため`context_size`を32以外(64等)にしても
/// アルゴリズム上の変更は不要で、`local_size`と`shared`配列長・
/// ループ回数を`context_size`に合わせるだけで良いことを、実際に
/// `context_size=64`の実GPUテストで検証した(下記テスト参照)。
pub fn build_range_decoder_parallel_kernel(context_size: u32) -> ParallelRangeDecoderKernel {
    let mut b = Builder::new();
    b.set_version(1, 0);
    b.capability(spirv::Capability::Shader);
    b.memory_model(spirv::AddressingModel::Logical, spirv::MemoryModel::GLSL450);

    let void_ty = b.type_void();
    let voidf_ty = b.type_function(void_ty, vec![]);
    let uint_ty = b.type_int(32, 0);
    let bool_ty = b.type_bool();

    // storage buffer群(vector_add系と同じBufferBlock+runtime array)。
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
    let var_zero_one_state = make_buffer_var(&mut b, 1);
    let var_context_states = make_buffer_var(&mut b, 2);
    let var_output = make_buffer_var(&mut b, 3);

    // ワークグループ共有メモリ(GLSLの`shared`、SPIR-Vの`Workgroup`
    // ストレージクラス)。固定長32要素の`OpTypeArray`(runtime arrayでは
    // 使えない——Workgroupストレージクラスは固定長配列のみ許可)。
    let const_32_len = b.constant_bit32(uint_ty, context_size);
    let shared_array_ty = b.type_array(uint_ty, const_32_len);
    let shared_ptr_ty = b.type_pointer(None, spirv::StorageClass::Workgroup, shared_array_ty);
    let var_shared_state = b.variable(shared_ptr_ty, None, spirv::StorageClass::Workgroup, None);
    let uint_ptr_workgroup_ty = b.type_pointer(None, spirv::StorageClass::Workgroup, uint_ty);

    // gl_LocalInvocationIndex(ワークグループ内でのフラット化された
    // invocation番号——`local_size=(32,1,1)`ならそのまま0..31)。
    let uint_ptr_input_ty = b.type_pointer(None, spirv::StorageClass::Input, uint_ty);
    let var_lane = b.variable(uint_ptr_input_ty, None, spirv::StorageClass::Input, None);
    b.decorate(var_lane, spirv::Decoration::BuiltIn, vec![DrOperand::BuiltIn(spirv::BuiltIn::LocalInvocationIndex)]);

    let uint_ptr_function_ty = b.type_pointer(None, spirv::StorageClass::Function, uint_ty);

    let main_fn = b.begin_function(void_ty, None, spirv::FunctionControl::NONE, voidf_ty).expect("OpFunction");
    b.begin_block(None).expect("OpLabel entry");

    let const_0 = b.constant_bit32(uint_ty, 0);
    let const_1 = b.constant_bit32(uint_ty, 1);
    let const_2 = b.constant_bit32(uint_ty, 2);
    let const_8 = b.constant_bit32(uint_ty, 8);
    let const_256 = b.constant_bit32(uint_ty, 256);
    let const_0xff00 = b.constant_bit32(uint_ty, 0xFF00);
    let const_32 = b.constant_bit32(uint_ty, context_size);
    let scope_workgroup = b.constant_bit32(uint_ty, spirv::Scope::Workgroup as u32);
    let semantics_release_workgroup =
        b.constant_bit32(uint_ty, (spirv::MemorySemantics::ACQUIRE_RELEASE | spirv::MemorySemantics::WORKGROUP_MEMORY).bits());

    // Function storage classのローカル変数(このinvocationがlane==0の
    // 場合にのみ実際に使う逐次状態)は、規約通りエントリブロック先頭で
    // まとめて宣言する。
    let var_i = b.variable(uint_ptr_function_ty, None, spirv::StorageClass::Function, None);
    let var_low = b.variable(uint_ptr_function_ty, None, spirv::StorageClass::Function, None);
    let var_range = b.variable(uint_ptr_function_ty, None, spirv::StorageClass::Function, None);
    let var_pos = b.variable(uint_ptr_function_ty, None, spirv::StorageClass::Function, None);

    // --- ステップ1: 32レーン並列lookup(自分のcontext_statesを共有配列へ) ---
    let lane = b.load(uint_ty, None, var_lane, None, vec![]).expect("load lane");
    let ac_ctx_in = b.access_chain(uint_ptr_uniform_ty, None, var_context_states, vec![const_0, lane]).expect("context_states[lane]");
    let my_initial_state = b.load(uint_ty, None, ac_ctx_in, None, vec![]).expect("load context_states[lane]");
    let ac_shared_in = b.access_chain(uint_ptr_workgroup_ty, None, var_shared_state, vec![lane]).expect("shared_state[lane] (write)");
    b.store(ac_shared_in, my_initial_state, None, vec![]).expect("store shared_state[lane]");

    // --- ステップ2: バリア ---
    b.control_barrier(scope_workgroup, scope_workgroup, semantics_release_workgroup).expect("OpControlBarrier #1");

    // --- ステップ3: lane==0のみ、32回分を逐次処理 ---
    let is_lane0 = b.i_equal(bool_ty, None, lane, const_0).expect("lane == 0");
    let then_label = b.id();
    let after_serial_label = b.id();
    b.selection_merge(after_serial_label, spirv::SelectionControl::NONE).expect("OpSelectionMerge lane0");
    b.branch_conditional(is_lane0, then_label, after_serial_label, vec![]).expect("OpBranchConditional lane0");

    b.begin_block(Some(then_label)).expect("OpLabel then (lane0)");
    // low = (byte[0]<<8)|byte[1]; range=0xFF00; pos=2; i=0;
    let ac_b0 = b.access_chain(uint_ptr_uniform_ty, None, var_bytestream, vec![const_0, const_0]).expect("byte[0]");
    let b0 = b.load(uint_ty, None, ac_b0, None, vec![]).expect("load byte[0]");
    let ac_b1 = b.access_chain(uint_ptr_uniform_ty, None, var_bytestream, vec![const_0, const_1]).expect("byte[1]");
    let b1 = b.load(uint_ty, None, ac_b1, None, vec![]).expect("load byte[1]");
    let b0_shifted = b.shift_left_logical(uint_ty, None, b0, const_8).expect("byte[0]<<8");
    let low_init = b.bitwise_or(uint_ty, None, b0_shifted, b1).expect("(byte[0]<<8)|byte[1]");
    b.store(var_low, low_init, None, vec![]).expect("store low init");
    b.store(var_range, const_0xff00, None, vec![]).expect("store range init");
    b.store(var_pos, const_2, None, vec![]).expect("store pos init");
    b.store(var_i, const_0, None, vec![]).expect("store i init");
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
    let cond = b.u_less_than(bool_ty, None, i_val, const_32).expect("i < context_size");
    b.branch_conditional(cond, loop_body, loop_merge, vec![]).expect("OpBranchConditional loop");

    b.begin_block(Some(loop_body)).expect("OpLabel loop_body");
    let ac_shared_i = b.access_chain(uint_ptr_workgroup_ty, None, var_shared_state, vec![i_val]).expect("shared_state[i]");
    let state_i = b.load(uint_ty, None, ac_shared_i, None, vec![]).expect("load shared_state[i]");
    let range_val = b.load(uint_ty, None, var_range, None, vec![]).expect("load range");
    let low_val = b.load(uint_ty, None, var_low, None, vec![]).expect("load low");
    let range_times_state = b.i_mul(uint_ty, None, range_val, state_i).expect("range*state_i");
    let rangeoff = b.u_div(uint_ty, None, range_times_state, const_256).expect("(range*state_i)/256");
    let range_after = b.i_sub(uint_ty, None, range_val, rangeoff).expect("range-rangeoff");
    let bit_is_zero = b.u_less_than(bool_ty, None, low_val, range_after).expect("low < range_after");

    let branch_zero = b.id();
    let branch_one = b.id();
    let branch_merge = b.id();
    b.selection_merge(branch_merge, spirv::SelectionControl::NONE).expect("OpSelectionMerge bit");
    b.branch_conditional(bit_is_zero, branch_zero, branch_one, vec![]).expect("OpBranchConditional bit");

    // bit==0: range=range_after; new_state=zero_one_state[state_i]; output[i]=0
    b.begin_block(Some(branch_zero)).expect("OpLabel branch_zero");
    b.store(var_range, range_after, None, vec![]).expect("store range (bit=0)");
    let ac_zos0 =
        b.access_chain(uint_ptr_uniform_ty, None, var_zero_one_state, vec![const_0, state_i]).expect("zero_one_state[state_i]");
    let new_state0 = b.load(uint_ty, None, ac_zos0, None, vec![]).expect("load zero_one_state[state_i]");
    let ac_shared_i0 = b.access_chain(uint_ptr_workgroup_ty, None, var_shared_state, vec![i_val]).expect("shared_state[i] (write, bit=0)");
    b.store(ac_shared_i0, new_state0, None, vec![]).expect("store shared_state[i] (bit=0)");
    let ac_out0 = b.access_chain(uint_ptr_uniform_ty, None, var_output, vec![const_0, i_val]).expect("output[i] (bit=0)");
    b.store(ac_out0, const_0, None, vec![]).expect("store output[i]=0");
    b.branch(branch_merge).expect("branch_zero -> branch_merge");

    // bit==1: low-=range_after; range=rangeoff; new_state=zero_one_state[256+state_i]; output[i]=1
    b.begin_block(Some(branch_one)).expect("OpLabel branch_one");
    let low_after = b.i_sub(uint_ty, None, low_val, range_after).expect("low-range_after");
    b.store(var_low, low_after, None, vec![]).expect("store low (bit=1)");
    b.store(var_range, rangeoff, None, vec![]).expect("store range (bit=1)");
    let state_i_plus_256 = b.i_add(uint_ty, None, state_i, const_256).expect("state_i+256");
    let ac_zos1 = b
        .access_chain(uint_ptr_uniform_ty, None, var_zero_one_state, vec![const_0, state_i_plus_256])
        .expect("zero_one_state[256+state_i]");
    let new_state1 = b.load(uint_ty, None, ac_zos1, None, vec![]).expect("load zero_one_state[256+state_i]");
    let ac_shared_i1 = b.access_chain(uint_ptr_workgroup_ty, None, var_shared_state, vec![i_val]).expect("shared_state[i] (write, bit=1)");
    b.store(ac_shared_i1, new_state1, None, vec![]).expect("store shared_state[i] (bit=1)");
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

    b.begin_block(Some(loop_merge)).expect("OpLabel loop_merge (end of lane0 serial work)");
    b.branch(after_serial_label).expect("branch to after_serial_label");

    // --- ステップ4: バリア ---
    b.begin_block(Some(after_serial_label)).expect("OpLabel after_serial_label");
    b.control_barrier(scope_workgroup, scope_workgroup, semantics_release_workgroup).expect("OpControlBarrier #2");

    // --- ステップ5: 32レーン並列書き戻し ---
    let ac_shared_out = b.access_chain(uint_ptr_workgroup_ty, None, var_shared_state, vec![lane]).expect("shared_state[lane] (read back)");
    let final_state = b.load(uint_ty, None, ac_shared_out, None, vec![]).expect("load shared_state[lane] (final)");
    let ac_ctx_out = b.access_chain(uint_ptr_uniform_ty, None, var_context_states, vec![const_0, lane]).expect("context_states[lane] (write back)");
    b.store(ac_ctx_out, final_state, None, vec![]).expect("store context_states[lane] (final)");

    b.ret().expect("OpReturn");
    b.end_function().expect("OpFunctionEnd");

    b.entry_point(spirv::ExecutionModel::GLCompute, main_fn, "main", vec![var_lane]);
    b.execution_mode(main_fn, spirv::ExecutionMode::LocalSize, [context_size, 1, 1]);

    let module = b.module();
    let spirv_words = module.assemble();
    ParallelRangeDecoderKernel { spirv_words, entry_point: "main", local_size: (context_size, 1, 1) }
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
