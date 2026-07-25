// D3D11 Compute Shader (SM5.0)。vector_add.hlsl と同じ契約(RWStructuredBuffer
// 3本、256要素、1スレッド1要素)だが、演算が加算(add)ではなく乗算(mul)。
// SHEXデコーダを「add専用」から「addとmulの両方」へ一般化するための2本目の
// 実シェーダー。

RWStructuredBuffer<float> InputA : register(u0);
RWStructuredBuffer<float> InputB : register(u1);
RWStructuredBuffer<float> Output : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 dtid : SV_DispatchThreadID)
{
    uint i = dtid.x;
    Output[i] = InputA[i] * InputB[i];
}
