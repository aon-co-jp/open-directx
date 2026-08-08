// 境界チェック付きチェーンの7項への拡張(2026-08-08、rs-sync横断セッション
// より「6項以上は未検証」というギャップを埋める増分)向けの新規シェーダー
// (D3D11 Compute Shader, SM5.0)。
// `vector_add_mul_div_sub_add_mul_chain6_bounded.hlsl`(境界チェック付き、
// 6項まで検証済み: add->mul->div->sub->add->mul)にもう1個演算(div)を
// 追加し、「7個の逐次2項演算チェーン + 定数バッファ(要素数N) +
// ult/if/endifによる境界チェック」という、これまで未検証だった項数を
// SHEX/DXILデコーダへ実際に検証させる。UAV本数は既存チェーン系と同じく
// 3本(`opencuda-vulkan::VulkanDevice::launch_kernel`の"vector_add"名固定
// 引数配線にそのまま乗せるため)。

cbuffer Params : register(b0)
{
    uint ElementCount;
};

RWStructuredBuffer<float> InputA : register(u0);
RWStructuredBuffer<float> InputB : register(u1);
RWStructuredBuffer<float> Output : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 dtid : SV_DispatchThreadID)
{
    uint i = dtid.x;
    if (i < ElementCount)
    {
        float t1 = InputA[i] + InputB[i];
        float t2 = t1 * InputA[i];
        float t3 = t2 / InputB[i];
        float t4 = t3 - InputA[i];
        float t5 = t4 + InputB[i];
        float t6 = t5 * InputA[i];
        Output[i] = t6 / InputB[i];
    }
}
