// vector_add_mul_div_sub_add_mul_chain6_bounded.hlsl(DXBC/SM5.0、境界
// チェック付き6項演算チェーン)と全く同じ契約・演算内容(cbuffer(b0)で
// ElementCountを受け取り、if (i < ElementCount) の内側で t1=a[i]+b[i];
// t2=t1*a[i]; t3=t2/b[i]; t4=t3-a[i]; t5=t4+b[i]; c[i]=t5*a[i]; を実行)
// だが、こちらはdxc.exeで -T cs_6_0にコンパイルする(SM6.0 = DXIL)。
// 既存の*_dxil.hlsl分離パターンに合わせて別ファイルとした。DXIL側の
// 境界チェック付きチェーンが6項以上でも対応できるかを検証するための
// 新規シェーダー(2026-08-07)。

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
        float t3 = t2 / b[i];
        float t4 = t3 - a[i];
        float t5 = t4 + b[i];
        c[i] = t5 * a[i];
    }
}
