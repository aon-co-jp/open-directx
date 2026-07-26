// チェーンクラス(decode_chain_shape)へsub/div対応を追加するための新規
// シェーダー(D3D11 Compute Shader, SM5.0)。既存のvector_add_mul_chain.hlsl
// と同じUAV3本・InputAの多重参照パターンだが、演算をsub/divに変えた版。
// t = InputA[i] - InputB[i](1回目の演算)、Output[i] = t / InputA[i]
// (2回目の演算、tと`InputA[i]`の再読み込みで割る)。

RWStructuredBuffer<float> InputA : register(u0);
RWStructuredBuffer<float> InputB : register(u1);
RWStructuredBuffer<float> Output : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 dtid : SV_DispatchThreadID)
{
    uint i = dtid.x;
    float t = InputA[i] - InputB[i];
    Output[i] = t / InputA[i];
}
