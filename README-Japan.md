# open-directx (日本語版)

DXVK/vkd3d-protonと同じ方向性の、クロスプラットフォームDirectX
(D3D9/10/11/12)互換層。Windows専用のDirectXアプリを、DXBC/DXILシェーダー
バイトコードをSPIR-Vへ翻訳し、[open-cuda](https://github.com/aon-co-jp/open-cuda)の
`opencuda-vulkan`(Vulkan Compute実行基盤)経由でディスパッチすることで、
Linux(将来的にAndroid/macOS)上で実際に動かすことを目指す。

設計の背景・正直なスコープ/ロードマップ・セッション引き継ぎ記録の全文は
[`CLAUDE.md`](CLAUDE.md)を参照。このREADMEは現状の**検証済みの部分だけ**
を要約する。

## 現状(2026-07-25、フェーズ0→フェーズ1垂直スライス着手中)

**DXBCコンテナ/チャンク解析のフロントエンドのみ**実装済み。SPIR-Vコード
生成・実Vulkanディスパッチは**まだ未実装**——動作すると仮定しないこと。

- `crates/directx-shader-translate`: `fxc.exe`が生成した実際のDXBC
  コンテナ(Shader Model 5.1以下、D3D9/10/11が使用)を解析する。低レベル
  のチャンクテーブル/RDEF/ISGN/OSGN/SHEX解析は、車輪の再発明を避けるため
  既存の[`dxbc`クレート](https://crates.io/crates/dxbc)(crates.io、MIT、
  実シェーダー1000件超での往復検証済み)を再利用し、その上に将来の
  SPIR-Vコード生成バックエンドへ渡すための`ShaderModule`要約を薄く
  かぶせている。
- `crates/directx-shader-translate/shaders/vector_add.hlsl`: 最小の
  D3D11コンピュートシェーダー(`RWStructuredBuffer`によるベクトル加算、
  SM5.0)——フェーズ1垂直スライス(DXBC→SPIR-V→`opencuda-vulkan`
  ディスパッチ→CPU参照実装との数値一致)の対象。今回はDXBC解析の半分
  だけが完了、翻訳・ディスパッチは今後の課題。
- `crates/directx-shader-translate/shaders/vector_add.dxbc`: 実際に
  コンパイル済みのDXBCバイト列(956バイト)、
  `fxc.exe /T cs_5_0 /E main vector_add.hlsl`で生成、テストフィクスチャ
  としてコミット済み(再生成は`tools/compile-dxbc-shaders.ps1`参照)。

## ビルド・テスト

```powershell
cargo build --workspace
cargo test --workspace
```

実際に観測した出力(2026-07-25):

```
running 3 tests
test tests::rejects_garbage_bytes_that_are_not_a_dxbc_container ... ok
test tests::rejects_truncated_dxbc_header ... ok
test tests::parses_real_fxc_compiled_vector_add_dxbc_container ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

DXBCフィクスチャをHLSLから再生成する場合(Windows SDK付属の`fxc.exe`が
必要——`dxc.exe`はDXIL/SM6+専用でDXBCは出力できない点に注意):

```powershell
pwsh tools/compile-dxbc-shaders.ps1
```

## 未実装(正直な開示)

- DXBC→SPIR-Vコード生成(命令列そのものの翻訳)。フェーズ1の残作業の
  大部分。
- DXIL(Shader Model 6+、D3D12)の解析・翻訳——D3D11コンピュートの垂直
  スライスがエンドツーエンドで動くまでスコープ外。
- `opencuda-vulkan`の`GpuDevice`/`KernelSource::SpirV`ディスパッチ経路
  への実配線——概念設計のみ(`PORTING.md`参照)、コードは未着手。
- フルグラフィックスパイプライン(ラスタライザ・テクスチャサンプラ・
  ブレンドステート)——コンピュート版垂直スライスが動くまでスコープ外。
- PlayStationファミリー対応——法務・利用規約上の懸念から明示的に
  スコープ外(詳細は`CLAUDE.md`)。

## 関連プロジェクト

- [open-cuda](https://github.com/aon-co-jp/open-cuda) — 本プロジェクトが
  ディスパッチ先として設計しているVulkan Compute実行基盤
  (`opencuda-core::GpuDevice`、`KernelSource::SpirV`)。同リポジトリには
  無関係の`opencuda-directx`クレート(WindowsでD3D12を**ネイティブに**
  実行する既存実装)も存在するが、これは本プロジェクト(DirectXを**他OS
  上で**動かす)とは逆方向。
- [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm) — 本プロジェクト
  との直接の技術的依存関係は無い(実際に確認済み、推測ではない)。
