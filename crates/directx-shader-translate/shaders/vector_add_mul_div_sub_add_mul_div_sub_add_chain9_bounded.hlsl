// 境界チェック付きチェーンの9項への拡張(2026-08-08、8項からの続き)。
// `vector_add_mul_div_sub_add_mul_div_sub_chain8_bounded.hlsl`(境界
// チェック付き、8項まで検証済み: add->mul->div->sub->add->mul->div->sub)
// にもう1個演算(add)を追加した。

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
