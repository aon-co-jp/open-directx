// FFv1のMED予測器、実画像2次元インデックス版(x-1,y-1の近傍参照)。
// 境界(x==0またはy==0)は単純にleft/top/topleftを自分自身の値で
// 埋める(FFv1本来の境界規約とは異なる簡略化、比較器+2次元インデックス
// 計算のデコード検証が目的)。
RWStructuredBuffer<float> Image : register(u0);
RWStructuredBuffer<float> Output : register(u1);

cbuffer Params : register(b0)
{
    uint Width;
    uint Height;
};

[numthreads(64, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    uint i = id.x;
    if (i < Width * Height)
    {
        uint x = i % Width;
        uint y = i / Width;
        float center = Image[i];
        float left = (x > 0) ? Image[y * Width + (x - 1)] : center;
        float top = (y > 0) ? Image[(y - 1) * Width + x] : center;
        float topleft = (x > 0 && y > 0) ? Image[(y - 1) * Width + (x - 1)] : center;

        float pred;
        if (topleft >= max(left, top)) {
            pred = min(left, top);
        } else if (topleft <= min(left, top)) {
            pred = max(left, top);
        } else {
            pred = left + top - topleft;
        }
        Output[i] = pred;
    }
}
