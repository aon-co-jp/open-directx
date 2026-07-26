// vector_add.hlsl と全く同じ契約(RWStructuredBuffer<float> 3本、
// 256要素、numthreads(64,1,1))だが、こちらは dxc.exe で -T cs_6_0
// にコンパイルする(SM6.0 = DXIL)。SM5.0/DXBC版(vector_add.hlsl,
// fxc.exeでコンパイル)と同じHLSLソースを使ってしまうと、DXBC/DXILの
// コンテナ構造差分だけを見たいのにシェーダー内容の差分まで混ざるため、
// あえて別ファイルとして分離し、中身は意図的に同一にしている。
RWStructuredBuffer<float> a : register(u0);
RWStructuredBuffer<float> b : register(u1);
RWStructuredBuffer<float> c : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    c[id.x] = a[id.x] - b[id.x];
}
