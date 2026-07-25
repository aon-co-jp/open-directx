// D3D11 Compute Shader (SM5.0)。vector_add.hlsl/vector_mul.hlslと同じ契約
// (RWStructuredBuffer 3本、256要素、1スレッド1要素、境界チェック無し)だが、
// 演算が除算(div)。SHEXデコーダをadd/mul/negated-add-as-sub(3演算)から、
// 4つ目の実オペコード(div)へ一般化するための実シェーダー。

RWStructuredBuffer<float> InputA : register(u0);
RWStructuredBuffer<float> InputB : register(u1);
RWStructuredBuffer<float> Output : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 dtid : SV_DispatchThreadID)
{
    uint i = dtid.x;
    Output[i] = InputA[i] / InputB[i];
}
