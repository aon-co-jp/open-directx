// 任意形状GEMM(行列積)コンピュートシェーダー: C(MxN) = A(MxK) * B(KxN)。
//
// 既存の`gemm2x2.hlsl`は「2x2固定・K=2完全アンロール・制御フロー無し」という
// 垂直スライス専用だったため、任意形状(例: LLM推論の1x768x50257)には使えな
// かった。このシェーダーはその制約を外すために新規追加したもので、
// **動的ループ(`loop`/`endloop`/`breakc`命令)と定数バッファ(`cb0`)による
// 実行時サイズ指定**を使う。
//
// バッファレイアウト(row-major):
//   A: MxK 行列 register(u0)
//   B: KxN 行列 register(u1)
//   C: MxN 行列(出力) register(u2)
//   Params: M, K, N を実行時に与える定数バッファ register(b0)
//     (SPIR-V側では`open-cuda`の`matmul`カーネル契約に合わせて
//      12バイトのpush constant `{uint m; uint k; uint n;}`へ写像する)
//
// C[i][j] = sum_{k=0}^{K-1} A[i*K+k] * B[k*N+j]   (i = dtid.y, j = dtid.x)
//
// ディスパッチグリッドは切り上げになるため、境界チェック(i<M && j<N)が必須。
cbuffer Params : register(b0)
{
    uint M;
    uint K;
    uint N;
};

RWStructuredBuffer<float> A : register(u0);
RWStructuredBuffer<float> B : register(u1);
RWStructuredBuffer<float> C : register(u2);

[numthreads(8, 8, 1)]
void main(uint3 dtid : SV_DispatchThreadID)
{
    uint j = dtid.x;
    uint i = dtid.y;
    if (i >= M || j >= N)
    {
        return;
    }

    float acc = 0.0;
    for (uint k = 0; k < K; ++k)
    {
        acc += A[i * K + k] * B[k * N + j];
    }
    C[i * N + j] = acc;
}
