// 最小のDirectCompute垂直スライス用シェーダー(D3D11 Compute Shader, SM5.0)。
// RWStructuredBuffer 2本を読み、要素ごとの和を3本目へ書く。
// open-cuda側 opencuda-directx/shaders/vector_add.hlsl (SM6.0/DXIL版) と
// 同じ契約(256要素、1スレッド1要素)にして、将来の数値比較を容易にする。

RWStructuredBuffer<float> InputA : register(u0);
RWStructuredBuffer<float> InputB : register(u1);
RWStructuredBuffer<float> Output : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 dtid : SV_DispatchThreadID)
{
    uint i = dtid.x;
    Output[i] = InputA[i] + InputB[i];
}
