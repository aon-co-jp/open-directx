// mulのnegateフラグ検証用シェーダー(2026-08-08、「次にすべきこと」の
// 未検証項目「mulのnegateフラグが立つケース」を埋める増分)。
// `Output[i] = InputA[i] * -InputB[i]`という、片方の入力を否定してから
// 乗算する式——fxcが`mul`命令のオペランドにnegate修飾子を付けるか、
// 別命令(`mov`で符号反転してから`mul`等)へ展開するかを実際に確認する
// ための最小シェーダー。

RWStructuredBuffer<float> InputA : register(u0);
RWStructuredBuffer<float> InputB : register(u1);
RWStructuredBuffer<float> Output : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 dtid : SV_DispatchThreadID)
{
    uint i = dtid.x;
    Output[i] = InputA[i] * (-InputB[i]);
}
