// vector_max.hlslと対になる最小値版(MED予測器のmin(left,top)部分の検証用)。
RWStructuredBuffer<float> InputA : register(u0);
RWStructuredBuffer<float> InputB : register(u1);
RWStructuredBuffer<float> Output : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    Output[id.x] = min(InputA[id.x], InputB[id.x]);
}
