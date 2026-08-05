// vector_sub_div_chain.hlsl と全く同じ契約・演算内容(RWStructuredBuffer<float>
// 3本、256要素、numthreads(64,1,1)、t = a-b; c = t/a という2演算チェーン)だが、
// こちらは dxc.exe で -T cs_6_0 にコンパイルする(SM6.0 = DXIL)。理由は
// vector_add_mul_chain_dxil.hlsl と同じ(DXBC/DXILのコンテナ構造差分だけを
// 見たいためソースを分離、レジスタ変数名も a/b/c/id に統一)。

RWStructuredBuffer<float> a : register(u0);
RWStructuredBuffer<float> b : register(u1);
RWStructuredBuffer<float> c : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    float t = a[id.x] - b[id.x];
    c[id.x] = t / a[id.x];
}
