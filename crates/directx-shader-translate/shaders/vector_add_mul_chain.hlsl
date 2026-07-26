// タスク2(DXBCデコーダの一般化)向けの新規シェーダー(D3D11 Compute Shader, SM5.0)。
// `vector_add`/`vector_mul`/`vector_sub_bounded`/`vector_div`は全て「1回の
// 2項演算+一時レジスタ1個」という同一形状だったため、これを一般化する軸として
// 「N個の逐次2項演算(制御フロー無し)」を選んだ(タスク指示のcandidate 1)。
// t = InputA[i] + InputB[i](1回目の演算)、Output[i] = t * InputA[i]
// (2回目の演算、tと`InputA[i]`の再読み込みを掛け合わせる)という2演算の式を
// 計算する——UAV3本のまま(既存の`opencuda-vulkan::VulkanDevice`が
// `launch_kernel`で"vector_add"名の引数配線として厳密に3バッファ固定で
// 期待するため、実Vulkan実機検証を既存の配線経路にそのまま乗せられるよう
// あえてUAV本数は既存4形状と揃え、演算回数だけを2回へ増やした)。

RWStructuredBuffer<float> InputA : register(u0);
RWStructuredBuffer<float> InputB : register(u1);
RWStructuredBuffer<float> Output : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 dtid : SV_DispatchThreadID)
{
    uint i = dtid.x;
    float t = InputA[i] + InputB[i];
    Output[i] = t * InputA[i];
}
