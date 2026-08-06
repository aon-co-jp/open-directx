// vector_sub_div_add_mul_chain4.hlsl と全く同じ契約・演算内容
// (RWStructuredBuffer<float>3本、256要素、numthreads(64,1,1)、
// t1=a-b; t2=t1/a; t3=t2+b; c=t3*a という4個の逐次2項演算チェーン、
// sub->div->add->mulという未検証の順序)だが、こちらはdxc.exeで
// -T cs_6_0にコンパイルする(SM6.0 = DXIL)。既存の
// `vector_add_mul_div_chain3_dxil.hlsl`と同じ分離パターン
// (レジスタ変数名もa/b/c/idに統一)。

RWStructuredBuffer<float> a : register(u0);
RWStructuredBuffer<float> b : register(u1);
RWStructuredBuffer<float> c : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    float t1 = a[id.x] - b[id.x];
    float t2 = t1 / a[id.x];
    float t3 = t2 + b[id.x];
    c[id.x] = t3 * a[id.x];
}
