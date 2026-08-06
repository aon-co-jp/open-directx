// タスク(1)(4項以上のチェーン、またはsub/divを含む新しい演算順序組み合わせ
// での実シェーダー検証)向けの新規シェーダー(D3D11 Compute Shader, SM5.0)。
// 既存の`vector_add_mul_div_chain3.hlsl`(add->mul->div、3項)とは異なり、
// (a) 演算数を4個へ増やし、かつ(b) sub/divを含む未検証の順序
// (sub -> div -> add -> mul)を選んだ。既存チェーン群はadd/mul/divのみを
// 含む順序(add->mul、sub->div、add->mul->div)しか実バイト列で検証して
// いなかったため、subが先頭に来る・sub/divとadd/mulが混在する順序は
// 今回が初めての実コンパイル・実機検証となる。
//
// t1 = InputA[i] - InputB[i](1回目: sub、fxc.exeは`negated add`へ最適化する
//      規約——decode_chain_shape側の既存のnegate処理がそのまま通るかを
//      確認する狙い)
// t2 = t1 / InputA[i](2回目: div、InputAの再読み込み)
// t3 = t2 + InputB[i](3回目: add、InputBの再読み込み)
// Output[i] = t3 * InputA[i](4回目: mul、InputAの再読み込み)
// UAVは既存チェーン群と同じく3本のまま(InputA/InputBをそれぞれ2回参照)。

RWStructuredBuffer<float> InputA : register(u0);
RWStructuredBuffer<float> InputB : register(u1);
RWStructuredBuffer<float> Output : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 dtid : SV_DispatchThreadID)
{
    uint i = dtid.x;
    float t1 = InputA[i] - InputB[i];
    float t2 = t1 / InputA[i];
    float t3 = t2 + InputB[i];
    Output[i] = t3 * InputA[i];
}
