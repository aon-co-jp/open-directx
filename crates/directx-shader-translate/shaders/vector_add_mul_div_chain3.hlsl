// タスク(1)(3項以上のチェーン検証)向けの新規シェーダー(D3D11 Compute
// Shader, SM5.0)。既存の`vector_add_mul_chain.hlsl`/`vector_sub_div_chain.hlsl`
// (いずれも「2個の逐次2項演算」)を、3個の逐次2項演算へ拡張したもの。
// t1 = InputA[i] + InputB[i](1回目: add)
// t2 = t1 * InputA[i](2回目: mul、InputAの再読み込み——既存2項チェーンと
//      同じCSEパターンを踏襲)
// Output[i] = t2 / InputB[i](3回目: div、InputBの再読み込み)
// UAVは既存チェーン群と同じく3本のまま(InputA/InputBをそれぞれ2回参照)——
// `opencuda-vulkan::VulkanDevice::launch_kernel`が"vector_add"名の引数配線
// として厳密に3バッファ固定で期待するため、既存の実Vulkan配線経路に
// そのまま乗せられるようにした(既存の2項チェーン群と同じ設計判断)。

RWStructuredBuffer<float> InputA : register(u0);
RWStructuredBuffer<float> InputB : register(u1);
RWStructuredBuffer<float> Output : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 dtid : SV_DispatchThreadID)
{
    uint i = dtid.x;
    float t1 = InputA[i] + InputB[i];
    float t2 = t1 * InputA[i];
    Output[i] = t2 / InputB[i];
}
