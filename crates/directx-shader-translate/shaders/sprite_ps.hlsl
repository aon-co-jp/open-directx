// 2026-08-08 2Dスプライト描画プロトタイプ第一歩: テクスチャサンプリング
// 付きピクセルシェーダー(D3D11、SM5.0)。`sprite_vs.hlsl`が渡すUV座標で
// `SpriteTex`をサンプルし、そのまま出力する(パススルーのcolor出力を
// テクスチャサンプル結果に置き換えた形)。

Texture2D SpriteTex : register(t0);
SamplerState SpriteSampler : register(s0);

float4 main(float4 pos : SV_POSITION, float2 uv : TEXCOORD) : SV_TARGET
{
    return SpriteTex.Sample(SpriteSampler, uv);
}
