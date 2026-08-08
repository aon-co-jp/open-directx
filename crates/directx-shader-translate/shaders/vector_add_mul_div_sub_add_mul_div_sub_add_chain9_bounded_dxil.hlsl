// 境界チェック付き9項チェーンのDXIL(SM6.0)版。DXBC版
// `vector_add_mul_div_sub_add_mul_div_sub_add_chain9_bounded.hlsl`と
// 同一契約・同一演算内容。

cbuffer Params : register(b0)
{
    uint ElementCount;
};

RWStructuredBuffer<float> InputA : register(u0);
RWStructuredBuffer<float> InputB : register(u1);
RWStructuredBuffer<float> Output : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 dtid : SV_DispatchThreadID)
{
    uint i = dtid.x;
    if (i < ElementCount)
    {
        float t1 = InputA[i] + InputB[i];
        float t2 = t1 * InputA[i];
        float t3 = t2 / InputB[i];
        float t4 = t3 - InputA[i];
        float t5 = t4 + InputB[i];
        float t6 = t5 * InputA[i];
        float t7 = t6 / InputB[i];
        float t8 = t7 - InputA[i];
        Output[i] = t8 + InputB[i];
    }
}
