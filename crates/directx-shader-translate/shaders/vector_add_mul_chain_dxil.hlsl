// vector_add_mul_chain.hlsl と全く同じ契約・演算内容(RWStructuredBuffer<float>
// 3本、256要素、numthreads(64,1,1)、t = a+b; c = t*a という2演算チェーン)だが、
// こちらは dxc.exe で -T cs_6_0 にコンパイルする(SM6.0 = DXIL)。SM5.0/DXBC版
// (vector_add_mul_chain.hlsl, fxc.exeでコンパイル)と同じHLSLソースを使うと
// DXBC/DXILのコンテナ構造差分だけを見たいのにシェーダー内容の差分まで混ざる
// ため、vector_add_dxil.hlsl等の既存の分離パターンに合わせて別ファイルにする
// (レジスタ変数名も a/b/c/id に統一)。

RWStructuredBuffer<float> a : register(u0);
RWStructuredBuffer<float> b : register(u1);
RWStructuredBuffer<float> c : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    float t = a[id.x] + b[id.x];
    c[id.x] = t * a[id.x];
}
