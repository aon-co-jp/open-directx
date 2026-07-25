# open-directx (日本語版)

> **2026-07-25 更新**: 開発方針ファイル(`CLAUDE.md`)の見出しを
> 「開発方針＆開発環境ルール」から「設計思想＆開発方針＆開発環境ルール」
> へ改名しました。プロジェクトの設計思想(何を大事にしているか)・
> 開発方針(どう進めるか)・開発環境ルール(具体的な運用規約)を明確に
> 区別して記載しています。詳細は`CLAUDE.md`を参照してください。


DXVK/vkd3d-protonと同じ方向性の、クロスプラットフォームDirectX
(D3D9/10/11/12)互換層。Windows専用のDirectXアプリを、DXBC/DXILシェーダー
バイトコードをSPIR-Vへ翻訳し、[open-cuda](https://github.com/aon-co-jp/open-cuda)の
`opencuda-vulkan`(Vulkan Compute実行基盤)経由でディスパッチすることで、
Linux(将来的にAndroid/macOS)上で実際に動かすことを目指す。

設計の背景・正直なスコープ/ロードマップ・セッション引き継ぎ記録の全文は
[`CLAUDE.md`](CLAUDE.md)を参照。このREADMEは現状の**検証済みの部分だけ**
を要約する。

## 現状(2026-07-25続き: DXILのbitstreamレベルパース + D3D11 VS/PSのDXBCパース)

下記フェーズ1(Compute Shader垂直スライス)に加え、今回2つ着手した:

- **DXIL(D3D12/SM6+)——実バイト列をbitstream/ブロックレベルまで実際に
  パース。** `crates/directx-shader-translate/src/dxil.rs`
  (`parse_dxil_container`)が、実`dxc.exe -T cs_6_0`でコンパイルした
  DXBCコンテナ(`shaders/vector_add_dxil.hlsl` →
  `shaders/vector_add.dxil`、`tools/compile-dxbc-shaders.ps1`で生成)を
  解析する: 既存の`dxbc`クレートで`DXIL`チャンクの`DxilProgramHeader`/
  `DxilBitcodeHeader`(シェーダー種別・SM6.0・DXIL版数)を取り出し、
  中の生LLVM bitcodeを新規依存の`llvm-bitcode`クレート(DXIL固有知識を
  持たない汎用LLVM bitstreamリーダー)に渡してブロック/レコード木を
  実際にデコードする。実バイト列で確認できたこと: LLVMラッパーマジック
  `BC\xC0\xDE`、トップレベルの`MODULE_BLOCK`(id=8)が1個、その中に
  標準LLVMサブブロック——`TYPE_BLOCK_ID_NEW`(17)・
  `PARAMATTR_GROUP_BLOCK`(10)・`PARAMATTR_BLOCK`(9)・
  `CONSTANTS_BLOCK`(11)・`FUNCTION_BLOCK`(12、`main`の基本ブロックの数
  だけ5個)・`VALUE_SYMTAB_BLOCK`(14)・`METADATA_BLOCK`(15、2個)。
  **更新(2026-07-25続き5、D3D12track)**: この後、型テーブル解決と
  命令列の大分類デコードを追加した(同ファイルの`resolve_type_table`/
  `decode_function_instructions`)。LLVM公式の`TYPE_BLOCK`/`FUNC_CODE`
  レコードコード表を実`vector_add.dxil`に当てはめ、22個の型
  (`Float`・`StructNamed{"class.RWStructuredBuffer<float>"}`含む)と、
  実際の命令列(`DeclareBlocks -> Call`x5` -> ExtractValue -> Call ->
  ExtractValue -> BinOp -> Call -> Ret`)を得た。**それでもDXIL→SPIR-V
  変換は無い**——DXILは組み込み演算をすべて通常のLLVM`Call`として表現
  するため、`VALUE_SYMTAB_BLOCK`(関数名解決)と相対値参照オペランドの
  デコードが無い現状では7個の`Call`を区別できない(詳細は下記
  「未実装」節)。
- **D3D11グラフィックスパイプライン——DXBCパースのみ、SPIR-V無し。**
  `shaders/triangle_vs.hlsl`/`shaders/triangle_ps.hlsl`(最小のパス
  スルー頂点+ピクセルシェーダーの組、`POSITION`/`COLOR`入力→
  `SV_POSITION`/`SV_TARGET`出力)を実`fxc.exe /T vs_5_0`/`/T ps_5_0`で
  コンパイル。既存の`parse_dxbc`(コンテナレベルのみ、無改修)が両方とも
  問題なくパースできることを確認——同じDXBCコンテナ/チャンクの
  フロントエンドがCompute Shaderだけでなくグラフィックスシェーダーにも
  そのまま使えることの実証。`examples/dump_shex.rs`で実SHEX命令列を
  ダンプし、オペコード/オペランド語彙が実際にCompute Shaderとは異なる
  ことを確認した: `dcl_input`/`dcl_input_ps`(`linear`補間付き)/
  `dcl_output`/`dcl_output_siv`(`SV_POSITION`)/`mov`——
  `dcl_uav_structured`・`ld_structured`/`store_structured`・
  `dcl_thread_group`は一切出現しない。`translate_shader`
  (Compute専用)はVS/PSいずれも`SpirvGenError::UnsupportedShader`で
  正しく拒否する(誤った翻訳を試みない、新規テストで確認済み)。SPIR-V
  コード生成・ラスタライザ・実Vulkanでの三角形描画は今回のスコープ外
  (`CLAUDE.md`のHANDOFF参照)。

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

DXIL型テーブル/命令列デコード追加分(2026-07-25続き5、D3D12track)を
含めると、`cargo test --workspace`は合計23件(単体テスト19件+実Vulkan
統合テスト4件)が全て成功する。既存20件に加え新規3件:
`dxil::tests::resolves_real_dxil_type_table_and_finds_float_and_
resource_struct`・`dxil::tests::decodes_real_dxil_function_block_into_
matching_vector_add_shape`・`dxil::tests::shape_matcher_honestly_
rejects_unexpected_instruction_orderings`。

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
- **DXIL(Shader Model 6+、D3D12)の解析・翻訳——型テーブルと命令列の
  大分類までは実装済み、DXIL→SPIR-V変換は未着手。** `dxil.rs`の
  `resolve_type_table`/`decode_function_instructions`が、実
  `vector_add.dxil`のTYPE_BLOCK/FUNCTION_BLOCKをLLVM公式のレコード
  コード表に基づいて実際にデコードする。**それでもSPIR-V変換に届かない
  理由**: DXILは`CreateHandle`/`ThreadId`/`BufferLoad`/`BufferStore`を
  全て通常のLLVM`Call`として表現するため、`VALUE_SYMTAB_BLOCK`
  (関数名解決)とLLVM bitcodeの相対値参照オペランドのデコードが無い
  現状では、どの`Call`がどの組み込みかを区別できず、UAVバインド
  ポイントも取り出せない。次にやるべきことはこの2点。
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
