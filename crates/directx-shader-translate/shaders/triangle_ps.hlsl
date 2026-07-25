// 最小のピクセルシェーダー(ps_5_0): 頂点シェーダーから受け取った補間済み
// COLORをそのまま出力するだけ(単色三角形描画の最小構成、
// triangle_vs.hslと対になる)。
struct PsInput
{
    float4 position : SV_POSITION;
    float4 color    : COLOR;
};

float4 main(PsInput input) : SV_TARGET
{
    return input.color;
}
