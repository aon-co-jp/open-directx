# open-directx (日本語版)

DXVK/vkd3d-protonと同じ方向性の、クロスプラットフォームDirectX
(D3D9/10/11/12)互換層。Windows専用のDirectXアプリを、DXBC/DXILシェーダー
バイトコードをSPIR-Vへ翻訳し、[open-cuda](https://github.com/aon-co-jp/open-cuda)の
`opencuda-vulkan`(Vulkan Compute実行基盤)経由でディスパッチすることで、
Linux(将来的にAndroid/macOS)上で実際に動かすことを目指す。

設計の背景・正直なスコープ/ロードマップ・セッション引き継ぎ記録の全文は
[`CLAUDE.md`](CLAUDE.md)を参照。このREADMEは現状の**検証済みの部分だけ**
を要約する。

## 現状(2026-07-25、3つの既知シェーダーへ一般化してフェーズ1垂直スライス達成)

`crates/directx-shader-translate`が、**3つの既知シェーダー**
(`vector_add.hlsl`/`vector_mul.hlsl`/`vector_sub_bounded.hlsl`)で
エンドツーエンドの垂直スライスを達成した: DXBC解析 → 狭いSM5.0オペコード
部分集合のデコード → SPIR-Vコード生成(`rspirv`使用) → 実Vulkanディス
パッチ(`open-cuda`の`opencuda-vulkan`) → CPU参照実装との数値一致——
このマシンの実NVIDIA GT 730で検証済み。**依然として汎用SM5.0→SPIR-V
デコーダではない**点に注意(詳細は下記「未実装」節)。

- `parse_dxbc`(フェーズ0): DXBCコンテナ/チャンク解析(RDEF/ISGN/OSGN/
  SHEXの存在確認)、既存部分から変更なし。
- `spirv_gen::translate_shader`(フェーズ1、2026-07-25に一般化):
  `fxc.exe`が実際に生成する3つのオペコード形状(いずれも共通の骨格
  `dcl_globalFlags` → (`dcl_constantbuffer`?) → `dcl_uav_structured`x3 →
  `dcl_input` → `dcl_temps` → `dcl_thread_group` → (`ult`+`if`?) →
  `ld_structured`x2 → `add`/`mul` → `store_structured` → (`endif`?) →
  `ret`)を認識する:
  - `vector_add.hlsl`: `add`、境界チェック無し。
  - `vector_mul.hlsl`: `add`の代わりに`mul`。
  - `vector_sub_bounded.hlsl`: `add`の第1ソースオペランドに`negate`
    フラグが立った形(実`fxc.exe`出力で確認済み——`fxc`は`a - b`を専用の
    `sub`オペコードではなく`add dest, -b, a`へ最適化する)+実際の
    `if (id.x < N)`境界チェック(定数バッファとの`ult`＋`if`/`endif`)。
    生成するSPIR-Vもこの境界チェックを実際の`OpSelectionMerge`/
    `OpBranchConditional`として実装し、push constantの`n`を比較に使う。
  上記以外のオペコード・形状は`SpirvGenError::UnsupportedShader`で
  明示的に拒否する(誤翻訳しない)。UAVバインドポイント・スレッドグループ
  サイズ・演算種別・境界チェックの有無は、いずれも実際にパースしたDXBC
  から抽出したものであり、決め打ちではない。`translate_vector_add_shader`
  は後方互換のため`translate_shader`への薄いエイリアスとして残している。
- `tests/vector_add_real_vulkan.rs`・`tests/vector_mul_real_vulkan.rs`・
  `tests/vector_sub_bounded_real_vulkan.rs`: それぞれ翻訳したSPIR-Vを
  `open-cuda`の実`opencuda-vulkan::VulkanDevice`(`ash`、`real-vulkan`
  フィーチャ)経由でディスパッチし、CPU参照実装(256要素、誤差1e-3/1e-2
  以内)と数値一致することを検証する。境界チェックテストはさらに、
  論理要素数256に対し320スレッドをディスパッチし、256..320の範囲が
  一切書き込まれず(センチネル値のまま)残ることをassertすることで、
  生成したSPIR-Vの`if (id.x < N)`分岐が単にコンパイルが通るだけでなく
  実際に実行をゲートしていることを実証している。
- `examples/dump_shex.rs`: 実SHEXオペコード列を人間に見える形でダンプ
  する小さな独立ツール(`cargo run -p directx-shader-translate --example
  dump_shex -- <path.dxbc>`)。今回のセッションでデコーダへ対応を追加する
  前に「実際に何が出てくるか」を確認するために使った。今後のオペコード別
  一般化作業のために残す。

## ビルド・テスト

```powershell
cargo build --workspace
cargo test --workspace -- --nocapture
```

実際に観測した出力(2026-07-25、このマシン、NVIDIA GeForce GT 730):

```
running 7 tests
test spirv_gen::tests::rejects_garbage_bytes_honestly_instead_of_pretending_to_translate ... ok
test tests::rejects_garbage_bytes_that_are_not_a_dxbc_container ... ok
test tests::rejects_truncated_dxbc_header ... ok
test tests::parses_real_fxc_compiled_vector_add_dxbc_container ... ok
test spirv_gen::tests::translates_real_fxc_compiled_vector_mul_dxbc_to_valid_spirv ... ok
test spirv_gen::tests::translates_real_fxc_compiled_vector_add_dxbc_to_valid_spirv ... ok
test spirv_gen::tests::translates_real_fxc_compiled_vector_sub_bounded_dxbc_to_valid_spirv ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

running 1 test
device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
OK: DXBC(fxc.exe実コンパイル)->SPIR-V(自前生成)->実Vulkan(NVIDIA GT 730)経路が、CPU参照実装(a[i]+b[i])と256要素すべてで数値一致した
c[0]=128, c[255]=255.5
test dxbc_vector_add_matches_cpu_reference_on_real_vulkan_hardware ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s

running 1 test
device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
OK: DXBC(fxc.exe実コンパイル, mul)->SPIR-V(自前生成)->実Vulkan経路が、CPU参照実装(a[i]*b[i])と256要素すべてで数値一致した
c[0]=64, c[255]=6.625
test dxbc_vector_mul_matches_cpu_reference_on_real_vulkan_hardware ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.59s

running 1 test
device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
OK: DXBC(fxc.exe実コンパイル, sub+境界チェック)->SPIR-V(自前生成)->実Vulkan経路が、CPU参照実装(a[i]-b[i])と有効範囲256要素すべてで数値一致し、境界外の64要素はセンチネル値のまま(書き込まれなかった)ことを確認した
c[0]=10, c[255]=137.5, c[319]=-1
test dxbc_vector_sub_bounded_matches_cpu_reference_and_respects_bounds_on_real_vulkan_hardware ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
```

`cargo clippy --workspace --all-targets`: 警告0件。

DXBCフィクスチャをHLSLから再生成する場合(Windows SDK付属の`fxc.exe`が
必要——`dxc.exe`はDXIL/SM6+専用でDXBCは出力できない点に注意):

```powershell
pwsh tools/compile-dxbc-shaders.ps1
```

## 未実装(正直な開示)

- **汎用SM5.0命令デコード。** 上記3つのオペコード形状以外は全て拒否
  される(誤翻訳ではなく明示的なエラー)。真の汎用デコーダの実装
  (または既存実装、例えば`dxbc-spirv`/`dxil-spirv`のアプローチのより
  深い調査・移植)が引き続き本当の次のマイルストーン。
- **DXIL(Shader Model 6+、D3D12)の解析・翻訳——2026-07-25にコンテナ
  レベルの調査のみ実施、実装は未着手。** DXILはLLVM 3.7時代のbitcode
  で、DXBCと同じ外枠のDXContainer形式の中に`DXIL`パートとして格納
  される(ProgramHeader + BitcodeHeader + シリアライズされたLLVM IR
  モジュール、マジック値`0x4C495844`)。LLVM公式ドキュメントが今では
  このコンテナ形式とネイティブLLVM DXILバックエンドのアーキテクチャを
  記載している(`llvm.org/docs/DirectX/DXContainer.html`・
  `.../DXILArchitecture.html`)——これは以前この件を調査した時点より
  新しい・より公式な情報源である。今後着手する場合の候補となるRust側の
  部品: 本プロジェクトが既に使っている`dxbc`クレートは`DXIL`チャンクを
  不透明バイト列として保持するのみ(デコードしない)。bitcode層自体を
  扱う汎用`llvm-bitcode`クレートがcrates.ioに存在する。このリポジトリ
  ではDXILバイト列を一切パースしていない——この節はPhase 0のDXBC
  コンテナ調査と同じ深さの、コンテナ形式レベルの調査結果のみである。
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
