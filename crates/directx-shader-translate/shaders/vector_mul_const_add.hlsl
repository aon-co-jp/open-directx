// RegExpr::Immediate(即値定数)対応の実機検証用シェーダー(2026-09-12)。
// make-diskのyuv_to_rgb変換係数(1.402等)のような定数をチェーン式に
// 含められることを、実際のfxc.exe出力で確認するためのもの。
// Output[i] = InputA[i] * 1.402 + InputB[i]
// (掛け算の片方が即値定数1.402、という最小構成)。

RWStructuredBuffer<float> InputA : register(u0);
RWStructuredBuffer<float> InputB : register(u1);
RWStructuredBuffer<float> Output : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 dtid : SV_DispatchThreadID)
{
    uint i = dtid.x;
    float t = InputA[i] * 1.402;
    Output[i] = t + InputB[i];
}
