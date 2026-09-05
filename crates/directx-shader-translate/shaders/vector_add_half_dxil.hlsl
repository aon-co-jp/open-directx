// vector_add_dxil.hlsl と同一契約(RWStructuredBuffer 3本、256要素、
// numthreads(64,1,1)、c = a + b)だが、要素型を`float`ではなく`half`
// (16-bit浮動小数点、HLSLのnative 16-bit types機能)にしたもの。
// SM6.2+ + `-enable-16bit-types`が必要(dxc.exeのドキュメント上、
// half/min16float等のnative 16-bit typesはSM6.2以降でのみ有効化できる)。
// TYPE_BLOCKでLLVM TYPE_CODE_HALF(=10)が実際にどう現れるかを実バイト列で
// 確認するための検証用シェーダー(2026-09-05、CLAUDE.md
// 「次にすべきこと(1)」への対応)。
RWStructuredBuffer<half> a : register(u0);
RWStructuredBuffer<half> b : register(u1);
RWStructuredBuffer<half> c : register(u2);

[numthreads(64, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    c[id.x] = a[id.x] + b[id.x];
}
