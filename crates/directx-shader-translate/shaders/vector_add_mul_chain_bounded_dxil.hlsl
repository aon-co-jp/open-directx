// vector_add_mul_chain_bounded.hlsl(DXBC/SM5.0、境界チェック付き2項演算
// チェーン)と全く同じ契約・演算内容(cbuffer(b0)でElementCountを受け取り、
// if (i < ElementCount) の内側で t = a[i]+b[i]; c[i] = t*a[i]; を実行)だが、
// こちらは dxc.exe で -T cs_6_0 にコンパイルする(SM6.0 = DXIL)。既存の
// *_dxil.hlsl分離パターン(vector_add_mul_chain_dxil.hlsl等)に合わせて
// 別ファイルとした。DXIL側の境界チェック対応(resolve_dxil_calls_and_chain
// への検出ロジック追加)を検証するための新規シェーダー(2026-08-06)。

cbuffer Params : register(b0)
{
    uint ElementCount;
};

RWStructuredBuffer<float> a : register(u0);
RWStructuredBuffer<float> b : register(u1);
RWStructuredBuffer<float> c : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    uint i = id.x;
    if (i < ElementCount)
    {
        float t = a[i] + b[i];
        c[i] = t * a[i];
    }
}
