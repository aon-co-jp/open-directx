// D3D11 Compute Shader (SM5.0)。境界チェック付き(`if (id.x < N)`)の
// 減算シェーダー。RWStructuredBuffer 3本 + 定数バッファ(要素数N)。
// vector_add.hlsl/vector_mul.hlslには無かった「比較+分岐」オペコード
// (ilt/if_z等)と「定数バッファからの読み込み」をSHEXデコーダへ追加
// させるための3本目の実シェーダー。

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
        Output[i] = InputA[i] - InputB[i];
    }
}
