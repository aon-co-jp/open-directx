// 2026-08-08 2Dスプライト描画プロトタイプ第一歩: テクスチャ付き矩形
// (スプライト)用の最小頂点シェーダー(D3D11、SM5.0)。既存の
// `triangle_vs.hlsl`(COLORパススルー)と同じ構造だが、COLORの代わりに
// TEXCOORD(UV座標)をパススルーする。

struct VSInput
{
    float3 pos : POSITION;
    float2 uv : TEXCOORD;
};

struct VSOutput
{
    float4 pos : SV_POSITION;
    float2 uv : TEXCOORD;
};

VSOutput main(VSInput input)
{
    VSOutput o;
    o.pos = float4(input.pos, 1.0);
    o.uv = input.uv;
    return o;
}
