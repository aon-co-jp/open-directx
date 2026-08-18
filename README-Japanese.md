# open-directx

> 📌 **最近の更新(2026-08-08、2Dスプライト描画プロトタイプ)**:
> ユーザー提案「dream-os/Linux上でopen-directx経由でGT730のPCでGAME…の
> 試作品を開発」を受け、まず2Dスプライト描画に絞って着手。テクスチャ
> サンプリング(`Texture2D.Sample`)初実装→複数スプライト/スプライト
> シート対応→ゲームループ(位置更新+跳ね返り物理)→**実ウィンドウ+
> 実Vulkanスワップチェーン+実キーボード入力**(`directx-graphics-window`
> クレート新設、ユーザー自身が実行し「パドルが動いて玉を弾き返した」と
> 目視確認済み)→アルファブレンド(半透明スプライト)→実PNGファイル
> からのテクスチャ読み込み、という一連の増分をすべてWindows実機
> (NVIDIA GT 730)、一部はLinux実機(WSL2 Ubuntu)でも検証した。次の候補:
> 複数の動くスプライト+衝突判定、ウィンドウリサイズ対応。詳細は
> [CLAUDE.md](CLAUDE.md)参照。
>
> *English*: Following the user's proposal to build game/mining/LLM
> prototypes for the GT 730 via open-directx on dream-os/Linux, started
> narrowly with a 2D sprite-rendering prototype: first texture sampling
> support, then multi-sprite/sprite-sheet support, a game loop
> (position update + bounce physics), a **real window + real Vulkan
> swapchain + real keyboard input** (new `directx-graphics-window`
> crate — the user ran it themselves and confirmed "the paddle moved
> and hit the ball back"), alpha blending, and loading textures from
> real PNG files. All verified on real Windows hardware (NVIDIA GT 730),
> some also on real Linux hardware (WSL2 Ubuntu). Next candidates:
> multiple moving sprites with collision detection, window resize
> support. See [CLAUDE.md](CLAUDE.md) for details.

> 📌 保留タスク(2026-08-06): 東芝SBM・DeepSeek技術の組み込み構想あり(dream-os等8リポジトリ対象)。詳細は[CLAUDE.md](CLAUDE.md)参照。

> 📌 **最近の更新(2026-08-08)**: 境界チェック付き7項チェーンについて
> DXBC側のみでDXIL側が無いという非対称を解消——`vector_add_mul_div_sub_
> add_mul_div_chain7_bounded_dxil.hlsl`を新規に`dxc.exe`で実コンパイルし、
> NVIDIA GT730実機でCPU参照実装との数値一致・境界チェックの動作を確認
> 済み(ワークスペース全体50単体テスト+実機テスト22本すべてgreen、
> 警告0件)。詳細は[CLAUDE.md](CLAUDE.md)参照。
>
> *English*: Closed the DXBC-only vs. DXIL-missing asymmetry for the
> boundary-checked 7-term chain — added and real-`dxc.exe`-compiled
> `vector_add_mul_div_sub_add_mul_div_chain7_bounded_dxil.hlsl`, verified
> on real NVIDIA GT730 hardware (matches the CPU reference, boundary
> check confirmed; full workspace: 50 unit tests + 22 real-hardware
> tests all green, zero warnings). See [CLAUDE.md](CLAUDE.md) for
> details.

> 📌 **最近の更新(2026-08-07)**: 境界チェック付きDXBC/DXILチェーンを
> 6項へ拡張し、NVIDIA GT730実機で動作確認。dream-os/open-cuda/
> aruaru-llmとの連携強化(SBM/DeepSeek移植等)を検討したが、既存の
> DXBC/DXILチェーン生成ロジックへの深い理解を伴わない拡張は数値的な
> 誤りを見逃すリスクがあると判断し、コード変更は行わず調査結果を
> [CLAUDE.md](CLAUDE.md)へ正直に記録した。
>
> *English*: Extended the boundary-checked DXBC/DXIL chain to 6 terms,
> verified on real NVIDIA GT730 hardware. Investigated deeper
> integration with dream-os/open-cuda/aruaru-llm (SBM/DeepSeek
> transplant) but decided against guessing extensions to the DXBC/DXIL
> chain logic without deep domain understanding — no code changed there,
> findings honestly recorded in [CLAUDE.md](CLAUDE.md).

> **Updated 2026-07-25**: The dev-policy file (`CLAUDE.md`) heading was
> renamed from "Development Policy & Dev Environment Rules" to
> "Design Philosophy & Development Policy & Dev Environment Rules",
> to more clearly separate the project's design philosophy (what we
> value), development policy (how we work), and dev environment rules
> (concrete operational conventions). See `CLAUDE.md` for details.


