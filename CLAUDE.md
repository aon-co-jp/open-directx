# 設計思想＆開発方針＆開発環境ルール(open-directx)

作業ドライブは`F:\runo`。この節は[`open-raid-z`](https://github.com/aon-co-jp/open-raid-z)の
`CLAUDE.md`を正本とし、各プロジェクトへコピーして同期する方針に準じる。
GitHubリポジトリ: [aon-co-jp/open-directx](https://github.com/aon-co-jp/open-directx)。

**開発開始日: 2026-07-25**(このリポジトリへの実装着手日。GitHub上の空リポジトリ
自体は2026-07-01に作成済みだった)。

## このプロジェクトの役割

Windows専用APIである**DirectX(D3D9/10/11/12)で書かれた既存のアプリ/ゲームを、
実際にLinux・Android・将来的にはmacOS・PlayStationファミリーでも動かす**ことを
目指す、クロスプラットフォームDirectX互換層。

**2026-07-25、ユーザーの明確な選択により「真の逆方向互換層」(Windows DirectX
バイナリ/シェーダーを他OSでそのまま動かす)を本気で目指す方針で開始した**
(過去の代替案「Vulkanを共通基盤にしつつ各プラットフォームへDirectX風のAPIを
提供する」は不採用)。

## 技術的な位置づけの訂正(2026-07-25、重要)

2026-07-23時点の調査(`open-cuda`側CLAUDE.md HANDOFF参照)では「DXVK/
vkd3d-proton/MoltenVK等はいずれもDirectX→Vulkan/Metalという一方向の変換のみで、
逆方向(DirectXを他OSでそのまま動かす)の実例は無く技術的に筋が悪い」と評価して
いたが、これは**軸の混同だった**と訂正する。

- DXVK/vkd3d-proton(Valve社Proton、Steam上のLinux版DirectXゲーム互換の基盤
  技術)・MoltenVK経由のCrossOver/Whisky(macOS)は、**「DirectX(Windows専用API)
  で書かれた既存の実バイナリ・実ゲームを、そのままLinux/macOS上で動かす」という、
  まさにユーザーが求めている逆方向互換をすでに実現している実例**である。
- 「DirectX API呼び出しをVulkan API呼び出しへ変換する」という**変換の方向**と、
  「Windows向けDirectXアプリが実際にLinux/macOS上で動くかどうか」という
  **エンドユーザー体験の方向**は別の軸であり、前者がVulkan向けであっても
  後者は正しく「DirectXを他OSで動かす」を達成している。
- したがって、**Vulkanを内部の実行基盤として使うことと、真の逆方向互換層を
  目指すことは矛盾しない**——DXVK等がまさにその実例。本プロジェクトも
  同じアプローチ(D3D API呼び出しのインターセプト+DXBC/DXIL シェーダーの
  実行時翻訳→Vulkan実行)を採用する。

## スコープと正直なロードマップ

**フェーズ0(現在、設計・調査段階)**:
- DXBC/DXIL(DirectXシェーダーバイトコード形式)の構造調査。
- 既存OSS実装(DXVK・vkd3d-proton・[dxil-spirv](https://github.com/HansKristian-Work/dxil-spirv)
  ——vkd3d-protonが実際に使うDXIL→SPIR-V変換ツール、[SPIRV-Cross](https://github.com/KhronosGroup/SPIRV-Cross)、
  [naga](https://github.com/gfx-rs/naga)(wgpu系のシェーダー翻訳基盤))の
  アーキテクチャ調査——車輪の再発明を避け、実績のある設計判断を参考にする。
- 現実的なMVPスコープの切り出し: **フルグラフィックスパイプライン
  (ラスタライザ・テクスチャサンプラ・ブレンドステート等)は当面スコープ外**。
  まずは**D3D11 Compute Shader(DirectCompute)のディスパッチのみ**を対象とした
  垂直スライス(1つのシンプルなコンピュートシェーダーが、実際にDXBC/DXILから
  SPIR-Vへ翻訳され、`open-cuda`の`opencuda-vulkan`経由でVulkan実行され、
  CPU参照実装と数値一致することを実証する)から始める。グラフィックス
  パイプライン対応は、この垂直スライスが実際に動いてから次のフェーズとして
  着手する。

**フェーズ1以降(未着手)**:
1. D3D11 Compute Shader垂直スライス(DXBC/DXIL→SPIR-V翻訳+Vulkanディスパッチ)。
2. D3D11の最小グラフィックスパイプライン(頂点/ピクセルシェーダー+基本的な
   ラスタライズ)。
3. D3D12対応(コマンドリスト・ディスクリプタヒープ・ルートシグネチャ)。
4. Android対応(Vulkan自体はAndroidネイティブ対応のため、Linux版の資産を
   大部分再利用できる見込み——ただしWin32/COM層のエミュレーション
   〈Wine相当〉が必要になる可能性が高く、その場合はWineプロジェクト自体との
   連携・流用を検討する)。
5. macOS対応(MoltenVK経由、CrossOver/Whiskyと同じアプローチ)。

**PlayStation 4/5/6/7対応について(正直な開示、2026-07-25時点の判断)**:
ユーザーの構想には含まれているが、**技術的難易度だけでなく法務・利用規約上の
懸念が技術的難易度とは別次元で存在する**——PlayStationの開発SDKは非公開・
NDA対象であり、非公式なリバースエンジニアリングは各種利用規約・法律
(DMCA等)に抵触するリスクがある。本プロジェクトでは、PS4-7対応は
**「将来的な野心」としてロードマップに明記するに留め、現時点では設計・実装の
対象に含めない**。着手する場合は、法的なリスク評価を別途行った上で、
ユーザーへ改めて確認してから判断する。

## ベースにするプロジェクト(ユーザー指示、2026-07-25)

- **[open-cuda](https://github.com/aon-co-jp/open-cuda)**: `opencuda-vulkan`
  (Vulkan Compute実行基盤、実機〈NVIDIA GT 730〉検証済み)をシェーダー実行
  バックエンドとして利用する。`opencuda-core::GpuDevice`抽象(alloc/memcpy/
  launch_kernel)をそのまま再利用し、DXBC/DXIL→SPIR-V変換後のカーネルを
  `KernelSource::SpirV`として渡す設計を想定(詳細はopen-cuda側
  `opencuda-core`のAPIを要確認)。`opencuda-directx`(Windows専用D3D12
  バックエンド、Phase 1&2実装済み)とは**別物**——`opencuda-directx`は
  「WindowsでDirectXを直接叩く」実装、本プロジェクトは「DirectXを
  他OSで動かす」実装であり、方向性が逆である点に注意。
- **[aruaru-llm](https://github.com/aon-co-jp/aruaru-llm)**: 現時点では
  直接の技術的依存関係は無い(aruaru-llmはLLM推論サービス、本プロジェクトは
  グラフィックスAPI互換層のため)。ユーザー指示にaruaru-llmが「ベース」として
  挙がっている意図の詳細(共通の「分身の術」サービス化パターンを踏襲する、
  という意味である可能性が高い——`TenantRegistry`的な管理APIパターンを
  本プロジェクトの何らかの管理面〈翻訳キャッシュサーバー等〉に適用する等)は
  未確認。次回具体的な統合ポイントが判明次第、この節を更新する。

## 開発方針(全リポジトリ共通、要約)

- Rust実装を基本とする。`windows`クレート(Windows API)・Vulkanバインディング
  (`ash`等、`opencuda-vulkan`が使用しているクレートに合わせる)を用いる。
- 「型チェック・ビルド成功のみで完了と報告しない」——実際にDXBC/DXILの
  実バイトコードを翻訳し、実Vulkan実行し、CPU参照実装と数値一致することを
  実機で確認してから「動作した」と報告する(このエコシステム全体の徹底方針)。
- 未実装・スタブの機能は「対応している」という誤ったシグナルを出さない
  (`opencuda-directx`の`supports_dxil()`パターンを踏襲)。
- 新規ファイル・新規クレート追加、新規リポジトリ作成等、命名・配置判断を
  伴う場面は着手前にユーザーへ確認する(2026-07-23の教訓、
  `open-raid-z`側CLAUDE.md参照)。

## HANDOFF

- **2026-07-25 リポジトリ着手**: 空だったGitHubリポジトリ
  (`aon-co-jp/open-directx`、2026-07-01作成)へ、設計方針・スコープ・
  ロードマップを記載したこの`CLAUDE.md`を新規作成。まだコードは一切
  実装していない(次回セッションでフェーズ0の調査〈DXBC/DXIL構造・
  dxil-spirv/SPIRV-Cross/nagaのアーキテクチャ〉から着手する)。
  - 次にすべきこと: (1) DXBC/DXILバイトコード形式の構造調査(公式仕様は
    非公開だが、Microsoft`DirectXShaderCompiler`〈dxcソースコード〉・
    `dxil-spirv`のソースコードから逆算的に理解する)、(2) 依存クレート
    選定(DXBCパーサーを自作するか、既存のRust製クレート
    〈`dxbc`クレート等〉が実在し使えるか日英検索で調査)、(3) 最小の
    D3D11 Compute Shader垂直スライスの設計・実装着手。

- **2026-07-25(続き) フェーズ0調査完了・DXBCコンテナ解析クレートを実装、実fxc.exe出力で3テストgreen**:
  1. **調査結果**:
     - **DXBC構造**: マジック`b"DXBC"` + 16バイトハッシュ + version + total_size + chunk_count のヘッダ、続くチャンクオフセットテーブル、各チャンクは`fourcc(4)+size(4)+payload`。標準チャンク: `RDEF`(リソース定義/定数バッファ)・`ISGN`/`OSGN`(入出力シグネチャ)・`SHEX`/`SHDR`(命令列本体)・`STAT`(統計)。LLVM公式ドキュメントDXContainerページとRust`dxbc`クレートの実装で裏取り。
     - **DXIL構造**: LLVM bitcodeベース(Shader Model 6+、D3D12用)。DXBCコンテナの`DXIL`チャンク内にLLVM IRとして埋め込まれる。今回は構造把握のみ、パース実装はスコープ外(D3D11 DXBCが先)。
     - **dxil-spirv(vkd3d-protonが実際に使うDXIL→SPIR-V変換器)**: LLVM C++ APIの自前サブセット実装でLLVM bitcodeを直接パースし、IR生成→CFG構造化→SPIR-Vモジュール出力という段階的パイプライン。重要な発見: dxil-spirvは「レガシーDXBCシェーダーも別サブプロジェクト`dxbc-spirv`との統合で扱う」とREADMEに明記されており、DXIL専用ではなくDXBCも視野に入れた設計。
     - **既存Rustクレート調査(車輪の再発明回避)**: crates.io/lib.rsで`dxbc`クレート(v0.1.0、2026-03公開、MIT、coconutbird)を発見。RDEF/ISGN/OSGN/SHEX含む18種のチャンク型をフルパースでき、fxc.exe実出力1000件超での往復検証済みと説明されている。**採用**(自作DXBCパーサーは実装しない、このクレートに依存する)。DXILはこのクレートも`DXIL`チャンクを不透明なLLVM bitcodeとして保持するのみでデコードはしない(今回のスコープと一致、問題なし)。
     - **SPIRV-Cross/naga**: 参考程度の調査に留めた(SPIR-Vを別IRへ"逆"変換する用途が主、本プロジェクトが必要とする「DXBC/DXIL→SPIR-V」の順方向変換には直接使えない)。
     - **aruaru-llmとの技術的接続の結論**: `open-cuda`側CLAUDE.mdの2026-07-23付HANDOFFで既に「直接の技術的依存関係は無い」と結論済みであり、本セッションで`aruaru-llm/CLAUDE.md`を読んだ限りでもopencuda-vulkan/opencuda-core等への直接依存は見当たらなかった(`TenantRegistry`の「分身の術」パターンは管理API一般の設計思想であって、DirectX互換層固有の技術的接続点ではない)。正直に「無し」と再確認する。
  2. **実装**: Cargoワークスペース新設(`Cargo.toml`)。`crates/directx-shader-translate`クレート: `dxbc`クレート(依存)をラップした`parse_dxbc(bytes) -> Result<ShaderModule, TranslateError>`(RDEF/ISGN/OSGN存在有無・SHEX命令数を要約する薄い構造体)。`crates/directx-shader-translate/shaders/vector_add.hlsl`(D3D11 Compute Shader、SM5.0、`RWStructuredBuffer`3本のベクトル加算、`opencuda-directx`の`vector_add.hlsl`〈SM6.0/DXIL版〉と同じ256要素契約で将来の比較を容易にした)。
  3. **fxc.exeの実在確認**: このマシンに`dxc.exe`(VulkanSDK同梱、DXIL/SM6+専用)と`fxc.exe`(Windows SDK同梱、`C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\fxc.exe`、DXBC/SM<=5.1専用)の両方が存在することを確認。**重要な訂正**: 当初の想定`dxc --version`だけでは不十分で、DXBC(D3D9/10/11向け)を得るには`fxc.exe`が必須(`dxc.exe`はDXIL専用でDXBCを出力できない)と気づき、`fxc.exe /T cs_5_0 /E main`で実際に`vector_add.hlsl`をコンパイルした(956バイトの実DXBCバイト列、手書きのダミーバイト列ではない)。`tools/compile-dxbc-shaders.ps1`新設(open-cudaの`tools/compile-dx12-shaders.ps1`と同じ命名・構造)。
  4. **実際に`cargo test`で確認した結果(誇張なし、実出力そのまま)**:
     ```
     running 3 tests
     test tests::rejects_truncated_dxbc_header ... ok
     test tests::rejects_garbage_bytes_that_are_not_a_dxbc_container ... ok
     test tests::parses_real_fxc_compiled_vector_add_dxbc_container ... ok

     test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     ```
     `parses_real_fxc_compiled_vector_add_dxbc_container`は実`fxc.exe`出力(`include_bytes!`で埋め込み)を解析し、チャンク数4以上・RDEF存在・OSGN存在・SHEX命令数>0を検証。`cargo build --workspace`警告0件、`cargo clippy --workspace --all-targets`警告0件。
  5. **正直な開示・未実装(誇張しない)**:
     - **SPIR-Vコード生成は一切未着手**。`parse_dxbc`はコンテナ/チャンクの存在確認までで、命令列をSPIR-Vへ翻訳する処理は無い。
     - **`opencuda-vulkan`/`GpuDevice`/`KernelSource::SpirV`への配線も未着手**(コードは1行も無い、`PORTING.md`に概念設計のみ記載)。
     - **DXIL(SM6+, D3D12)パースは未着手**(`dxbc`クレートが`DXIL`チャンクを不透明バイト列として保持するのみ)。
     - **実Vulkan実行・CPU参照実装との数値一致は未検証**(そもそもSPIR-Vが存在しないため検証しようがない)。
  - 次にすべきこと: (1) DXBCの`SHEX`命令列(`dxbc`クレートの`Program`/`Instruction`型)を実際に走査し、`vector_add.hlsl`程度の単純な命令列(バッファ読み込み2回+加算+書き込み)に対応する最小限のSPIR-Vモジュールを手書き生成するコードを書く(汎用命令セット全体を一度に狙わず、この1シェーダーが動くことをまず実証する)。(2) 生成したSPIR-Vバイト列を`opencuda_core::CompiledKernel::spirv`経由で`open-cuda`の実`VulkanDevice`(`../open-cuda/crates/opencuda-vulkan`をpath依存追加)へディスパッチし、CPU側の単純な加算結果と数値一致することを実機(このマシンのNVIDIA GT 730)で検証する——これができて初めて「フェーズ1垂直スライス達成」と報告できる。(3) 上記が動いてから、DXILパース(SM6+)・D3D11最小グラフィックスパイプラインへ進む(現時点ではまだ手を出さない)。

- **2026-07-25(続き) フェーズ1垂直スライス達成: DXBC(SM5.0)->SPIR-V翻訳+実Vulkan(NVIDIA GT 730)ディスパッチ+CPU参照実装との数値一致を実機で確認**:
  1. **実際のSHEX命令列を確認(ハードコード前提を排除)**: `vector_add.dxbc`(実fxc.exe出力)の`SHEX`チャンクを`dxbc`クレートで実際にデコードして中身を確認した。実際に出てくるオペコード列は`dcl_globalFlags` -> `dcl_uav_structured`(u0/u1/u2, stride=4) -> `dcl_input`(vThreadID) -> `dcl_temps`(1) -> `dcl_thread_group`(64,1,1) -> `ld_structured`(u0) -> `ld_structured`(u1) -> `add` -> `store_structured`(u2) -> `ret` という非常に狭い列だった。**Path A(狭いが実物のSM5.0デコーダ+実SPIR-Vコード生成)を選択**——このシェーダーが実際に使うオペコードだけを対象にした本物のデコーダが十分書ける規模だと確認できたため、Path B(既知シェーダーの手書きSPIR-V)へ逃げる必要はなかった。
  2. **実装**: `crates/directx-shader-translate/src/spirv_gen.rs`新設。
     - `decode_vector_add_shape(instructions: &[Instruction]) -> Result<VectorAddShape, SpirvGenError>`: 上記の狭いオペコード列を実際に1命令ずつ走査し、`dcl_uav_structured`/`ld_structured`/`store_structured`からUAVバインドポイント(u#の#)を、`dcl_thread_group`からスレッドグループサイズを抽出する。**1つでも想定外のオペコード・オペランド形状が混ざっていれば`SpirvGenError::UnsupportedShader`を返し処理を止める**(未対応命令を無視して"動いたふり"をしない)。
     - `emit_spirv(shape: &VectorAddShape) -> Vec<u32>`: `rspirv::dr::Builder`(手書きバイナリ列の直接構築ではなく、車輪の再発明を避けてrspirvクレートを採用、CLAUDE.md方針通り)を使い、抽出した実データ(UAVバインドポイント・スレッドグループサイズ)を反映したSPIR-Vモジュールを組み立てる。レイアウトは`opencuda-vulkan`の`vector_add`契約(storage buffer 3本、set=0/binding=実バインドポイント、push constant `uint n`、entry point `"main"`)に合わせた。
     - `translate_vector_add_shader(bytes: &[u8]) -> Result<TranslatedKernel, SpirvGenError>`(公開API): 上記2つを繋ぎ、`TranslatedKernel { spirv_words, entry_point, local_size, uav_bind_points }`を返す。
     - **正直な開示**: これは汎用SM5.0デコーダではない。`vector_add.hlsl`が実際に生成する狭いオペコード列(上記10命令種)専用。他のシェーダーを渡せば(対応外オペコードに遭遇した時点で)確実にエラーになる——これはモジュールのdocコメントとエラーメッセージに明記済み。
     - **境界チェック無し**: 実DXBCの命令列に比較・分岐命令が一切無かった(numthreads(64,1,1)×正確なグリッド数だけをディスパッチする前提)ため、生成したSPIR-Vにも境界チェックを入れていない。呼び出し側は`numthreads`の倍数でディスパッチする責任を負う(この点はテストのコメントにも明記)。
  3. **`opencuda-vulkan`への実配線**: `crates/directx-shader-translate/tests/vector_add_real_vulkan.rs`新設。`Cargo.toml`に`[dev-dependencies]`として`opencuda-core`/`opencuda-vulkan`(`real-vulkan`フィーチャ)へのcross-repoパス依存を追加(`../../../open-cuda/crates/opencuda-vulkan`、aruaru-dbの`open_raid_z_core`/`rust-json`パス依存と同じパターン)。**open-cuda側のコード・テストは一切変更していない**(依存するのみ)。テストは`open-cuda`の`examples/vector_add_vulkan_real`と同じ実機テストパターン(実GPU/Vulkanドライバが無ければ`eprintln!`してスキップ、フェイク成功にしない)に従う。
  4. **実際に`cargo test --workspace`で確認した結果(誇張なし、実出力そのまま、このマシンの実NVIDIA GT 730で実行)**:
     ```
     running 5 tests
     test spirv_gen::tests::rejects_garbage_bytes_honestly_instead_of_pretending_to_translate ... ok
     test tests::rejects_truncated_dxbc_header ... ok
     test tests::rejects_garbage_bytes_that_are_not_a_dxbc_container ... ok
     test tests::parses_real_fxc_compiled_vector_add_dxbc_container ... ok
     test spirv_gen::tests::translates_real_fxc_compiled_vector_add_dxbc_to_valid_spirv ... ok

     test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     running 1 test
     device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
     OK: DXBC(fxc.exe実コンパイル)->SPIR-V(自前生成)->実Vulkan(NVIDIA GT 730)経路が、CPU参照実装(a[i]+b[i])と256要素すべてで数値一致した
     c[0]=128, c[255]=255.5
     test dxbc_vector_add_matches_cpu_reference_on_real_vulkan_hardware ... ok

     test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     ```
     `dxbc_vector_add_matches_cpu_reference_on_real_vulkan_hardware`は、実fxc.exe出力のDXBCバイト列を実際にパース->実際にSPIR-V生成->実`VulkanDevice`(`ash`経由、`NVIDIA GeForce GT 730`)でディスパッチ->256要素すべてでCPU参照実装(`a[i]+b[i]`)と`1e-3`以内の一致を確認している。`cargo build --workspace`/`cargo clippy --workspace --all-targets`はいずれも警告0件。
  5. **正直な開示・まだやっていないこと(誇張しない)**:
     - **汎用SM5.0デコーダは無い**。今回実装したのは`vector_add.hlsl`が実際に生成する狭いオペコード列(10命令種)専用の変換器。異なるシェーダー(分岐・テクスチャサンプル・複数演算等を含むもの)を渡せば`SpirvGenError::UnsupportedShader`で拒否される(これは意図した動作であり、バグではない)。
     - **DXIL(SM6+, D3D12)パースは未着手**。
     - **境界チェック無し**——`numthreads`の倍数以外のNでディスパッチすると範囲外書き込みになりうる(現状のテストは256=64×4の倍数のみ検証)。
     - **D3D11グラフィックスパイプライン(頂点/ピクセルシェーダー・ラスタライズ)は未着手**。
  - 次にすべきこと: (1) 2つ目・3つ目の実DXBC Compute Shader(異なる演算・異なるUAV数・境界チェック付き等)を`fxc.exe`で実際にコンパイルし、`decode_vector_add_shape`相当のデコーダをそれらにも対応するよう一般化していく(1つの狭い専用デコーダから、実際に遭遇したオペコードを1つずつ追加していく漸進的なアプローチを継続する)。(2) DXILパース(SM6+, D3D12向け)に着手する。(3) この垂直スライスが安定してから、D3D11最小グラフィックスパイプラインへ進む。

- **2026-07-25(続き2) デコーダを3シェーダー形状(add/mul/境界チェック付きsub)へ一般化、実Vulkanで3本とも数値一致確認**:
  1. **新規シェーダー2本を実際に`fxc.exe /T cs_5_0 /E main`でコンパイル**(`tools/compile-dxbc-shaders.ps1`更新済み):
     - `shaders/vector_mul.hlsl`: `vector_add.hlsl`と同じ契約(UAV3本、256要素)だが演算が乗算。
     - `shaders/vector_sub_bounded.hlsl`: 定数バッファ(`cbuffer Params : register(b0) { uint ElementCount; }`)+`if (id.x < ElementCount)`境界チェック付きの減算。
  2. **実SHEX命令列を`examples/dump_shex.rs`(今回新設した調査用ツール)で実際にダンプして確認**(思い込みでデコーダを書かない、CLAUDE.md方針):
     - `vector_mul.dxbc`: `vector_add.dxbc`と全く同じ命令列で、`Opcode::Add`が`Opcode::Mul`に変わっているだけだった。
     - `vector_sub_bounded.dxbc`: **重要な発見**——`a - b`はfxcによって専用の`sub`オペコードではなく`add dest, -b, a`(第1ソースオペランド`operands[1]`に`negate: true`が立った`add`)へ最適化されることが実機出力で判明した。加えて`dcl_constantbuffer`(b0, immediateIndexed)・`ult`(定数バッファとの比較)・`if`/`endif`が実際に出現した。
  3. **`crates/directx-shader-translate/src/spirv_gen.rs`を一般化**: `decode_vector_add_shape`/`VectorAddShape`/`translate_vector_add_shader`を、共通骨格(`dcl_globalFlags` -> `dcl_constantbuffer`? -> `dcl_uav_structured`x3 -> `dcl_input` -> `dcl_temps` -> `dcl_thread_group` -> (`ult`+`if`)? -> `ld_structured`x2 -> (`add`|`mul`) -> `store_structured` -> `endif`? -> `ret`)を検証する`decode_shader_shape`/`ShaderShape`/`translate_shader`へ置き換えた。`BinaryOp::{Add,Mul,Sub}`を新設し、`add`命令の第1ソースオペランドの`negate`フラグでSubを検出する。`translate_vector_add_shader`は後方互換のため`translate_shader`への薄いエイリアスとして残した。**引き続き「対応している」という誤ったシグナルは出さない**——3パターンいずれにも一致しない命令・境界チェック構成が中途半端(定数バッファはあるのに`ult`/`if`/`endif`が揃っていない等)な場合は`SpirvGenError::UnsupportedShader`で拒否する。
  4. **`emit_spirv`を拡張**: `BinaryOp`に応じて`OpFAdd`/`OpFMul`/`OpFSub`を選択。境界チェック付きの場合は、従来「宣言のみで未使用」だったpush constant(`uint n`)を実際に`OpULessThan`の比較へ使い、`OpSelectionMerge`+`OpBranchConditional`+then/mergeブロックという本物の制御フローをSPIR-Vへ生成する(見せかけのpush constantではなく、実際に分岐を左右する)。
  5. **実Vulkanテストを2本追加**(`vector_add_real_vulkan.rs`と同じパターン): `tests/vector_mul_real_vulkan.rs`・`tests/vector_sub_bounded_real_vulkan.rs`。後者は320スレッドをディスパッチしつつpush constantの論理要素数を256に留め、256..320がセンチネル値(-1.0)のまま書き込まれないことをassertすることで、**境界チェックが実際に実行をゲートしていること**(単にコンパイルが通るだけでなく)を検証している。`opencuda-vulkan::VulkanDevice::launch_kernel`はカーネル名で引数配線を選ぶ実装(`"vector_add"`/`"matmul"`のみ認識)のため、mul/sub_boundedテストも引数レイアウトが同一な`"vector_add"`名を(コメントで理由を明記した上で)再利用している——実行される演算はSPIR-Vバイト列側で決まる。
  6. **実際に`cargo test --workspace -- --nocapture`で確認した結果(誇張なし、実出力そのまま、NVIDIA GeForce GT 730)**:
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
     `cargo clippy --workspace --all-targets`は警告0件(初回`needless_range_loop`警告1件を`iter().enumerate().skip()`へ書き換えて解消済み)。`cargo build --workspace`も警告0件。
  7. **DXIL調査(時間の許す範囲、フェーズ0相当・コンテナレベルのみ、日英Web検索)**: LLVM公式ドキュメントに`DirectX Container`(`llvm.org/docs/DirectX/DXContainer.html`)・`Architecture and Design of DXIL Support in LLVM`(`.../DXILArchitecture.html`)というページが存在することを確認——DXILパートは`ProgramHeader`+`BitcodeHeader`+シリアライズされたLLVM 3.7 IRモジュールから成り、マジック値`0x4C495844`(`'DXIL'`)を持つ。これは以前(同日の初回調査時点)より新しい・より公式な情報源で、LLVM本体がDXILバックエンドのアーキテクチャを文書化し始めていることが分かった。Rust側では`llvm-bitcode`という汎用bitcodeパーサークレートがcrates.ioに存在することも確認(未使用・未検証、候補として記録するのみ)。**実装は一切行っていない**——`dxbc`クレートは引き続き`DXIL`チャンクを不透明バイト列として保持するのみ。
  8. **正直な開示・まだやっていないこと(誇張しない)**:
     - **汎用SM5.0デコーダは依然として無い**。3パターン(add/mul/境界チェック付きnegated-add-as-sub)以外は`SpirvGenError::UnsupportedShader`で拒否される。真の`sub`オペコード(fxcが最適化しないケース)・`div`・`if`が複数ある形・elseブロック・ループ等は未対応。
     - **DXILパースは今回も実装していない**(container形式の調査のみ、上記7参照)。
     - **D3D11グラフィックスパイプラインは引き続き未着手**。
  - 次にすべきこと: (1) 引き続き実DXBCシェーダーを1つずつ`fxc.exe`でコンパイルし遭遇したオペコードを追加していく(次候補: 実際の`div`オペコード、複数の一時レジスタを使う式、`else`分岐)。(2) DXILの実バイト列パースへ本格着手する場合は`llvm-bitcode`クレートの実際の検証(このワークスペースでの`cargo add`試験)から始める。(3) この垂直スライスの3シェーダーが安定してから、D3D11最小グラフィックスパイプラインへ進む。

- **2026-07-25(続き3) DXIL実バイト列をbitstream/ブロックレベルまでパース(container止まりから前進)、D3D11 VS/PSのDXBCパースに着手**:
  1. **タスク1(DXIL)**: `shaders/vector_add_dxil.hlsl`(SM5.0版`vector_add.hlsl`と同一契約、あえて別ファイルに分離)を実`dxc.exe -T cs_6_0 -E main`(`C:\VulkanSDK\1.4.350.0\Bin\dxc.exe`)でコンパイルし、`shaders/vector_add.dxil`(実DXILバイト列、1392バイトのbitcode本体を含むDXBCコンテナ)を得た。`tools/compile-dxbc-shaders.ps1`にdxc.exe自動検出ロジック(`DXC_BIN`環境変数→PATH→`%VULKAN_SDK%\Bin\dxc.exe`の順)を追加。
     - 既存の`dxbc`クレート(`0.1.0`)を実際に調べたところ、`DXIL`チャンクの`DxilProgramHeader`/`DxilBitcodeHeader`(シェーダー種別・SM版数・マジック`'DXIL'`・bitcodeオフセット/サイズ)は**既にパース済み**で、`ChunkData::Dxil(DxilData { shader_kind, major_version, minor_version, dxil_version, bitcode })`として取得できることが分かった(前回HANDOFFの「不透明バイト列として保持するのみ」は、bitcode本体の中身を指しては正しいが、ヘッダ自体は既にパースされていたという訂正)。
     - `llvm-bitcode = "0.4.0"`を新規依存として実際に追加し(`cargo add`→ビルド確認)、`dxil_chunk.bitcode`(生LLVM bitcode)を`llvm_bitcode::Bitcode::new()`に渡したところ、**実際に成功した**。まず`examples/dump_dxil.rs`(調査用ツール、`dump_shex.rs`と同じ位置づけ)で中身を確認: LLVMラッパーマジック`BC\xC0\xDE`、トップレベル`MODULE_BLOCK`(id=8, 16要素)、その中に`TYPE_BLOCK_ID_NEW`(17)・`PARAMATTR_GROUP_BLOCK`(10)・`PARAMATTR_BLOCK`(9)・`CONSTANTS_BLOCK`(11)・`FUNCTION_BLOCK`(12、5個——`main`の基本ブロック数分)・`VALUE_SYMTAB_BLOCK`(14)・`METADATA_BLOCK`(15、2個)という、LLVM標準ブロックIDとして辻褄の合う構造が実際に出てきた。
     - `crates/directx-shader-translate/src/dxil.rs`新設: `parse_dxil_container(bytes) -> Result<DxilModule, DxilParseError>`。`DxilModule`は`shader_kind`/`shader_model_major`/`minor`/`dxil_version`/`bitcode_byte_len`/`bitcode_has_llvm_magic`(実バイト列を見て確認、決め打ちでない)/`top_level_blocks: Vec<BitcodeBlockSummary>`(各ブロックの生の`block_id`・子要素数・子ブロックID一覧)を持つ。5件のテスト(実dxc.exe出力を`include_bytes!`で埋め込み、DXILチャンクの無いDXBC〈SM5.0〉を渡した場合に正直に`NoDxilChunk`を返すことも確認)、全green。
     - **正直な開示・ここで止まっている**: LLVM型システムの解決(`TYPE_BLOCK`のレコードを実際の型として解釈する)、命令オペコードの意味解釈(`FUNCTION_BLOCK`内のレコードがload/store/callのどれか等)は一切していない。ブロック/レコードの木構造(生の数値ID)が読めるところまで。DXIL→SPIR-V変換は存在しない。
  2. **タスク2(D3D11グラフィックスパイプライン)**: `shaders/triangle_vs.hlsl`(`POSITION`/`COLOR`入力→`SV_POSITION`/`COLOR`出力の最小パススルー頂点シェーダー)・`shaders/triangle_ps.hlsl`(`COLOR`入力→`SV_TARGET`出力のパススルーピクセルシェーダー)を実`fxc.exe /T vs_5_0`・`/T ps_5_0`でコンパイル。
     - 既存の`parse_dxbc`(無改修)がVS/PSのDXBCコンテナも問題なくパースできることを確認(`src/lib.rs`に2件のテスト追加、`has_input_signature`/`has_output_signature`/`instruction_count > 0`を検証)。
     - `examples/dump_shex.rs`で実SHEX命令列を実際にダンプして確認(思い込みで書かない、CLAUDE.md方針の継続): VSは`dcl_globalFlags`→`dcl_input`(POSITION, mask 7)→`dcl_input`(COLOR, mask 15)→`dcl_output_siv`(SV_POSITION)→`dcl_output`(COLOR)→`mov`x3→`ret`。PSは`dcl_globalFlags`→`dcl_input_ps`(`linear`補間付き、COLOR)→`dcl_output`(SV_TARGET)→`mov`→`ret`。**Compute Shaderで出てきたオペコード(`dcl_uav_structured`・`ld_structured`・`store_structured`・`dcl_thread_group`)は一切出現しない**——想定通りだが、実際にダンプして確認した上での記録。
     - `translate_shader`(Compute専用)にVSのDXBCを渡すと`SpirvGenError::UnsupportedShader`で正しく拒否されることをテストで確認(`vertex_shader_spirv_translation_is_honestly_unimplemented_not_silently_wrong`)——誤ったSPIR-Vを黙って生成しないことの継続的な保証。
     - **正直な開示・未着手**: VS/PS向けのSPIR-Vコード生成、ラスタライザ、出力マージ、実Vulkanでの三角形描画は一切実装していない(タスク指示通りスコープ外)。
  3. **検証**: `cargo test --workspace`(型チェックのみで完了と報告しない、実行結果):
     ```
     running 15 tests (unit)
     ... 全15件 ok(内訳: 既存7件+dxil::tests 5件+lib.rs新規3件)
     test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     running 1 test (tests/vector_add_real_vulkan.rs) ... ok
     running 1 test (tests/vector_mul_real_vulkan.rs) ... ok
     running 1 test (tests/vector_sub_bounded_real_vulkan.rs) ... ok
     ```
     合計18件全green(このマシンの実NVIDIA GT 730での実Vulkan実行3件を含む、フェイク成功なし)。`cargo clippy --workspace --all-targets`警告0件。
  4. **正直な開示・全体まとめ(誇張しない)**:
     - DXILは「コンテナレベル」から「LLVM bitstreamのブロック/レコード木レベル」まで前進したが、命令・型の意味解釈と DXIL→SPIR-V 変換は依然として未着手。
     - D3D11グラフィックスパイプラインは「DXBCコンテナがパースできる」段階のみ。SPIR-V生成・実描画には未到達(タスク指示のスコープ通り)。
     - Compute Shader側(SM5.0、3シェーダー形状)の対応範囲・実Vulkan検証済み範囲に変更は無い(既存のまま)。
  - 次にすべきこと: (1) DXIL側: `TYPE_BLOCK`のレコードを実際の型テーブルへ、`FUNCTION_BLOCK`のレコードを実際の命令列へデコードする(`vector_add_dxil.hlsl`という1つの既知シェーダーに絞ってまず動かす、DXBC/SM5.0側で採用した「狭いが実物」のアプローチを踏襲)。(2) グラフィックス側: `spirv_gen`(またはそれと並行する新規デコーダ)をVS/PSのオペコード(`dcl_output_siv`・`dcl_input_ps`・補間モード等)に対応するよう拡張し、最終的に実Vulkanで実際に三角形を描画するところまで進める。(3) 両タスクとも、今回同様「実バイト列を確認してから対応opcodeを追加する」漸進的アプローチを継続する。
