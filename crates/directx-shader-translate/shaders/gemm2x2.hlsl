// 最小GEMM(行列積)コンピュートシェーダー: 固定サイズ2x2 * 2x2 = 2x2。
// open-directx Phase 1 垂直スライスの既存アーキテクチャ(制御フロー無し、
// ld_structuredで読み込み-> 演算 -> store_structuredで書き込み)方針を踏襲し、
// 動的ループ(loop/endloop命令)を使わず、K=2を完全アンロールしたdot積として書く。
// 2次元ディスパッチ(numthreads(2,2,1))を使い、行番号iと列番号jをそれぞれ
// SV_DispatchThreadID.y / .xから直接得ることで、1次元化のための除算・剰余
// (fxcがbfi/and命令列へ最適化してしまい既存デコーダの想定外になる)を避ける。
//
// バッファレイアウト(row-major):
//   A: 2x2 行列(4要素) register(u0)
//   B: 2x2 行列(4要素) register(u1)
//   C: 2x2 行列(4要素、出力) register(u2)
// C[i][j] = A[i][0]*B[0][j] + A[i][1]*B[1][j]  (i,j in {0,1})
RWStructuredBuffer<float> A : register(u0);
RWStructuredBuffer<float> B : register(u1);
RWStructuredBuffer<float> C : register(u2);

[numthreads(2, 2, 1)]
void main(uint3 dtid : SV_DispatchThreadID)
{
    uint j = dtid.x;
    uint i = dtid.y;
    float a0 = A[i * 2 + 0];
    float a1 = A[i * 2 + 1];
    float b0 = B[0 * 2 + j];
    float b1 = B[1 * 2 + j];
    C[i * 2 + j] = a0 * b0 + a1 * b1;
}