クロスプラットフォームのDirectX(D3D9/10/11/12)互換層——DXVK / vkd3d-proton
の思想を継承し、DXBC/DXILシェーダーバイトコードをSPIR-Vへ翻訳し、既存の
Vulkanコンピュートバックエンド([open-cuda](https://github.com/aon-co-jp/open-cuda)の
`opencuda-vulkan`)経由でディスパッチすることで、未改変のWindows DirectX
アプリケーションをLinux(将来的にはAndroid/macOS)上で動かすことを目指す。

設計上の根拠の全体、正直なスコープ/ロードマップ、セッションHANDOFF
ログについては[`CLAUDE.md`](CLAUDE.md)を参照——このREADMEは現時点の、
検証済みの状態のみを要約する。

### プラットフォーム&ベンダー対応表(2026-07-27追加、正直な開示)

DirectX自体はWindows/Xbox専用APIであり——ここでの「クロスプラット
フォーム」とは、DXBC/DXILバイトコードがSPIR-Vへ翻訳され、Vulkan経由で
ディスパッチされることを指し、これが実際に非Windowsプラットフォームへ
到達する部分である。本リポジトリ自身のコードには今日時点で`cfg(windows)`
等のプラットフォームゲートは一切存在せず(DXBCパーサー・SPIR-Vコード
生成・`directx-graphics-vulkan`はいずれも素の、プラットフォーム非依存な
Rust+`ash`実装)、そのためビルド/テストの移植性はVulkan自体の到達範囲に
従う:

| プラットフォーム | 経路 | 状態 |
|---|---|---|
| Windows | ネイティブVulkan | **実機で検証済み**(本リポジトリの開発機、NVIDIA GeForce GT 730) |
| Linux | ネイティブVulkan | 未改変でビルド/実行できるはず(ブロック要因となるWindows固有コードは存在しない)——**この環境の実Linuxマシンではまだ未検証** |
| Android | ネイティブVulkan | `open-cuda`が`aarch64-linux-android`クロスコンパイルの成功を検証済み(そのCLAUDE.md記載通り)。実機実行(実際の端末での`vkCreateInstance`)は依然として未着手 |
| macOS | [MoltenVK](https://github.com/KhronosGroup/MoltenVK)経由のVulkan(Metalへ変換) | 未着手——MoltenVKはネイティブVulkanではなく変換層であるため、Linux/Androidよりも保証が弱い |
| iOS / iPadOS(2026-08-17追加) | MoltenVK経由のVulkan(Metalへ変換) | 未着手。**macOSと同じMoltenVKの留保が適用される**——VulkanはiOS/iPadOS上ではネイティブに動作せず、この変換層経由のみのため、実機で実際に試すまではWindows/Vulkanネイティブ経路とのパリティは保証されない。また正式配布にはApple Developer Programが別途必要。 |
| 各種UNIX/BSD(2026-08-17追加) | ネイティブVulkan、おそらく | 未調査——Vulkan対応はディストリビューション/ドライバによって異なる。調査後はLinux経路の大半を再利用できると見込まれる |
| ソニー PlayStation 4/5/6/7 | 該当なし | 現時点で明示的にスコープ外——下記「PlayStationファミリー対象について」の注記および`CLAUDE.md`を参照 |
| Nintendo Switch 2 / 3(2026-08-17追加) | 該当なし | PlayStationと同じ「将来的な野心、公式SDK/NDA待ちで保留」という状態。**Switch 3は2026-08-17時点で任天堂から公式発表されていない——この記載は発表・発売された場合のためのプレースホルダーに過ぎず、実在の製品情報に基づくものではない。** |

GPUベンダー対応(PCIベンダーID照合、本リポジトリと`open-cuda`で一貫: NVIDIA
`0x10DE`、AMD `0x1002`/`0x1022`、Intel `0x8086`):

| ベンダー | 状態 |
|---|---|
| NVIDIA | **実機で検証済み**(GeForce GT 730) |
| AMD | ベンダーID照合コードは存在し型チェックも通るが、この環境では**実AMDハードウェアで一度も実行されていない**——未検証として扱うこと |
| Intel | AMDと同様: コードは存在するが**実Intel GPUハードウェアでは未検証** |

この3ベンダーIDを*検出可能*にするための修正は不要——コードは
`open-directx`/`opencuda-vulkan`/`opencuda-directx`間で既に正しく、
かつ同一である。不足しているのは、その経路を実際に実行するための実AMD/
Intelハードウェアであり、この開発環境には存在しない。

## 現状(2026-07-27、最新: グラデーション補間、GPUベンダー診断、チェーンのsub/div)

D3D11最小グラフィックスパイプラインと以下のDXBCチェーンクラス作業の上に
3つの増分が加わり、いずれもこのマシンの実NVIDIA GT 730で検証済み: (1)
`render_gradient_triangle_and_read_back` — グラフィックスパイプラインが
頂点ごとに異なる色を割り当てられるようになった(単色一様の縮退ケースの
みではなくなった)、実機読み戻しピクセルに対する単位分割の不変量チェック
で検証。(2) `enumerate_graphics_devices()` — `open-cuda`のCompute経路には
ベンダーID検出があるのにここのGraphics経路には無いという診断上の非対称
を解消。単独実装であり`opencuda-vulkan`への新規依存は無い。(3)
`decode_chain_shape`が`sub`/`div`に対応(以前は検証不能として明示的に
拒否していた)——新規シェーダー(`vector_sub_div_chain.hlsl`)を実際に
`fxc.exe`でコンパイルし、そのSHEXダンプで正確なオペランド順序を確認した
上で、実機でCPU参照実装と突き合わせてエンドツーエンド検証した。詳細は
`CLAUDE.md`のHANDOFF(2026-07-27付エントリ群)を参照。

## 現状(2026-07-25、最新: DXIL垂直スライスが実機で完成)

D3D12/DXILコンピュートシェーダー垂直スライスが、D3D11/DXBC側と完全な
パリティに到達した: `vector_add.dxil`(実`dxc.exe -T cs_6_0`出力)が
エンドツーエンドでデコードされ(コンテナ -> LLVMビットストリーム ->
型テーブル -> 命令列 -> 7個の`Call`レコードすべてが実際の`dx.op.*`意味へ
disambiguate)、実SPIR-V
(`directx_shader_translate::translate_dxil_vector_add_to_spirv`)へ翻訳
され、`tests/vector_add_dxil_real_vulkan.rs`がこのマシンの実NVIDIA
GT 730上でディスパッチし、CPU参照実装`a[i]+b[i]`と数値一致することを
検証する。これは依然として1つの既知シェーダー形状のみで、汎用SM6.0
デコーダではない——正確な境界については下記「未実装(正直なスコープ)」
を参照。SPIR-Vのワークグループサイズは、今やDXILの`METADATA_BLOCK`
(`dx.entryPoints` -> `ShaderProperties` -> `NumThreads`)から実際に抽出
されており、ハードコードではない——詳細は`CLAUDE.md`の2026-07-25「続き9」
HANDOFFエントリ、「続き7」はこの垂直スライスの元の達成内容を参照。

## 現状(2026-07-25、続き: DXILビットストリームレベルのパース+D3D11 VS/PSのDXBCパース)

下記フェーズ1コンピュートシェーダー垂直スライスの上に2つの新しい作業が
加わった:

- **DXIL(D3D12/SM6+) — 実バイト列をパース、コンテナ/ビットストリーム
  レベルのみ。** `crates/directx-shader-translate/src/dxil.rs`
  (`parse_dxil_container`)は、実`dxc.exe -T cs_6_0`コンパイル済みDXBC
  コンテナ(`shaders/vector_add_dxil.hlsl` -> `shaders/vector_add.dxil`、
  `tools/compile-dxbc-shaders.ps1`で生成)をパースする: `DXIL`チャンクの
  `DxilProgramHeader`/`DxilBitcodeHeader`(シェーダー種別、SM6.0、DXIL
  版数)を既存の`dxbc`クレート経由で抽出し、生のLLVMビットコード
  ペイロードを`llvm-bitcode`クレート(新規追加依存、DXIL固有の知識を
  持たない汎用LLVMビットストリームリーダー)へ渡してブロック/レコード
  木を実際にデコードする。実バイト列に対して確認できたこと: LLVM
  ラッパーマジック`BC\xC0\xDE`、単一のトップレベル`MODULE_BLOCK`
  (id 8)、その中の標準的なLLVMサブブロック —
  `TYPE_BLOCK_ID_NEW`(17)、`PARAMATTR_GROUP_BLOCK`(10)、
  `PARAMATTR_BLOCK`(9)、`CONSTANTS_BLOCK`(11)、`FUNCTION_BLOCK`
  (12、x5——`main`の基本ブロック数分)、`VALUE_SYMTAB_BLOCK`(14)、
  `METADATA_BLOCK`(15、x2)。**更新(2026-07-25、続き、D3D12track)**:
  型テーブル解決と粗い命令デコードがその後追加され
  (同ファイルの`resolve_type_table`/`decode_function_instructions`)、
  LLVMの文書化された`TYPE_BLOCK`/`FUNC_CODE`レコード表を実
  `vector_add.dxil`バイト列に適用——`Float`と
  `StructNamed{"class.RWStructuredBuffer<float>"}`を含む22エントリの
  型テーブル、および実際の命令列(`DeclareBlocks -> Call*5 ->
  ExtractValue -> Call -> ExtractValue -> BinOp -> Call -> Ret`)を確認
  した。**更新(2026-07-25、続き6)**: 7個の`Call`レコードすべてが
  disambiguateされた。`resolve_vector_add_dxil_calls`が
  `VALUE_SYMTAB_BLOCK`の関数名(`Record::take_payload()`で取得——
  `fields()`ではない、前回エントリの当該クレート理解における実際の
  ギャップだった)を解決し、LLVMの相対値オペランドエンコーディングを
  手デコード(実バイト列に対して手検証済み)することで、
  `[CreateHandle{range_id:2}, CreateHandle{range_id:1},
  CreateHandle{range_id:0}, ThreadId, BufferLoad{handle_range_id:0},
  BufferLoad{handle_range_id:1}, BufferStore{handle_range_id:2}]`が
  得られる。DXILオペコード番号(`CreateHandle`=57、`BufferLoad`=68、
  `BufferStore`=69、`ThreadId`=93)は記憶に頼らずWeb検索でMicrosoftの
  `DirectXShaderCompiler/docs/DXIL.rst`と突き合わせて確認し、実際に
  デコードされた定数と完全に一致した。**依然としてDXIL→SPIR-V翻訳は
  存在しない**——それが次の増分。下記「未実装」を参照。
- **D3D11グラフィックスパイプライン — VS/PS向けの実SPIR-V生成に到達・
  検証済み、ラスタライザ/描画はまだ無い。** `shaders/triangle_vs.hlsl`/
  `shaders/triangle_ps.hlsl`(最小のパススルー頂点+ピクセルシェーダー
  ペア、`POSITION`/`COLOR`入力、`SV_POSITION`/`SV_TARGET`出力)を実
  `fxc.exe /T vs_5_0`/`/T ps_5_0`でコンパイル。`parse_dxbc`は無改修で
  両方をパースできる。`spirv_gen::translate_vertex_shader`/
  `translate_pixel_shader`(新規)は実際の、固定のSHEXオペコード列
  (VSは`dcl_input`x2/`dcl_output_siv`/`dcl_output`/`mov`x3/`ret`、PSは
  `dcl_input_ps`(linear)/`dcl_output`/`mov`/`ret`)をデコードし、実際の
  グラフィックスSPIR-Vを生成する: `OpEntryPoint Vertex`/`Fragment`
  (`GLCompute`ではない)、`Location`デコレーション付きの`Input`/
  `Output`ストレージクラス変数、頂点シェーダーの`SV_POSITION`出力への
  `BuiltIn Position`、フラグメントシェーダーへの`OpExecutionMode ...
  OriginUpperLeft`。2通りの方法で検証: (1) `rspirv`自身のローダーが
  生成バイト列をエラーなく再パースできる、(2) 実Vulkan SDKの
  `spirv-val.exe`(`C:\VulkanSDK\1.4.350.0\Bin\spirv-val.exe`)を両方の
  生成モジュールに対して実行し、両方とも診断無しで終了コード0を返した。
  `translate_shader`/`translate_chain_shader`(コンピュート専用)は
  引き続き両シェーダーを正しく拒否する。**ラスタライザ・フレーム
  バッファ・実際のVulkan描画コマンドは存在しない**——`opencuda-vulkan`
  は(実ソースを読んで確認した結果)`VkGraphicsPipelineCreateInfo`/
  レンダーパス/フレームバッファ関連コードを一切持たず、コンピュート
  ディスパッチ専用であるため、実際にレンダリングされたピクセルは今回の
  パスのスコープ外である。正直なマイルストーン境界の全体は`CLAUDE.md`
  のHANDOFFを参照。

## 現状(2026-07-26、D3D11最小グラフィックスパイプラインのマイルストーン到達)

新規クレート`crates/directx-graphics-vulkan`が`ash`をこのワークスペースの
**直接**依存として追加(`opencuda-vulkan`の上に重ねるのではない——同
クレートはソース監査によりコンピュートディスパッチ専用であることを
確認済み)。実際のレンダーパス・フレームバッファ・
`VkGraphicsPipelineCreateInfo`を実装し、既に`spirv-val`を通過済みの、
上記`translate_vertex_shader`/`translate_pixel_shader`が生成したSPIR-Vを
そのまま再利用する(シェーダー翻訳は再実装しない)。
`render_uniform_triangle_and_read_back`はフルビューポートの「大きい
三角形」を単一の一様頂点色で描画し、ホスト可視ステージングバッファ経由で
レンダリング済み画像を読み戻す。実機テスト
(`crates/directx-graphics-vulkan/tests/triangle_real_vulkan.rs`)は、
このマシンに存在する実NVIDIA GT 730上で、読み戻したすべてのピクセルが
パススルー頂点色と一致することをアサートする(`cargo test -p
directx-graphics-vulkan --test triangle_real_vulkan -- --nocapture`:
1件成功)。スコープは意図的に狭い: 固定のシェーダーペア1組、描画
コマンド1回、深度バッファ/テクスチャ/スワップチェーン/複数三角形の
補間検証は無し。詳細な正直な開示は`CLAUDE.md`のHANDOFF(2026-07-26続き)
を参照。

## 現状(2026-07-25、フェーズ1垂直スライスを既知シェーダー3個へ一般化)

`crates/directx-shader-translate`は、**3つの特定の既知シェーダー**
(`vector_add.hlsl`、`vector_mul.hlsl`、`vector_sub_bounded.hlsl`)に
ついて、完全な垂直スライス(DXBCパース -> 狭いSM5.0オペコードサブセット
デコード -> SPIR-Vコード生成〈`rspirv`経由〉-> 実Vulkanディスパッチ
〈`open-cuda`の`opencuda-vulkan`〉-> CPU参照実装との数値一致)をこの
マシンの実NVIDIA GT 730で検証済み。**依然として汎用SM5.0-to-SPIR-V
デコーダではない**——下記「未実装」を参照。

- `parse_dxbc`(フェーズ0): DXBCコンテナ/チャンクの内観(RDEF/ISGN/
  OSGN/SHEXの有無)、元のフロントエンドから無変更。
- `spirv_gen::translate_shader`(フェーズ1、2026-07-25に一般化):
  `fxc.exe`が実際に出力する3つのオペコード形状を認識、共通骨格
  (`dcl_globalFlags` -> オプション`dcl_constantbuffer` -> 3x
  `dcl_uav_structured` -> `dcl_input` -> `dcl_temps` ->
  `dcl_thread_group` -> オプション`ult`+`if` -> 2x`ld_structured` ->
  `add`/`mul` -> `store_structured` -> オプション`endif` -> `ret`)を
  共有:
  - `vector_add.hlsl`: `add`、境界チェック無し。
  - `vector_mul.hlsl`: `add`の代わりに`mul`。
  - `vector_sub_bounded.hlsl`: 第1ソースオペランドに`negate`フラグを
    持つ`add`(実`fxc.exe`出力をダンプして確認——`fxc`は`a - b`を専用の
    `sub`オペコードではなく`add dest, -b, a`へ最適化する)、加えて実際の
    `if (id.x < N)`境界チェック(定数バッファに対する`ult`+`if`/
    `endif`)。生成されたSPIR-Vはこれを実際の`OpSelectionMerge`/
    `OpBranchConditional`で実装し、push constantの`n`を比較に使う。
  上記以外のオペコード/形状はすべて`SpirvGenError::UnsupportedShader`で
  拒否され、黙って誤翻訳されることはない。UAVバインドポイント・
  スレッドグループサイズ・演算子・境界チェックの有無は、いずれも実際に
  パースされたDXBCから抽出され、ハードコードされていない。
  `translate_vector_add_shader`は`translate_shader`への薄い後方互換
  エイリアスとして残されている。
- `tests/vector_add_real_vulkan.rs`、`tests/vector_mul_real_vulkan.rs`、
  `tests/vector_sub_bounded_real_vulkan.rs`: それぞれ翻訳済みSPIR-Vを
  `open-cuda`の実`opencuda-vulkan::VulkanDevice`(`ash`、`real-vulkan`
  フィーチャ)経由でディスパッチし、256要素についてGPU出力をCPU参照
  実装と突き合わせる(誤差1e-3/1e-2)。境界チェックテストはさらに320
  スレッドをディスパッチしつつ論理要素数256で、要素256..320が(センチネル
  値のまま)決して書き込まれないことをアサートすることで、生成SPIR-Vの
  `if (id.x < N)`分岐が単にコンパイルが通るだけでなく、実際に実行を
  ゲートしていることを証明する。
- `examples/dump_shex.rs`: 実SHEXオペコード列を確認するための小さな
  スタンドアロンツール
  (`cargo run -p directx-shader-translate --example dump_shex --
  <path.dxbc>`)。このセッション中、デコーダ対応を書く前に使用した。
  今後のオペコード単位での一般化作業のために残してある。

**このセクションのタイトルが書かれてから**、4つ目の単一演算シェーダー
(`vector_div.hlsl`、単純な`div`)が、既存とまったく同じパターンに従って
`translate_shader`に追加され——さらに最近では、それとは種類の異なる
パターンクラス`spirv_gen::translate_chain_shader`が(置き換えではなく)
並行して追加された: 単一の固定演算ではなく、実際の逐次2項演算の
レジスタ式木をデコードする(制御フロー無し)。これは新規にコンパイル
したシェーダーで検証されており、その実SHEXは1つの一時レジスタの
コンポーネントをfxcのCSEにより使い回していた(追加の一時レジスタを
宣言する代わりに)。現在の全体像は`CLAUDE.md`の2026-07-25「続き9」
HANDOFFエントリを参照(このセクションは2026-07-25日中の状態についての
歴史的な正確性を保つため、当時のまま残してある)。

## ビルド&テスト

```powershell
cargo build --workspace
cargo test --workspace -- --nocapture
```

### 実際に何かが描画されるところを見る(2026-07-27追加)

このリポジトリは`fn main`を持たないライブラリ群であるため、テストの
ソースを読むのではなく、自分のGPU上でグラフィックスパイプラインが
動くところを*見る*最速の方法は次の通り:

```bash
cargo run -p directx-graphics-vulkan --example render_triangle
```

これは`tests/triangle_real_vulkan.rs`と同じ、実fxc.exeコンパイル済み
DXBC→SPIR-V翻訳シェーダーを再利用し、実Vulkanハードウェア上に
グラデーション(赤/緑/青)三角形を描画し、フレームバッファを読み戻して
`render_triangle.ppm`(プレーンPPM、追加の画像クレート依存不要——例えば
`magick render_triangle.ppm render_triangle.png`で変換するか、多くの
画像ビューアで直接開ける)へ書き込む。利用可能なVulkanデバイス/ドライバ
が存在しない場合は、成功を装うことなく正直なエラーを表示して非ゼロで
終了する。

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

DXIL型テーブル/命令デコード作業(2026-07-25、続き、D3D12track)の後、
`cargo test --workspace`は合計23件のテスト(19単体+4実Vulkan統合)を
実行し、すべて成功する。既存20件に加えて3件が新規: `dxil::tests::
resolves_real_dxil_type_table_and_finds_float_and_resource_struct`、
`dxil::tests::decodes_real_dxil_function_block_into_matching_vector_
add_shape`、`dxil::tests::shape_matcher_honestly_rejects_unexpected_
instruction_orderings`。

HLSLからDXBCフィクスチャを再生成するには(Windows SDKの`fxc.exe`が
必要——`dxc.exe`はDXIL/SM6+専用でDXBCは出力できない点に注意):

```powershell
pwsh tools/compile-dxbc-shaders.ps1
```

## 未実装(正直なスコープ)

- **汎用SM5.0命令デコード。** 上記3つのオペコード形状のみ対応。それ以外の
  D3D11コンピュートシェーダー(異なるリソースレイアウト、他の制御フロー、
  他の組み込み関数、複数の境界チェック、negated-addではなく実オペコード
  としての`div`/`sub`等)は、誤翻訳されるのではなく拒否される。真の汎用
  デコーダを構築する(または既存のもの、例えば`dxbc-spirv`/`dxil-spirv`
  のアプローチをより詳しく研究して採用/移植する)ことが、実際の次の
  マイルストーンとして残っている。
- **DXIL(Shader Model 6+、D3D12): `vector_add.dxil`垂直スライスは実機で
  エンドツーエンドに完成した——ただし依然としてこの1つの既知シェーダー
  形状のみで、汎用SM6.0ではない。** `dxil.rs`の`resolve_type_table`/
  `decode_function_instructions`/`resolve_vector_add_dxil_calls`は、
  実際の`TYPE_BLOCK`/`FUNCTION_BLOCK`/`VALUE_SYMTAB_BLOCK`レコードを
  LLVMの文書化されたコードに照らしてデコードし、7個の`Call`レコード
  すべてを実際の`dx.op.*`意味(`CreateHandle`/`ThreadId`/`BufferLoad`/
  `BufferStore`、UAVバインドポイント付き)へdisambiguateする。
  `translate_dxil_vector_add_to_spirv`(新規)はそのdisambiguate済み出力を
  `spirv_gen.rs`の共有`emit_spirv_for_kernel`(DXBC側の`emit_spirv`から
  切り出され、両バックエンドが1つのコードパスから発行するようになった)へ
  渡して実SPIR-Vを生成し、`tests/vector_add_dxil_real_vulkan.rs`がこの
  マシンの実NVIDIA GT 730上で`opencuda-vulkan`経由でディスパッチし、
  256要素すべてでCPU参照実装`a[i]+b[i]`と一致することを検証する——DXBC側
  の`vector_add`テストと同じ厳密さ。**ワークグループサイズは今や実際に
  抽出されており、ハードコードではない**: `extract_numthreads_from_
  metadata`(`dxil.rs`)は実際の`METADATA_BLOCK`経路`dx.entryPoints` ->
  エントリポイントごとのタプル -> `ShaderProperties` -> `kDxilNumThreadsTag`
  (=4、Microsoft`DirectXShaderCompiler`の`DxilMetadataHelper.h`/`.cpp`
  ソースと突き合わせて確認)を辿り、`{x,y,z}`ノードをモジュールの実際の
  値リストに対して解決し、`vector_add.dxil`の実バイト列から`(64,1,1)`を
  得る——前回エントリで既知だったハードコードは解消され、合成回帰テストが
  異なるメタデータを与えると抽出ロジックが実際に*異なる*値を返すことを
  証明している(「何を与えても常に64,1,1を返すだけ」ではないことの
  証明)。それ以外のオペコード/オペランド形状(異なる演算、複数の基本
  ブロック、境界チェック)は依然として拒否され、誤翻訳されない。D3D12の
  コマンドリスト/ディスクリプタヒープ/ルートシグネチャ対応(シェーダー
  翻訳の上位レイヤー)は未着手のまま。
- **DXBCデコーダは4つの固定単一演算形状を超えて一般化された: 現在は
  逐次2項演算のチェーン(制御フローなし)を、実際のレジスタ式木経由で
  扱える。5番目のハードコード形状ではない。** `spirv_gen::
  translate_chain_shader`/`decode_chain_shape`は`ld_structured`/`add`/
  `mul`/`store_structured`を走査し、(一時レジスタ, コンポーネント)を
  キーとする実際の式木を構築する。そのため1演算・2演算・N演算のいずれも
  同じ方法で扱える——新規にコンパイルした実シェーダー
  (`vector_add_mul_chain.hlsl`、`t = A[i]+B[i]; Out[i] = t*A[i]`)で
  検証済み。その実SHEXは1つの一時レジスタの`.x`/`.y`コンポーネントを
  使い回していた(fxcが繰り返しの`A[i]`ロードを再発行せずCSEした)——これは
  予測しておらず、木ベースのデコーダが追加のケース無しで扱えた本物の
  発見だった。実NVIDIA GT 730上でディスパッチし、CPU参照実装
  `(a[i]+b[i])*a[i]`と突き合わせて検証済み。チェーン内の`sub`/`div`は
  意図的に依然として拒否されている(オペランド順序の意味論は単一演算
  ケースでしか検証していないため)。元の4つの単一演算形状は無変更で、
  引き続きそのまま通る。
- **D3D11グラフィックスパイプライン: VS/PS向けのDXBCコンテナパースは
  動作確認済みだが、SPIR-Vコード生成・ラスタライザ・画面への実際の
  三角形描画は無い。** フルパイプライン(ラスタライザ・テクスチャ
  サンプラ・ブレンドステート・出力マージ)は引き続きスコープ外——
  `spirv_gen`の狭いオペコード形状デコーダを`dcl_output_siv`/
  `dcl_input_ps`/補間モードを理解できるよう拡張することも同様。
- PlayStationファミリー対象——明示的にスコープ外。法務/利用規約上の
  理由については`CLAUDE.md`を参照。

## 関連プロジェクト

- [open-cuda](https://github.com/aon-co-jp/open-cuda) — 本プロジェクトが
  ディスパッチ先として設計されているVulkanコンピュート実行バックエンド
  (`opencuda-core::GpuDevice`、`KernelSource::SpirV`)。また、無関係だが
  既に動作している`opencuda-directx`クレートも含む——こちらは**Windows
  上でネイティブに**D3D12を実行するもので、本プロジェクト(DirectX
  シェーダーを**非Windows対象で**実行する)とは逆方向。
- [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm) — 本プロジェクト
  との直接の技術的依存関係は無い(推測ではなく確認済み)。
