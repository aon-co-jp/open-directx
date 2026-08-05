// vector_add_mul_div_chain3.hlsl と全く同じ契約・演算内容(RWStructuredBuffer
// <float>3本、256要素、numthreads(64,1,1)、t1=a+b; t2=t1*a; c=t2/b という
// 3個の逐次2項演算チェーン)だが、こちらは dxc.exe で -T cs_6_0 にコンパイル
// する(SM6.0 = DXIL)。既存の`vector_add_mul_chain_dxil.hlsl`/
// `vector_sub_div_chain_dxil.hlsl`と同じ分離パターン(レジスタ変数名も
// a/b/c/idに統一)。

RWStructuredBuffer<float> a : register(u0);
RWStructuredBuffer<float> b : register(u1);
RWStructuredBuffer<float> c : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    float t1 = a[id.x] + b[id.x];
    float t2 = t1 * a[id.x];
    c[id.x] = t2 / b[id.x];
}
