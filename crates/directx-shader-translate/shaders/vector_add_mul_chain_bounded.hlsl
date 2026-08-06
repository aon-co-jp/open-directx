// 境界チェック付きチェーン(既存の「次にすべきこと」項目、複数のHANDOFF
// エントリで未対応と明記されていたギャップ)を解消するための新規シェーダー
// (D3D11 Compute Shader, SM5.0)。`vector_add_mul_chain.hlsl`(2項演算チェーン、
// 境界チェック無し)と`vector_sub_bounded.hlsl`(単一演算、境界チェック有り)を
// 組み合わせ、「2項演算のチェーン + 定数バッファ(要素数N) + ult/if/endifに
// よる境界チェック」という、既存のどのクラスにも当たらない組み合わせを
// SHEXデコーダへ追加させる。UAV本数は既存のチェーン系と同じく3本
// (`opencuda-vulkan::VulkanDevice::launch_kernel`の"vector_add"名固定引数
// 配線にそのまま乗せるため)。

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
        float t = InputA[i] + InputB[i];
        Output[i] = t * InputA[i];
    }
}
