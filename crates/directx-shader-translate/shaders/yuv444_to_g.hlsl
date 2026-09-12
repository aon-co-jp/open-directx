// yuv_to_rgb(4:4:4)のGチャンネル: G = Y - 0.344136*(U-128) - 0.714136*(V-128)
RWStructuredBuffer<float> Y : register(u0);
RWStructuredBuffer<float> U : register(u1);
RWStructuredBuffer<float> V : register(u2);
RWStructuredBuffer<float> Output : register(u3);

[numthreads(64, 1, 1)]
void main(uint3 dtid : SV_DispatchThreadID)
{
    uint i = dtid.x;
    Output[i] = Y[i] - 0.344136 * (U[i] - 128.0) - 0.714136 * (V[i] - 128.0);
}
