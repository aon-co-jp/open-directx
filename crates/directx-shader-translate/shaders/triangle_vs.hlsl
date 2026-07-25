// 最小の頂点シェーダー(vs_5_0): クリップ空間位置をそのままパススルーし、
// 色を頂点属性からピクセルシェーダーへ渡すだけ。単色三角形描画の最小構成。
// D3D11グラフィックスパイプライン着手の第一歩(open-directx CLAUDE.md
// 今回のタスク2)。ラスタライザ・出力マージ等の実装はスコープ外、
// あくまでDXBCバイト列としてVS/PSのSHEX命令列の形を確認する目的。
struct VsInput
{
    float3 position : POSITION;
    float4 color    : COLOR;
};

struct VsOutput
{
    float4 position : SV_POSITION;
    float4 color    : COLOR;
};

VsOutput main(VsInput input)
{
    VsOutput output;
    output.position = float4(input.position, 1.0);
    output.color = input.color;
    return output;
}
