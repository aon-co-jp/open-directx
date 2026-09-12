// InputA/InputB二本を読み、要素ごとの最大値をOutputへ書く。
// MED予測器の`min(left, top)`/`max(left, top)`部分と同じ形の
// 最小プロトタイプ(比較器/選択命令のデコード検証用)。
RWStructuredBuffer<float> InputA : register(u0);
RWStructuredBuffer<float> InputB : register(u1);
RWStructuredBuffer<float> Output : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    Output[id.x] = max(InputA[id.x], InputB[id.x]);
}
