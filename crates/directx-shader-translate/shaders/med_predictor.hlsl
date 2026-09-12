// FFv1のMED(median edge detector)予測器。実際の画像2次元インデックス
// (x-1, y-1等)は今回のプロトタイプでは対象外とし、既存のyuv444_to_rgb系
// と同様にleft/top/topleftをあらかじめ切り出した3本の独立バッファとして
// 受け取る単純化を行う(比較器+分岐ロジック自体の検証が目的)。
RWStructuredBuffer<float> Left : register(u0);
RWStructuredBuffer<float> Top : register(u1);
RWStructuredBuffer<float> TopLeft : register(u2);
RWStructuredBuffer<float> Output : register(u3);

[numthreads(64, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    float left = Left[id.x];
    float top = Top[id.x];
    float topleft = TopLeft[id.x];
    float pred;
    if (topleft >= max(left, top)) {
        pred = min(left, top);
    } else if (topleft <= min(left, top)) {
        pred = max(left, top);
    } else {
        pred = left + top - topleft;
    }
    Output[id.x] = pred;
}
