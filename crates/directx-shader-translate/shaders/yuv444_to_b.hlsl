// yuv_to_rgb(4:4:4)のBチャンネル: B = Y + 1.772*(U-128)
RWStructuredBuffer<float> Y : register(u0);
RWStructuredBuffer<float> U : register(u1);
RWStructuredBuffer<float> Output : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 dtid : SV_DispatchThreadID)
{
    uint i = dtid.x;
    Output[i] = Y[i] + 1.772 * (U[i] - 128.0);
}
