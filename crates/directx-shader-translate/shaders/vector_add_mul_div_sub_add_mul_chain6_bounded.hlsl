// 境界チェック付きチェーンの6項への拡張(2026-08-07付HANDOFF「次にすべき
// こと(1)」)向けの新規シェーダー(D3D11 Compute Shader, SM5.0)。
// `vector_add_mul_div_sub_add_chain5_bounded.hlsl`(境界チェック付き、5項
// まで検証済み: add->mul->div->sub->add)にもう1個演算(mul)を追加し、
// 「6個の逐次2項演算チェーン + 定数バッファ(要素数N) + ult/if/endifに
// よる境界チェック」という、これまで未検証だった項数をSHEX/DXILデコーダ
// へ実際に検証させる。UAV本数は既存チェーン系と同じく3本
// (`opencuda-vulkan::VulkanDevice::launch_kernel`の"vector_add"名固定
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
        Output[i] = t5 * InputA[i];
    }
}
