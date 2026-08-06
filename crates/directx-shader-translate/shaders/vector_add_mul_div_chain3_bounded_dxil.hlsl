// vector_add_mul_div_chain3_bounded.hlsl(DXBC/SM5.0、境界チェック付き3項
// 演算チェーン)と全く同じ契約・演算内容(cbuffer(b0)でElementCountを受け
// 取り、if (i < ElementCount) の内側で t1=a[i]+b[i]; t2=t1*a[i];
// c[i]=t2/b[i]; を実行)だが、こちらは dxc.exe で -T cs_6_0 にコンパイル
// する(SM6.0 = DXIL)。既存の*_dxil.hlsl分離パターンに合わせて別ファイル
// とした。DXIL側の境界チェック付きチェーンが3項以上でも対応できるかを
// 検証するための新規シェーダー(2026-08-06)。

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
        float t1 = a[i] + b[i];
        float t2 = t1 * a[i];
        c[i] = t2 / b[i];
    }
}
