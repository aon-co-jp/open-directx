# open-directx (日本語版)

DXVK/vkd3d-protonと同じ方向性の、クロスプラットフォームDirectX
(D3D9/10/11/12)互換層。Windows専用のDirectXアプリを、DXBC/DXILシェーダー
バイトコードをSPIR-Vへ翻訳し、[open-cuda](https://github.com/aon-co-jp/open-cuda)の
`opencuda-vulkan`(Vulkan Compute実行基盤)経由でディスパッチすることで、
Linux(将来的にAndroid/macOS)上で実際に動かすことを目指す。

設計の背景・正直なスコープ/ロードマップ・セッション引き継ぎ記録の全文は
[`CLAUDE.md`](CLAUDE.md)を参照。このREADMEは現状の**検証済みの部分だけ**
を要約する。

## 現状(2026-07-25、1シェーダー限定でフェーズ1垂直スライス達成)

`crates/directx-shader-translate`が、**1つの既知シェーダー
(`vector_add.hlsl`)限定**でエンドツーエンドの垂直スライスを達成した:
DXBC解析 → 狭いSM5.0オペコード部分集合のデコード → SPIR-Vコード生成
(`rspirv`使用) → 実Vulkanディスパッチ(`open-cuda`の`opencuda-vulkan`)
→ CPU参照実装との数値一致——このマシンの実NVIDIA GT 730で検証済み。
**汎用SM5.0→SPIR-Vデコーダではない**点に注意(詳細は下記「未実装」節)。

- `parse_dxbc`(フェーズ0): DXBCコンテナ/チャンク解析(RDEF/ISGN/OSGN/
  SHEXの存在確認)、既存部分から変更なし。
- `spirv_gen::translate_vector_add_shader`(フェーズ1、2026-07-25新規):
  `fxc.exe`が`vector_add.hlsl`に対して実際に生成する狭いオペコード列
  (`dcl_uav_structured`x3 + `dcl_thread_group` + `ld_structured`x2 +
  `add` + `store_structured` + `ret`)だけを認識し、それ以外は
  `SpirvGenError::UnsupportedShader`で明示的に拒否する(誤翻訳しない)。
  UAVバインドポイント・スレッドグループサイズは実際にパースしたDXBCから
  抽出したものであり、決め打ちではない。`rspirv::dr::Builder`で実際の
  SPIR-Vモジュールを組み立てる。
- `tests/vector_add_real_vulkan.rs`: 翻訳したSPIR-Vを`open-cuda`の実
  `opencuda-vulkan::VulkanDevice`(`ash`、`real-vulkan`フィーチャ)経由で
  ディスパッチし、CPU参照実装`a[i]+b[i]`(256要素、誤差1e-3以内)と
  数値一致することを検証する。

## ビルド・テスト

```powershell
cargo build --workspace
cargo test --workspace --release -- --nocapture
```

実際に観測した出力(2026-07-25、このマシン、NVIDIA GeForce GT 730):

```
running 5 tests
test spirv_gen::tests::rejects_garbage_bytes_honestly_instead_of_pretending_to_translate ... ok
test tests::rejects_truncated_dxbc_header ... ok
test tests::parses_real_fxc_compiled_vector_add_dxbc_container ... ok
test tests::rejects_garbage_bytes_that_are_not_a_dxbc_container ... ok
test spirv_gen::tests::translates_real_fxc_compiled_vector_add_dxbc_to_valid_spirv ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 1 test
device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
OK: DXBC(fxc.exe実コンパイル)->SPIR-V(自前生成)->実Vulkan(NVIDIA GT 730)経路が、CPU参照実装(a[i]+b[i])と256要素すべてで数値一致した
c[0]=128, c[255]=255.5
test dxbc_vector_add_matches_cpu_reference_on_real_vulkan_hardware ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s
```

DXBCフィクスチャをHLSLから再生成する場合(Windows SDK付属の`fxc.exe`が
必要——`dxc.exe`はDXIL/SM6+専用でDXBCは出力できない点に注意):

```powershell
pwsh tools/compile-dxbc-shaders.ps1
```

## 未実装(正直な開示)

- **汎用SM5.0命令デコード。** `vector_add.hlsl`の狭いオペコード形状
  以外は全て拒否される(誤翻訳ではなく明示的なエラー)。真の汎用デコーダ
  の実装(または既存実装、例えば`dxbc-spirv`/`dxil-spirv`のアプローチの
  より深い調査・移植)が引き続き本当の次のマイルストーン。
- DXIL(Shader Model 6+、D3D12)の解析・翻訳——汎用SM5.0デコードが
  終わるまでスコープ外。
- フルグラフィックスパイプライン(ラスタライザ・テクスチャサンプラ・
  ブレンドステート)——それまでスコープ外。
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
