// yuv_to_rgb(4:4:4、クロマサブサンプリング無し)のRチャンネル計算。
// BT.601: R = Y + 1.402 * (V - 128)
// Y/V/Output(R)はすべて同じ添字i(サブサンプリング無しなので全チャンネル
// 同解像度)で読み書きするため、既存のチェーンデコーダの制約
// (「全バッファがdtid.xで直接添字される」)にそのまま収まる。
// Uチャンネルは未使用(R計算には不要)だが、Y/U/V+3出力という将来の
// 本実装(G/Bも同時計算)へのUAVバインドポイント配置を揃えるため、
// このRチャンネル単体シェーダーではY=u0, V=u1, Output=u2 とする。

RWStructuredBuffer<float> Y : register(u0);
RWStructuredBuffer<float> V : register(u1);
RWStructuredBuffer<float> Output : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 dtid : SV_DispatchThreadID)
{
    uint i = dtid.x;
    Output[i] = Y[i] + 1.402 * (V[i] - 128.0);
}
