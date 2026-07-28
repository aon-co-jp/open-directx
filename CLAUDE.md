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

- **2026-07-25(続き4) DXBC->SPIR-Vデコーダへ4つ目の実シェーダー(除算)を追加、実Vulkanで数値一致確認**:
  1. **新規シェーダー**: `shaders/vector_div.hlsl`(add/mul/sub_boundedと同じ`RWStructuredBuffer`3本・256要素契約、演算のみ除算)を実`fxc.exe /T cs_5_0 /E main`でコンパイル(`tools/compile-dxbc-shaders.ps1`更新済み)。
  2. **実SHEX命令列を`examples/dump_shex.rs`で実際にダンプして確認**(思い込みで対応を追加しない、既存方針の継続): `vector_div.dxbc`は`vector_add.dxbc`/`vector_mul.dxbc`と全く同じ命令形状(`dcl_uav_structured`x3 -> `dcl_input`(vThreadID) -> `dcl_temps`(1) -> `dcl_thread_group`(64,1,1) -> `ld_structured`x2 -> `Opcode::Div` -> `store_structured` -> `ret`)で、オペコードだけが`Add`/`Mul`から`Div`へ変わっているだけだった。fxcは除算に特別な最適化(vector_sub_boundedのnegated-addのような書き換え)を行わないことも実機出力で確認できた。
  3. **実装**: `BinaryOp::Div`を新設、`decode_shader_shape`の`match ins.opcode`へ`Opcode::Div => op = Some(BinaryOp::Div)`を追加、`emit_spirv`で`OpFDiv`(`b.f_div`)を選ぶ分岐を追加。既存の3パターン(add/mul/negated-add-as-sub)のロジックには一切手を入れていない(追加のみ、非破壊)。
  4. **実Vulkanテスト追加**: `tests/vector_div_real_vulkan.rs`(既存の`vector_mul_real_vulkan.rs`と同型パターン、ゼロ除算を避けるため入力を両方とも正の非ゼロ値に構成)。
  5. **実際に`cargo test --workspace -- --nocapture`で確認した結果(誇張なし、実出力そのまま、NVIDIA GeForce GT 730)**:
     ```
     test spirv_gen::tests::translates_real_fxc_compiled_vector_div_dxbc_to_valid_spirv ... ok
     test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     running 1 test
     device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
     OK: DXBC(fxc.exe実コンパイル, div)->SPIR-V(自前生成)->実Vulkan経路が、CPU参照実装(a[i]/b[i])と256要素すべてで数値一致した
     c[0]=0.5, c[255]=1.7966101
     test dxbc_vector_div_matches_cpu_reference_on_real_vulkan_hardware ... ok
     ```
     ワークスペース全体で計20テスト全green(実Vulkanディスパッチ4本を含む)。`cargo clippy --workspace --all-targets`警告0件。
  6. **正直な開示**: 対応する2項演算はadd/mul/negated-add-as-sub/divの4種のみ。この4パターン以外のオペコード・オペランド形状は引き続き`SpirvGenError::UnsupportedShader`で拒否される。DXIL側・D3D11グラフィックスパイプライン側は前回エントリから変更なし(未着手のまま)。
  - 次にすべきこと: 前回エントリの(1)(2)(3)は変更なし(DXILの型/命令デコード、VS/PS向けSPIR-V生成、両タスクとも「実バイト列確認→対応追加」の継続)。Compute Shader側の演算網羅については、次候補として複数の一時レジスタを使う式(例: `(a+b)*c`のような3入力演算、UAV数が3以外になるケース)や`else`分岐を検討する。

- **2026-07-25(続き5) D3D12track: DXILの型テーブル解決+FUNCTION_BLOCK命令列デコードへ前進(SPIR-V変換にはまだ未到達、正直な区切り)**:
  - ユーザー指示「D3D11が終わったらD3D12などに作業を移して下さい」を受け、D3D12track=DXIL(SM6+)パースの深化に着手。SM5.0/DXBC側(Compute Shader4形状・実Vulkan検証済み)は変更していない。
  1. **調査**: `llvm-bitcode`クレート(`0.4.0`、ローカルの`~/.cargo/registry/src/.../llvm-bitcode-0.4.0/src`)自体のソースを実際に読み、`Block { id, elements }`/`Record { id, fields() }`という生のブロック/レコードまでしか提供しない(DXIL/LLVM型・命令の意味解釈は一切していない)ことを確認した——前回HANDOFFで「候補として記録するのみ」だった点を実ソースで裏取りした形。
  2. **`examples/dump_dxil.rs`を拡張**: `TYPE_BLOCK_ID_NEW`(17)・`FUNCTION_BLOCK`(12)については、子ブロックIDだけでなくレコードの`code`と`fields()`の実際の値まで出力するようにした。これで`shaders/vector_add.dxil`のTYPE_BLOCK(26要素)・FUNCTION_BLOCK(13要素)の生レコードを実際に確認できた(値はコード側のdocコメントにも転記済み)。
  3. **型テーブル解決**: `crates/directx-shader-translate/src/dxil.rs`に`DxilType`(`Void`/`Float`/`Double`/`Integer{bits}`/`Pointer{pointee,address_space}`/`Function`/`StructNamed{name}`/`Metadata`/`Other{code}`/`StructNameMarker`)と`resolve_type_table(&Block) -> Vec<DxilType>`を追加。LLVM公式のTYPE_BLOCKレコードコード表(`llvm.org/docs/BitCodeFormat.html`のtype codes、`NUMENTRY=1`/`VOID=2`/`FLOAT=3`/`INTEGER=7`/`POINTER=8`/`FUNCTION=21`/`STRUCT_NAME=19`/`STRUCT_NAMED=20`/`METADATA=16`)をそのまま当てはめた。実`vector_add.dxil`に対して実際に解決した結果、型#12が`Float`、型#19が`StructNamed{name: "class.RWStructuredBuffer<float>"}`であることを確認した(推測ではなく実バイト列から得た値)。
  4. **命令列デコード**: `DxilInstruction`(`DeclareBlocks{basic_block_count}`/`BinOp{fields}`/`Ret`/`Call{fields}`/`ExtractValue{fields}`/`Other{code,fields}`)と`decode_function_instructions(&Block) -> Vec<DxilInstruction>`を追加。LLVM公式のFUNC_CODE表(`DECLAREBLOCKS=1`/`INST_BINOP=2`/`INST_RET=10`/`INST_EXTRACTVAL=26`/`INST_CALL=34`)を適用。実`vector_add.dxil`のFUNCTION_BLOCK(基本ブロック1つ)を実際にデコードした結果、`DeclareBlocks(1)` -> `Call`x5 -> `ExtractValue` -> `Call` -> `ExtractValue` -> `BinOp` -> `Call` -> `Ret`という並びが得られた。DXILは組み込み演算(`CreateHandle`/`ThreadId`/`BufferLoad`/`BufferStore`)をすべて`dx.op.*`という通常のLLVM関数呼び出し(`FUNC_CODE_INST_CALL`)として表現するため、これらは全部`Call`としてしか区別できない(呼び出し先関数のシンボル解決は今回未実装、正直な開示)。
  5. **`decode_vector_add_dxil_shape`/`decode_vector_add_dxil`**: 上記の実際に観測した並び(`DeclareBlocks(1)` -> `Call`/`ExtractValue`の混在 -> ちょうど1回の`BinOp` -> `BinOp`後に少なくとも1回の`Call` -> `Ret`)を検証する狭いマッチャーを実装。1つでも想定外の並び(`BinOp`が2回、基本ブロックが2つ以上、`Other`命令が混ざる等)なら`DxilShapeError`で正直に拒否する(DXBC側`SpirvGenError::UnsupportedShader`と同じ設計方針)。`decode_vector_add_dxil(bytes)`は`parse_dxil_container`と同様にDXBCコンテナ->DXILチャンク->bitstream->MODULE_BLOCK->TYPE_BLOCK/FUNCTION_BLOCKまで一気通貫で辿る便宜関数として追加。
  6. **テスト(実際に`cargo test --workspace`で確認、誇張なし)**:
     ```
     running 19 tests
     test dxil::tests::resolves_real_dxil_type_table_and_finds_float_and_resource_struct ... ok
     test dxil::tests::decodes_real_dxil_function_block_into_matching_vector_add_shape ... ok
     test dxil::tests::shape_matcher_honestly_rejects_unexpected_instruction_orderings ... ok
     (既存16件含め) test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     ```
     `resolves_real_dxil_type_table_and_finds_float_and_resource_struct`は実`vector_add.dxil`から22個の型を解決し、`Float`と`StructNamed{"class.RWStructuredBuffer<float>"}`が実際に含まれることを検証。`decodes_real_dxil_function_block_into_matching_vector_add_shape`は同じ実バイト列から`binop_count=1`/`extract_value_count=2`/`call_count=7`を得ることを検証。`shape_matcher_honestly_rejects_unexpected_instruction_orderings`はロジック単体(手構築した`DxilInstruction`列)に対して、`BinOp`2回・基本ブロック2つ・float型無しの3パターンがそれぞれ正しいエラーで拒否されることを確認。ワークスペース全体で23テスト全green(実Vulkanディスパッチ4本を含む、既存分に変更なし)。`cargo build --workspace`/`cargo clippy --workspace --all-targets`はいずれも警告0件。
  7. **正直な開示・まだやっていないこと(誇張しない)**:
     - **DXIL→SPIR-V変換は依然として存在しない**。今回やったのは型テーブル解決と命令列の「大分類」(Call/ExtractValue/BinOp/Ret)までで、`Call`命令の引数(LLVM bitcodeの相対値参照方式でエンコードされたオペランド列)を解決して「どの呼び出しがCreateHandleで、どのUAVバインドポイントを指すか」を突き止める処理は書いていない。これが無いとSPIR-Vの`OpAccessChain`/`OpLoad`/`OpStore`のバインディング先が決められないため、SPIR-V生成には未着手のまま。
     - **VALUE_SYMTAB_BLOCK(関数名解決)は読んでいない**——`dx.op.createHandle`等の実際の関数名を文字列として確認していないため、上記の`Call`区別ができないのはこれが理由(呼び出し先を名前で引けない)。
     - **`Call`命令のフィールド(`fields: Vec<u64>`)は生のまま保持しているのみ**——LLVM bitcodeの相対値参照(直前の値からの差分インデックス方式)のデコードは未実装。
     - **実Vulkanディスパッチ・CPU参照実装との数値一致検証は、DXIL側では未着手**(SPIR-Vが無いため検証しようがない、DXBC側の4シェーダーの検証範囲に変更は無い)。
     - D3D11グラフィックスパイプライン(VS/PS向けSPIR-V生成・ラスタライズ・実描画)は前回エントリから変更なし(未着手のまま)。
  - 次にすべきこと: (1) `VALUE_SYMTAB_BLOCK`(id=14)を読んで関数名文字列を解決し、`Call`命令がどの`dx.op.*`組み込みか(`CreateHandle`/`ThreadId`/`BufferLoad`/`BufferStore`)を実際に区別できるようにする。(2) LLVM bitcodeの相対値参照デコード(`Call`/`BinOp`/`ExtractValue`のオペランドが指す値を解決する)に着手し、UAVバインドポイント(u0/u1/u2)を実際に取り出す。(3) (1)(2)ができたら、DXBC側の`emit_spirv`(`spirv_gen.rs`)を再利用してDXIL版`vector_add`のSPIR-Vを生成し、実Vulkan(NVIDIA GT 730)でCPU参照実装との数値一致を検証する(DXBC側と同じ「フェーズ1垂直スライス」達成を目指す)。(4) D3D11グラフィックスパイプライン(VS/PS)側も引き続き並行して検討可。

- **2026-07-25(続き6) D3D12track: 7個のCall命令を`VALUE_SYMTAB_BLOCK`解決+LLVM相対値オペランドデコードで実際に全部disambiguate(SPIR-V生成にはまだ未到達、正直な区切り)**:
  1. **`examples/dump_dxil.rs`をさらに拡張**(調査ツール、思い込みで実装しない方針の継続): `VALUE_SYMTAB_BLOCK`(id=14)のレコードを`Record::take_payload()`まで掘り下げてダンプするようにした。**重要な発見**: `llvm-bitcode`クレートの`Record::fields()`は`VST_CODE_ENTRY`の値ID(1個)しか返さず、実際の関数名文字列は`fields()`には乗らず`payload()`(`Payload::Char6String`)側にあることが実際のダンプで判明した(`fields()`だけを見ていた前回HANDOFFの記述はこの点で不十分だった)。実際にダンプした結果: 値ID0=`main`, 1=`dx.op.threadId.i32`, 2=`dx.op.createHandle`, 3=`dx.op.bufferLoad.f32`, 4=`dx.op.bufferStore.f32`(この5つがモジュール内の全関数、宣言順=値ID順)。同様にモジュールレベル/関数ローカル両方の`CONSTANTS_BLOCK`(id=11)の生レコードも実際にダンプして確認した。
  2. **DXILオペコード番号をWeb検索で実際に確認**(記憶に頼らない、CLAUDE.md方針): Microsoft `DirectXShaderCompiler/docs/DXIL.rst`・LLVM `DXILOpBuilder`関連ドキュメントを検索し、`CreateHandle`=57・`BufferLoad`=68・`BufferStore`=69・`ThreadId`=93であることを確認した。実際に関数ローカル`CONSTANTS_BLOCK`から得た整数定数(符号付きVBRデコード後: 57, 68, 93, 69)とも完全に一致した。LLVM `CST_CODE_SETTYPE`=1/`CST_CODE_NULL`=2/`CST_CODE_UNDEF`=3/`CST_CODE_INTEGER`=4という定数コード表もLLVM公式`LLVMBitCodes.h`をWeb検索で確認した上で採用した。
  3. **LLVM相対値オペランド算術を実バイト列に対して手計算で検証**: グローバル値番号付け順序(関数宣言5個(値0-4) -> モジュールレベル定数(`SETTYPE`は値を消費しない、実際に値を消費するのは`NULL`/`UNDEF`/`INTEGER`のみ、値5-15) -> 関数ローカル定数(値16-21))を実際に組み立て、7個の`Call`・2個の`ExtractValue`・1個の`BinOp`の生フィールド列を「`current_value_no`(その時点までに定義済みの値の総数、これから追加するこの命令自身の結果は含まない) - フィールド値 = 絶対値ID」という規約で1つずつ手計算し、以下の対応が実際に導けることを確認した:
     - `Call`1つ目(基本ブロック内の3番目の要素): `CreateHandle(range_id=2)` — 引数はopcode=57(検証済み)・resourceClass=1(UAV)・range_id=2・index=2・nonUniform=false。
     - `Call`2つ目: `CreateHandle(range_id=1)`。
     - `Call`3つ目: `CreateHandle(range_id=0)`。
     - `Call`4つ目: `ThreadId`(component=0)。
     - `Call`5つ目: `BufferLoad`、ハンドル引数が`CreateHandle(range_id=0)`の結果を指す(=u0からの読み出し)、座標引数が`ThreadId`の結果を指す。
     - `ExtractValue`(1つ目): 直前の`BufferLoad`(u0)の集約値から`.x`を取り出す。
     - `Call`6つ目: `BufferLoad`、ハンドルが`CreateHandle(range_id=1)`の結果(=u1からの読み出し)。
     - `ExtractValue`(2つ目): u1側`BufferLoad`の集約値から`.x`。
     - `BinOp`: 上記2つの`ExtractValue`結果を加算(`fadd`、フラグ`31`=高速数学フラグ全部)。
     - `Call`7つ目: `BufferStore`、ハンドルが`CreateHandle(range_id=2)`の結果(=u2への書き込み)、座標が`ThreadId`の結果、値引数が`BinOp`の結果、マスク=1(x成分のみ書き込み)。
     この手計算トレースは`range_id`の値(2,1,0という宣言順とは逆の並び)まで含めてこのHANDOFFに転記済み(コード側のコメントにも同内容を記載)。
  4. **実装**: `crates/directx-shader-translate/src/dxil.rs`に`resolve_vector_add_dxil_calls(bytes) -> Result<Vec<ResolvedDxilCall>, DxilCallResolutionError>`を新設。内部で`DxilValue`(意味解決した値、`Function`/`ConstantInt`/`ConstantZero`/`ConstantUndef`/`CreateHandleResult`/`ThreadIdResult`/`BufferLoadAggregate`/`ExtractedBufferValue`/`BinOpResult`/`Other`)・`resolve_relative`(上記の相対値算術)・`decode_signed_vbr`(LLVMの符号付きVBR規約)・`resolve_module_function_names`(`VALUE_SYMTAB_BLOCK`解決)・`decode_constants_block`(`CONSTANTS_BLOCK`の値消費レコードのみを値リストへ積む)を実装し、`ResolvedDxilCall::{CreateHandle{range_id}, ThreadId, BufferLoad{handle_range_id}, BufferStore{handle_range_id}}`という列挙で7個の`Call`の意味を返す。想定と一致しない場合(未知の呼び出し先・引数数不一致・オペコード不一致・ハンドルが`CreateHandle`由来でない・格納値が`BinOp`結果でない等)は`DxilCallResolutionError`の各バリアントで正直に拒否する(`SpirvGenError::UnsupportedShader`/`DxilShapeError`と同じ設計方針、他の8種のCall/BinOp/ExtractValueパターンは一切対応しない、狭いが実物)。
  5. **正直な開示(このセクションのスコープ)**: 汎用LLVM値番号付け/相対値デコーダではない。`vector_add_dxil.hlsl`が実際に生成する狭い形状(関数1個・基本ブロック1個、`CreateHandle`x3+`ThreadId`x1+`BufferLoad`x2+`BufferStore`x1)専用。`dx.op.bufferStore.f32`はvoidを返すためLLVMが値番号を割り当てない、という前提はLLVM `BitcodeReader`のドキュメント化された規約に基づく判断であり、このシェーダーの命令列内では(直後が`Ret`のみで何もこの値を参照しないため)実バイト列だけからは検証しきれていない点も正直に記す。
  6. **テスト(実際に`cargo test --workspace`で確認、誇張なし)**:
     ```
     running 22 tests
     test dxil::tests::resolves_all_seven_calls_in_real_vector_add_dxil_to_their_real_dx_op_meaning ... ok
     test dxil::tests::resolve_relative_computes_absolute_index_from_current_value_count ... ok
     test dxil::tests::decode_signed_vbr_matches_llvm_sign_bit_convention ... ok
     (既存19件含め) test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     ```
     `resolves_all_seven_calls_in_real_vector_add_dxil_to_their_real_dx_op_meaning`は実`vector_add.dxil`から`[CreateHandle{2}, CreateHandle{1}, CreateHandle{0}, ThreadId, BufferLoad{0}, BufferLoad{1}, BufferStore{2}]`という、上記の手計算トレースと完全に一致する結果が得られることを検証する(ロジック単体のテストではなく実バイト列パイプライン経由)。ワークスペース全体で26テスト全green(実Vulkanディスパッチ4本含む、DXBC側4シェーダーの検証範囲に変更なし)。`cargo build --workspace`/`cargo clippy --workspace --all-targets`はいずれも警告0件。
  7. **正直な開示・まだやっていないこと(誇張しない)**:
     - **DXIL→SPIR-V変換は依然として存在しない**。今回やったのは7個の`Call`の意味解決までで、これらの情報(どのハンドルがどのUAVバインドポイントか・スレッドID取得方法・加算対象)を使って実際にSPIR-Vの`OpAccessChain`/`OpLoad`/`OpStore`を組み立てる処理は書いていない。タスク指示の「narrow but real」方針通り、ここを正直な区切りとする(SPIR-V生成・実Vulkanディスパッチはこの増分のスコープ外、次回増分の対象)。
     - `range_id`(2,1,0)が実際のDXBC側`u#`バインドポイント番号と1対1対応するという前提は、DXBC側の`dcl_uav_structured`のバインドポイント抽出ロジックと同じ発想だが、DXIL側で`range_id`が常にレジスタ番号と一致するかどうかの一般則までは検証していない(このシェーダー1本での実測に基づく)。
     - D3D11グラフィックスパイプライン(VS/PS向けSPIR-V生成・ラスタライズ・実描画)は前回エントリから変更なし(未着手のまま)。
  - 次にすべきこと: (1) 今回解決した7個のCallの意味(`ResolvedDxilCall`)を使い、DXBC側`spirv_gen.rs`の`emit_spirv`同等のSPIR-V生成処理をDXIL版`vector_add`向けに実装し、実Vulkan(NVIDIA GT 730)でCPU参照実装との数値一致を検証する(DXBC側と同じ「フェーズ1垂直スライス」達成が次の目標)。(2) D3D11グラフィックスパイプライン(VS/PS)側も引き続き並行して検討可。(3) 余裕があれば2つ目のDXILシェーダー(mul/sub/div等)を`dxc.exe`で実際にコンパイルし、`resolve_vector_add_dxil_calls`を一般化する土台を作る(DXBC側で採用した「1つずつ実バイト列を確認して対応を広げる」漸進的アプローチを継続)。

- **2026-07-25(続き7) D3D12track: DXIL版vector_addのSPIR-V生成+実Vulkanディスパッチを達成、D3D11/D3D12双方の垂直スライスがパリティに到達**:
  1. **共有化**: `spirv_gen.rs`の`emit_spirv(shape: &ShaderShape)`本体を`emit_spirv_impl`へリネームし、DXBC固有の`ShaderShape`型に依存しないパラメータのみを取る`pub(crate) fn emit_spirv_for_kernel(thread_group, uav_a, uav_b, uav_c, op: BinaryOp, bounds_check: bool) -> Vec<u32>`を新設した(既存の`emit_spirv`はこれを呼ぶ薄いラッパーへ変更、DXBC側の挙動・既存4テストは無変更)。
  2. **DXIL版バックエンド**: `dxil.rs`に`translate_dxil_vector_add_to_spirv(bytes) -> Result<TranslatedKernel, DxilSpirvError>`を追加。前回増分で解決済みの7個の`ResolvedDxilCall`(`CreateHandle{range_id}`x3・`ThreadId`・`BufferLoad{handle_range_id}`x2・`BufferStore{handle_range_id}`)から、最初の`BufferLoad`のrange_idをA、2番目をB、`BufferStore`のrange_idをCとして`emit_spirv_for_kernel`を呼ぶ(DXBC側`ld_uavs`の発見順規約と同じ)。演算は`BinaryOp::Add`固定・境界チェック無し固定(`vector_add_dxil.hlsl`が実際にこの形であることは前回までに確認済み)。
  3. **正直な開示(スレッドグループサイズ)**: DXBCの`dcl_thread_group`に相当する情報(`numthreads`)はDXILでは`METADATA_BLOCK`内の`dx.entryPoints`メタデータにエンコードされており、今回のスコープ(`FUNCTION_BLOCK`命令列デコード)では抽出していない。唯一対象とする実バイト列`vector_add.dxil`が`shaders/vector_add_dxil.hlsl`(`[numthreads(64,1,1)]`、DXBC版と意図的に同一契約)由来だと分かっているため、`(64,1,1)`を決め打ちで使っている——DXBC側の「宣言命令から実抽出」という原則からの唯一の逸脱であり、次回以降のMETADATA_BLOCK解析が対応すべき既知の負債として明記する。
  4. **実Vulkanテスト追加**: `tests/vector_add_dxil_real_vulkan.rs`(`vector_add_real_vulkan.rs`と全く同じパターン、`VulkanDevice`の`launch_kernel`が組み込みカーネル名でディスパッチ先を判定する契約のため、`CompiledKernel::spirv`のカーネル名は`"vector_add_dxil"`ではなく`"vector_add"`を渡す必要があった——実行時に`VulkanDevice v0.4.0 only implements vector_add/vector_add_f32 and matmul/matmul_f32`という実エラーで判明し、修正した)。
  5. **実際に`cargo test --workspace`で確認した結果(誇張なし、実出力そのまま、NVIDIA GeForce GT 730)**:
     ```
     test dxil_vector_add_matches_cpu_reference_on_real_vulkan_hardware ... ok
     test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.63s
     ```
     標準出力: `device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)` / `OK: DXIL(dxc.exe実コンパイル、SM6.0)->SPIR-V(自前生成)->実Vulkan(NVIDIA GT 730)経路が、CPU参照実装(a[i]+b[i])と256要素すべてで数値一致した`。DXBC側4シェーダー分の実機テストも引き続き全green(既存分に変更なし)。ワークスペース全体: unittests 22件 + 実機テスト5本(DXBC4本+DXIL1本)全green。`cargo build --workspace`/`cargo clippy --workspace --all-targets`はいずれも警告0件。
  6. **達成した内容(正直な棚卸し)**: D3D12/DXILパイプラインが、D3D11/DXBCパイプラインと同じ「実dxc.exeコンパイル済みシェーダー -> 実際のコンテナ/bitstream解析 -> 実SPIR-V生成 -> 実Vulkanディスパッチ -> CPU参照実装との数値一致」という垂直スライスの終端に到達した。**ただし依然として1つの既知シェーダー形状(`vector_add_dxil.hlsl`)専用**であり、汎用SM6.0デコーダではない(add以外の演算・複数基本ブロック・境界チェック・numthreads以外の値には非対応、いずれも今回の型テーブル/命令列/Call解決ロジックがそのまま拒否する)。
  - 次にすべきこと: (1) `METADATA_BLOCK`から実際に`numthreads`(および将来的にはリソースバインディング情報)を抽出し、DXIL側の決め打ち値を無くす。(2) DXBC側で先に対応したmul/sub_bounded/div相当のDXILシェーダーを`dxc.exe`で追加コンパイルし、`resolve_vector_add_dxil_calls`/`translate_dxil_vector_add_to_spirv`を一般化する(DXBC側で採用した「1つずつ実バイト列を確認して対応を広げる」漸進的アプローチを継続)。(3) D3D11グラフィックスパイプライン(VS/PS向けSPIR-V生成・ラスタライズ・実描画)は前回エントリから変更なし(未着手のまま、引き続き並行で検討可)。

- **2026-07-25(続き8) 独立した2つの完成度改善タスクを実施: (1)DXILのnumThreadsをMETADATA_BLOCKから実抽出(前回HANDOFFで明記した決め打ちの負債を解消)、(2)DXBCデコーダを「N個の逐次2項演算」パターンクラスへ一般化(既存4形状は無変更)**:
  1. **タスク1(DXIL numThreads実抽出)**:
     - `examples/dump_dxil.rs`を拡張し`METADATA_BLOCK`(id=15)のレコード(fields+`take_payload()`)まで実際にダンプできるようにした。`vector_add.dxil`には実際に`METADATA_BLOCK`が2個(実際のメタデータノード列41件、と`METADATA_KIND`固定名一覧16件)存在することを再確認。
     - Web検索でMicrosoft`DirectXShaderCompiler`の`lib/DXIL/DxilMetadataHelper.cpp`(`EmitDxilEntryProperties`/`LoadDxilEntryProperties`)・`include/dxc/DXIL/DxilMetadataHelper.h`を実際に確認し、`kDxilNumThreadsTag=4`という実際の数値と、`ShaderProperties`が`{tag, value, tag, value, ...}`という繰り返しノードで、`NumThreads`の値は`{x,y,z}`3要素の`MDNode`(各要素は`Uint32ToConstMD`)であることを確認した。
     - `vector_add.dxil`の実バイト列に対して、`dx.entryPoints`(named-node)→エントリポイント5要素タプル(`Function,Name,Signatures,Resources,ShaderProperties`)→`ShaderProperties`ノード(`{tag=MD3(値0=ShaderFlags), value=MD24, tag=MD14(値4=NumThreads), value=MD26}`)→`NumThreads`ノード(`MD26={MD25,MD2,MD2}`)→モジュール値リスト解決(`MD25`→値12→module定数64、`MD2`→値5→module定数1)という経路を実際に手計算でトレースし、`(64,1,1)`が導けることを検証した上でコード化した(推測ではなく実際にバイト列を1つずつ辿って確認)。
     - 実装(`crates/directx-shader-translate/src/dxil.rs`): `MetadataEntry`(`String`/`Value{value_ref}`/`Node(fields)`)・`decode_metadata_block`(`METADATA_STRING_OLD`(1)/`METADATA_VALUE`(2)/`METADATA_NODE`(3,5)/`METADATA_NAME`(4)/`METADATA_NAMED_NODE`(10)を実際にデコード、`METADATA_KIND`(6)等はMDインデックスを消費しないので無視)・`resolve_md_operand`(val-1、0=null)・`find_numthreads_in_shader_properties`(純粋関数、`{tag,value}`ペアを走査し`kDxilNumThreadsTag`=4と一致するペアの値ノードを解決)・`extract_numthreads_from_metadata(bytes) -> Result<(u32,u32,u32), DxilNumThreadsError>`(公開API、複数の`METADATA_BLOCK`兄弟から`dx.entryPoints`を持つ方を実際に探す、決め打ちで「1つ目」を使わない)。`resolve_vector_add_dxil_calls`が元々インラインで持っていた「関数宣言+モジュール定数」というグローバル値番号付けの構築ロジックを`build_module_value_list`へ切り出し、numThreads抽出側とも共有した(重複実装ではなく共通化)。
     - `translate_dxil_vector_add_to_spirv`が、以前の決め打ち`(64,1,1)`ではなく`extract_numthreads_from_metadata(bytes)?`を呼ぶよう変更(`DxilSpirvError`に`NumThreads(#[from] DxilNumThreadsError)`を追加)。
     - **回帰防止テスト**: `finds_numthreads_pair_even_when_a_different_value_precedes_it`は、`vector_add.dxil`とは異なる値`(32,8,2)`を持つ合成`MetadataEntry`/`DxilValue`列(タグ0=ShaderFlagsが先、タグ4=NumThreadsが後という同じ並び)を手構築し、`find_numthreads_in_shader_properties`が正しく`(32,8,2)`を返すことを検証する——実装が「METADATA_BLOCKを読んだふりをして実は`(64,1,1)`を返すだけ」というハードコードへ後退した場合に確実に失敗する(実バイト列側のテストだけでは`(64,1,1)`という偶然の一致で検出できない)。
  2. **タスク2(DXBCデコーダの一般化)**:
     - 一般化の軸として「N個の逐次2項演算(制御フロー無し)」を選択。新規シェーダー`shaders/vector_add_mul_chain.hlsl`(`t = InputA[i] + InputB[i]; Output[i] = t * InputA[i];`)を`fxc.exe /T cs_5_0`で実際にコンパイル。**UAV本数は当初4本(A/B/C/Out)を狙ったが、`opencuda-vulkan::VulkanDevice::launch_kernel`が`"vector_add"`/`"matmul"`いずれも厳密に3バッファ固定の引数配線(`ensure_vector_add_args`/`ensure_matmul_args`、`open-cuda`側は今回変更しない方針)しか持たないと実ソースで確認したため、UAV3本(`InputA`を2回参照)へ設計変更した**——正直な開示としてCLAUDE.md/PORTING.mdに明記。
     - `examples/dump_shex.rs`で実SHEX命令列をダンプして確認したところ、**予想に反して`dcl_temps`は1個のまま**だった。fxcは`t`と`InputA[i]`の2回目の参照を別々の一時レジスタにせず、`r0.x`/`r0.y`という同一レジスタの別コンポーネントへ詰め込んでいた。さらに**2回目の`InputA[i]`参照は`ld_structured`を再発行せず、最初のロード結果(共通部分式除去/CSE)をそのまま再利用していた**——これは予期していなかった実発見。
     - 実装(`crates/directx-shader-translate/src/spirv_gen.rs`、既存の`decode_shader_shape`/`ShaderShape`/`translate_shader`は一切変更していない、別のパターンクラスとして追加): `RegExpr`(`Load(uav)` / `BinOp(op, lhs, rhs)`という評価式の木)・`decode_chain_shape`(`ld_structured`/`add`/`mul`/`store_structured`を実際に走査し、`HashMap<(temp_index, component), RegExpr>`へ一時レジスタコンポーネントの内容を追跡、`store_structured`が最終的に参照するコンポーネントから逆算して式木全体を構築する。1回でも2回でもN回でも、fxcがCSEで詰め込んでいても同じロジックで扱える——「シェーダー5個目の形をそのままハードコードする」のではなく、マッチングロジック自体を一般化した)。`sub`(negateフラグ)・`div`はチェーン内では意図的に未対応のまま(オペランド順序の意味を単一演算ケースでしか検証していないため、正直な開示)。`translate_chain_shader`(公開API、`ChainTranslatedKernel{read_uav_bind_points: Vec<u32>, write_uav_bind_point: u32, ...}`——既存`TranslatedKernel`の3要素固定タプルでは表現できないため別の型として定義)・`emit_chain_spirv`(式木を実際に再帰(post-order)でSPIR-Vへ翻訳、`Load`ごとに`OpAccessChain`+`OpLoad`、`BinOp`ごとに`OpFAdd`/`OpFMul`)。
     - **実Vulkanテスト追加**: `tests/vector_add_mul_chain_real_vulkan.rs`(既存4本と同じパターン)、CPU参照実装`(a[i]+b[i])*a[i]`と256要素すべてで数値一致を実機(NVIDIA GT 730)で検証。
     - **既存4形状+DXILへの回帰が無いことを確認**: `chain_translator_also_accepts_the_pre_existing_single_op_vector_add_shader`で、既存の`vector_add.dxbc`が新設のチェーンクラス(N=1の自明な場合)としても正しく翻訳できることを追加確認(排他的である必要はなく、単に別のパターンクラスとして共存する)。
  3. **実際に`cargo test --workspace --lib`で確認した結果(誇張なし、実出力そのまま)**:
     ```
     running 28 tests
     ...(dxil::tests::extracts_real_numthreads_from_dxil_metadata_block_not_hardcoded ... ok
      dxil::tests::translate_dxil_vector_add_to_spirv_uses_extracted_not_hardcoded_local_size ... ok
      dxil::tests::finds_numthreads_pair_even_when_a_different_value_precedes_it ... ok
      spirv_gen::tests::translates_real_fxc_compiled_vector_add_mul_chain_dxbc_to_valid_spirv ... ok
      spirv_gen::tests::chain_translator_also_accepts_the_pre_existing_single_op_vector_add_shader ... ok
      spirv_gen::tests::chain_translator_honestly_rejects_garbage_bytes ... ok を含む)
     test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     ```
     `cargo test --workspace --test '*' -- --nocapture`(実機、NVIDIA GeForce GT 730、全6本green):
     ```
     device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
     OK: DXIL(dxc.exe実コンパイル、SM6.0)->SPIR-V(自前生成)->実Vulkan(NVIDIA GT 730)経路が、CPU参照実装(a[i]+b[i])と256要素すべてで数値一致した
     test dxil_vector_add_matches_cpu_reference_on_real_vulkan_hardware ... ok

     device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
     OK: DXBC(fxc.exe実コンパイル, 2項演算2回のチェーン)->SPIR-V(自前生成、式木の再帰翻訳)->実Vulkan経路が、CPU参照実装((a[i]+b[i])*a[i])と256要素すべてで数値一致した
     c[0]=65, c[255]=708.875
     test dxbc_vector_add_mul_chain_matches_cpu_reference_on_real_vulkan_hardware ... ok

     device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
     OK: DXBC(fxc.exe実コンパイル)->SPIR-V(自前生成)->実Vulkan(NVIDIA GT 730)経路が、CPU参照実装(a[i]+b[i])と256要素すべてで数値一致した
     test dxbc_vector_add_matches_cpu_reference_on_real_vulkan_hardware ... ok

     device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
     OK: DXBC(fxc.exe実コンパイル, div)->SPIR-V(自前生成)->実Vulkan経路が、CPU参照実装(a[i]/b[i])と256要素すべてで数値一致した
     test dxbc_vector_div_matches_cpu_reference_on_real_vulkan_hardware ... ok

     device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
     OK: DXBC(fxc.exe実コンパイル, mul)->SPIR-V(自前生成)->実Vulkan経路が、CPU参照実装(a[i]*b[i])と256要素すべてで数値一致した
     test dxbc_vector_mul_matches_cpu_reference_on_real_vulkan_hardware ... ok

     device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
     OK: DXBC(fxc.exe実コンパイル, sub+境界チェック)->SPIR-V(自前生成)->実Vulkan経路が、CPU参照実装(a[i]-b[i])と有効範囲256要素すべてで数値一致し、境界外の64要素はセンチネル値のまま(書き込まれなかった)ことを確認した
     test dxbc_vector_sub_bounded_matches_cpu_reference_and_respects_bounds_on_real_vulkan_hardware ... ok
     ```
     ワークスペース全体: unittests 28件 + 実機テスト6本(DXBC単一演算4本+DXBCチェーン1本+DXIL1本)全green。`cargo build --workspace`/`cargo clippy --workspace --all-targets`はいずれも警告0件。
  4. **正直な開示・まだやっていないこと(誇張しない)**:
     - **DXIL側**: `numthreads`の実抽出はこのシェーダー(`vector_add_dxil.hlsl`)専用の経路検証。他のDXILシェーダー(mul/sub/div相当・複数エントリポイント等)でも同じ`METADATA_BLOCK`構造が成り立つかは未検証(この1シェーダーでの実測に基づく)。
     - **DXBC側**: チェーンクラスは`add`/`mul`のみ対応、`sub`/`div`のチェーン内混在・3項以上への一般化(3回以上の逐次演算)は今回実測していない(理屈上`decode_chain_shape`のロジックはN回まで対応できる形にはなっているが、実際に3回以上の演算を持つシェーダーをコンパイルして検証してはいない)。境界チェックはチェーンクラスでは非対応のまま。
     - D3D11グラフィックスパイプライン(VS/PS向けSPIR-V生成・ラスタライズ・実描画)は前回エントリから変更なし(未着手のまま)。
  - 次にすべきこと: (1) DXIL側: `resolve_vector_add_dxil_calls`/`translate_dxil_vector_add_to_spirv`をmul/sub/div相当のDXILシェーダーへ一般化(DXBC側で先行実施済みのアプローチを踏襲)。(2) DXBC側: チェーンクラスへ`sub`/`div`のオペランド順序を実際に検証した上で対応追加、3項以上の実シェーダーでの検証。(3) D3D11グラフィックスパイプライン(VS/PS)は引き続き並行で検討可。

- **2026-07-26 D3D11グラフィックスパイプライン: VS/PS向け実SPIR-V生成に到達・2通りの実ツールで検証(ラスタライザ/実描画は未着手、正直な区切り)**:
  1. **前提の再確認**: 前回(2026-07-25続き4)の到達点通り、`triangle_vs.dxbc`/`triangle_ps.dxbc`は既存の`parse_dxbc`で問題なくパースでき、`translate_shader`(Compute専用)は両方とも`SpirvGenError::UnsupportedShader`で正しく拒否する状態だった。`examples/dump_shex.rs`で改めて実SHEX命令列を番号付きでダンプし直し、VSが実際に9命令(`dcl_globalFlags` -> `dcl_input`(v0, mask=7=xyz, POSITION) -> `dcl_input`(v1, mask=15=xyzw, COLOR) -> `dcl_output_siv`(o0, mask=15, SV_POSITION) -> `dcl_output`(o1, mask=15, COLOR) -> `mov o0.xyz, v0.xyzx` -> `mov o0.w, l(1.0)` -> `mov o1.xyzw, v1.xyzw` -> `ret`)、PSが実際に5命令(`dcl_globalFlags` -> `dcl_input_ps`(linear, v1, mask=15, COLOR) -> `dcl_output`(o0, mask=15) -> `mov o0.xyzw, v1.xyzw` -> `ret`)であることを確認した(HANDOFFの前回記述「mov×3」「mov×1」と一致、より詳細に番号・オペランド形状まで裏取り)。
  2. **実装**: `crates/directx-shader-translate/src/spirv_gen.rs`へ、既存のCompute Shader向けコード(`decode_shader_shape`/`decode_chain_shape`とその周辺、`emit_spirv_impl`/`emit_chain_spirv`)を一切変更せず、独立した新セクションを追加。
     - `decode_vertex_shader_shape`/`decode_pixel_shader_shape`: 上記の実命令列を1命令ずつ厳密に突き合わせる(この2シェーダーはパススルーのみで可変要素が無いため、Compute側のように値を抽出するのではなく、形状の一致検証のみを行う)。1つでも一致しなければ`SpirvGenError::UnsupportedShader`で拒否する。
     - `emit_vertex_spirv`/`emit_pixel_spirv`: 検証を通った場合にのみ返す固定のSPIR-Vモジュールを`rspirv::dr::Builder`で組み立てる。Compute側(`GLCompute`実行モデル、storage buffer+push constant)とは根本的に異なる形: `OpEntryPoint Vertex`/`Fragment`、`Input`/`Output`ストレージクラス変数+`Location`デコレーション、頂点シェーダーの`SV_POSITION`出力への`BuiltIn Position`デコレーション(`vec3`のPOSITION入力から`vec4`を`OpCompositeConstruct`で組み立て、`.w`は実際の`mov o0.w, l(1.0)`命令通り定数1.0)、フラグメントシェーダーへの`OpExecutionMode ... OriginUpperLeft`(Vulkan必須、DXBC側に対応物は無く追加した)。
     - `translate_vertex_shader`/`translate_pixel_shader`(新設公開API)。
  3. **2通りの実検証(誇張なし、実出力そのまま)**:
     - (1) `rspirv`自身のローダーで再パース: 新規テスト`translates_real_fxc_compiled_triangle_vs_dxbc_to_valid_vertex_spirv`/`_triangle_ps_dxbc_to_valid_fragment_spirv`が、`rspirv::binary::parse_bytes`が成功し再パース後のモジュールに`OpEntryPoint Vertex`/`Fragment`が実際に含まれることを検証。
     - (2) 実Vulkan SDK付属の`spirv-val.exe`による外部検証: 新設`examples/dump_graphics_spirv.rs`で両モジュールをファイルへ書き出し、実際に実行した結果:
       ```
       $ /c/VulkanSDK/1.4.350.0/Bin/spirv-val.exe ./triangle_vs.spv
       exit=0
       $ /c/VulkanSDK/1.4.350.0/Bin/spirv-val.exe ./triangle_ps.spv
       exit=0
       ```
       (`spirv-val`は成功時に何も出力しないため、上記の空出力+終了コード0がそのまま実結果。)
  4. **回帰防止テスト追加**: `vertex_translator_honestly_rejects_the_pixel_shader_and_vice_versa`(VS用デコーダにPSのDXBCを渡す、逆も同様——いずれも拒否されることを確認)・`graphics_translators_honestly_reject_garbage_bytes`・`compute_translators_still_honestly_reject_graphics_shaders`(既存の`translate_shader`/`translate_chain_shader`が引き続きVS/PSを拒否することの継続確認)。
  5. **実際に`cargo test --workspace --lib`で確認した結果(誇張なし、実出力そのまま)**: ワークスペース全体で33件全green(既存27件+新規6件)。`cargo test --workspace --test '*' -- --nocapture`で既存の実機Compute Shaderテスト6本(DXBC単一演算4本+DXBCチェーン1本+DXIL1本)も全green・変更なしを再確認(この増分は追加のみで既存経路への回帰無し)。`cargo build --workspace`/`cargo clippy --workspace --all-targets`はいずれも警告0件。
  6. **タスク指示のstep 3調査結果(実ソースを実際に読んで確認)**: `../open-cuda/crates/opencuda-vulkan`(`src/lib.rs`/`src/real.rs`)を実際に読んだ結果、`VkGraphicsPipelineCreateInfo`・レンダーパス・フレームバッファに関するコードは一切存在せず、`launch_kernel`によるCompute専用ディスパッチのみであることを確認した(`grep -rl "GraphicsPipeline\|VkGraphicsPipeline\|RenderPass\|Framebuffer"`が0件)。`ash`はこのワークスペースには`opencuda-vulkan`の`real-vulkan`フィーチャ経由でのみ間接的に来ており、Compute経路専用。したがって実際に三角形をVulkanへディスパッチしてピクセルを読み戻すには、(a)`opencuda-vulkan`側にグラフィックスパイプライン対応を追加する(本プロジェクトの「open-cudaに依存するのみで変更しない」方針によりスコープ外)か、(b)`open-directx`自身に`ash`を直接の依存として追加し、最小限のグラフィックスパイプライン(レンダーパス・フレームバッファ・描画コマンド・読み戻し)を自前で組む、のいずれかが必要——これは正直な現状の開示であり、今回のパスでは(b)には着手していない。
  7. **正直な開示・まだやっていないこと(誇張しない)**:
     - **汎用VS/PSデコーダではない**。今回対応したのは`triangle_vs.hlsl`/`triangle_ps.hlsl`が実際に生成する固定の命令列(可変要素・抽出対象値が一切無い、パススルーのみ)専用。別の頂点/ピクセルシェーダー(異なるセマンティクス・複数の`mov`連鎖・テクスチャサンプル・複数レンダーターゲット等)を渡せば`SpirvGenError::UnsupportedShader`で拒否される。
     - **ラスタライザ・出力マージ・実際のVulkan描画コマンド・フレームバッファ読み戻しは一切実装していない**(タスク指示の到達し得る範囲として明記された「SPIR-V生成+検証まで」を正直な区切りとした)。
     - DXIL(SM6+)・Compute Shader側(DXBC/DXIL双方)は前回エントリから変更なし(既存の対応範囲・実Vulkan検証範囲に変更無し)。
  - 次にすべきこと: (1) `open-directx`自身へ`ash`を直接の依存として追加し、最小限のグラフィックスパイプライン(レンダーパス+フレームバッファ+`VkGraphicsPipelineCreateInfo`+描画コマンド+読み戻し)を実装し、今回生成したSPIR-V2本(VS/PS)を実際にVulkanへディスパッチして三角形を描画、フレームバッファから読み戻したピクセル色がパススルー元の色と一致することを実機(NVIDIA GT 730)で確認する——これが達成できれば「D3D11最小グラフィックスパイプライン」の完全なマイルストーンとなる。(2) DXIL側・DXBC Compute側の一般化継続は前回エントリの記載通り。

- **2026-07-26(続き) D3D11最小グラフィックスパイプラインのマイルストーン達成: `ash`を新規クレートとして直接依存に追加し、実レンダーパス+フレームバッファ+`VkGraphicsPipelineCreateInfo`+実描画コマンド+実読み戻しをNVIDIA GT 730で実証**:
  1. **新規クレート`crates/directx-graphics-vulkan`**(`ash = "0.37"`を直接の依存として追加。`open-cuda`の`opencuda-vulkan`は`launch_kernel`によるCompute専用ディスパッチのみで`GraphicsPipeline`/`RenderPass`/`Framebuffer`関連のコードが一切無いことを前回エントリで実ソース監査済みのため、それをラップせず、指示通り`open-directx`自身の別クレートとして実装した。`open-cuda`側は一切変更していない)。
  2. **実装内容**(`src/lib.rs`、`render_uniform_triangle_and_read_back(vs_spirv, ps_spirv, vertex_color, width, height)`): 実際に`vkCreateInstance`→グラフィックスキューファミリを持つ物理デバイスの列挙→`vkCreateDevice`→デバイスローカルな`R8G8B8A8_UNORM`カラーアタッチメント画像(レンダーターゲット兼コピー元)→レンダーパス(1カラーアタッチメント、`finalLayout=TRANSFER_SRC_OPTIMAL`にして明示的なバリアを使わずコピー用レイアウト遷移をレンダーパス自体に行わせる設計)→フレームバッファ→ホスト可視頂点バッファ(POSITION vec3+COLOR vec4、`triangle_vs`のSPIR-V入力レイアウトのLocation 0/1と一致)→`translate_vertex_shader`/`translate_pixel_shader`が生成した実SPIR-Vバイト列からの`vkCreateShaderModule`(コード側でシェーダー翻訳を再実装せず再利用、タスク指示通り)→`VkGraphicsPipelineCreateInfo`(頂点入力・入力アセンブリ・ビューポート/シザー・ラスタライズ・マルチサンプル・カラーブレンドの各ステートを実際に構築)→コマンドバッファ記録(`vkCmdBeginRenderPass`でクリア色黒→`vkCmdBindPipeline`→`vkCmdBindVertexBuffers`→`vkCmdDraw(3,1,0,0)`→`vkCmdEndRenderPass`→`vkCmdCopyImageToBuffer`でホスト可視読み戻しバッファへコピー)→`vkQueueSubmit`+フェンス待機→`vkMapMemory`で実際にRGBA8ピクセル列を読み出す、という一気通貫の実装。
  3. **検証手法(CPU参照実装相当のチェック)**: 頂点色による補間の曖昧さを避けるため、3頂点すべてに同一の`vertex_color`(`(200,100,50,255)`のRGBA8相当)を与え、かつビューポート全体を覆う「大きい三角形」(NDC座標`(-1,-1),(3,-1),(-1,3)`——三角形1枚でビューポート全体をカバーする定番手法)を描画した。パススルーの頂点/ピクセルシェーダーが正しく動作していれば、読み戻したフレームバッファの全ピクセルが入力色と(UNORM量子化の丸め誤差±1以内で)完全一致するはずであり、これがCPU側の「期待値」に相当する。
  4. **実際に`cargo test -p directx-graphics-vulkan --test triangle_real_vulkan -- --nocapture`で確認した結果(誇張なし、実出力そのまま、NVIDIA GeForce GT 730)**:
     ```
     running 1 test
     OK: D3D11 minimal graphics pipeline (real ash-driven render pass + framebuffer + VkGraphicsPipelineCreateInfo) drew a full-viewport triangle using triangle_vs.dxbc/triangle_ps.dxbc's real translated SPIR-V, and all 4x4 read-back pixels matched the passthrough vertex color Rgba8 { r: 200, g: 100, b: 50, a: 255 } on the real GPU present on this machine.
     test d3d11_triangle_draw_call_matches_passthrough_vertex_color_on_real_vulkan_hardware ... ok

     test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s
     ```
     4x4=16ピクセル全て(全チャンネルR/G/B/A)がパススルー元の頂点色と一致することを実際に確認した(GPUがフェイクデータを返していないこと、シェーダーモジュールが実際にロードされ実行されたことの直接証拠)。
  5. **ワークスペース全体の実際の検証結果**: `cargo test --workspace`で**unittests 33件+実機テスト7本(DXBC Compute 4本+DXBCチェーン1本+DXIL Compute 1本+今回のグラフィックス1本)全green**(既存6本の実機Computeテストに変更・回帰なし)。`cargo build --workspace`/`cargo clippy --workspace --all-targets`はいずれも**警告0件**(`manual_find`警告1件を`Iterator::find`ベースの実装へ書き換えて解消済み)。
  6. **正直な開示・まだやっていないこと(誇張しない)**:
     - **汎用グラフィックスパイプラインではない**。`triangle_vs.hlsl`/`triangle_ps.hlsl`専用のパススルーSPIR-V2本を、固定の頂点データ(1つの大三角形・単色)で1回描画するだけの単一シナリオ。深度バッファ・テクスチャ・複数レンダーターゲット・スワップチェーン・実ウィンドウ表示は一切無い。
     - **`open-cuda`側は一切変更していない**(指示通り)。今回のグラフィックスパイプラインは`open-directx`自身が保持する独立したVulkanインスタンス/デバイス経由(Compute側の`opencuda-vulkan`とは別のVulkanコンテキスト)。将来的に同一デバイス/コンテキストを共有する設計にするかどうかは未検討(現時点ではCompute経路とグラフィックス経路が完全に独立している)。
     - 補間の検証(頂点ごとに異なる色を与えて三角形内部のグラデーションが正しく補間されるか)は今回のテストでは意図的に検証していない(全頂点同色にしてパススルーの正しさのみを曖昧さ無く確認する設計を選んだため)。次の増分候補。
  - 次にすべきこと: (1) 異なる頂点色での補間検証(グラデーション三角形)によるラスタライザの補間ロジックそのものの検証。(2) DXIL側のmul/sub/div一般化、DXBC Computeチェーンクラスのsub/div対応(前回エントリから継続、今回は未着手)。(3) 深度バッファ・複数三角形・インデックスバッファ等、より本格的なD3D11描画コマンドへの拡張。

- **2026-07-26(続き2) DXIL側のmul/sub/div一般化を達成(前回HANDOFFの次項(2)前半)、実Vulkanで4演算全て検証。作業中断からの再開時に見つけた実バグ2件も修正**:
  1. **実装**: `dxil.rs`に`resolve_dxil_calls_and_binop`(既存`resolve_vector_add_dxil_calls`を一般化、`BinOp`〈`FUNC_CODE_INST_BINOP`〉のオペコード〈add=0/sub=1/mul=2/div=4、LLVM `GetEncodedBinaryOpcode`規約〉とオペランド順序〈`lhs_range_id`/`rhs_range_id`〉も解決する`ResolvedDxilBinOp`を追加)、`translate_dxil_binary_op_to_spirv`(旧`translate_dxil_vector_add_to_spirv`をadd専用から一般化、後方互換のため後者は前者への薄いエイリアスとして残置)を実装。新規シェーダー3本(`vector_mul_dxil.hlsl`/`vector_sub_dxil.hlsl`/`vector_div_dxil.hlsl`、`vector_add_dxil.hlsl`と同一契約・演算のみ異なる)を`dxc.exe -T cs_6_0`で実コンパイル、`tools/compile-dxbc-shaders.ps1`に追記。
  2. **セッション中断からの再開で発覚した実バグ2件(このパスで修正)**: 前回セッションが検証途中(`cargo test --workspace`未実行)で中断しており、再開後に実際にテストを回したところ2つの問題が見つかった。
     - **(a) `lib.rs`のexport漏れ**: `translate_dxil_binary_op_to_spirv`が`dxil`モジュール内に実装されていたが`pub use dxil::{...}`に含まれておらず、新設の実機テスト(`tests/vector_mul_sub_div_dxil_real_vulkan.rs`)がコンパイルエラーになっていた。`lib.rs`のexportリストへ追加して解消。
     - **(b) mul/addのBinOpオペランド順序に関する誤った手計算前提**: 前回セッションが「add.dxilと同じCreateHandle順序だからmulもlhs=u0,rhs=u1のはず」と手計算のみで書いたテスト期待値(`(0,1)`)が、実際に`resolve_dxil_calls_and_binop`を実行すると`(1,0)`を返し、失敗した(mulだけでなく**addも同様に`(1,0)`**であることを、既存の実機テスト`vector_add_dxil_real_vulkan.rs`の同種のハードコード済みアサーションが落ちたことで確認)。原因はadd/mulが可換演算のため、dxc/LLVMの最適化パスがオペランドの相対値参照順序を(sub/divとは独立に)並べ替えていたこと。**数値的には可換なので実行結果自体は正しい**——問題は「手計算のみに頼った期待値」であって、`resolve_dxil_calls_and_binop`の解決ロジック自体にバグは無かった(実際にVulkanへディスパッチしてCPU参照実装と数値一致することで裏付け済み)。該当する3箇所のテスト・アサーション(`resolves_mul_binop_from_real_dxc_compiled_dxil`・`translate_dxil_binary_op_to_spirv_handles_mul_sub_div_not_just_add`・`vector_add_dxil_real_vulkan.rs`)を、可換演算(add/mul)は読み出し元2本を`{u0,u1}`の集合として検証、非可換演算(sub/div)は引き続き順序`(0,1)`を厳密に検証する形へ修正した。**教訓として明記**: 手計算トレースだけに頼らず必ず実行結果で裏取りする、という既存方針(このリポジトリのHANDOFF随所に既出)が、まさにこの箇所で機能した具体例。
  3. **実際に`cargo test --workspace --release -- --nocapture`で確認した結果(誇張なし、実出力そのまま、NVIDIA GeForce GT 730)**:
     ```
     device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730) (div)
     OK: DXIL(dxc.exe実コンパイル, div)->SPIR-V(自前生成、resolve_dxil_calls_and_binopで演算/オペランド順序を実解決)->実Vulkan経路が、CPU参照実装と256要素すべてで数値一致した
     device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730) (mul)
     device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730) (sub)
     test dxil_vector_div_matches_cpu_reference_on_real_vulkan_hardware ... ok
     test dxil_vector_sub_matches_cpu_reference_on_real_vulkan_hardware ... ok
     test dxil_vector_mul_matches_cpu_reference_on_real_vulkan_hardware ... ok
     test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     ```
     `dxil_vector_add_matches_cpu_reference_on_real_vulkan_hardware`(既存)も修正後green。ワークスペース全体: unittests 37件+実機テスト8本(DXBC Compute 4本+DXBCチェーン1本+DXIL Compute 4本〈add/mul/sub/div〉+グラフィックス1本)全green(実機NVIDIA GT 730、スキップ無し)。`cargo build --workspace`/`cargo clippy --workspace --all-targets --release`はいずれも警告0件。
  4. **正直な開示・まだやっていないこと**: DXBC Computeチェーンクラス(`decode_chain_shape`)へのsub/div対応は今回未着手(前回HANDOFFの次項(2)後半)。D3D11グラフィックスパイプラインの補間検証・深度バッファ等も未着手のまま(次項(1)(3)、変更なし)。
  - 次にすべきこと: (1) 異なる頂点色での補間検証(グラデーション三角形)。(2) DXBC Computeチェーンクラスのsub/div対応、3項以上の実シェーダーでの検証。(3) 深度バッファ・複数三角形・インデックスバッファ等、より本格的なD3D11描画コマンドへの拡張。

- **2026-07-27(続き) DXBCチェーンクラス(`decode_chain_shape`)にsub/div
  対応を追加——複数エントリにわたって「次にすべきこと」に残っていた
  項目を実際に検証・解消(以前は「1シェーダーだけでは正しいオペランド
  順序を検証しきれない」として明示的に拒否していた)**:
  1. **新規シェーダー`vector_sub_div_chain.hlsl`**(`t = InputA[i] -
     InputB[i]; Output[i] = t / InputA[i];`、既存の`vector_add_mul_chain.hlsl`
     と同じUAV3本・InputA多重参照パターン、演算のみsub/divに変更)を
     実際に`fxc.exe /T cs_5_0`でコンパイルし、`examples/dump_shex`で
     実SHEX命令列をダンプして正しいオペランド順序を確認した(推測では
     実装していない)。
  2. **実際に確認したオペランド順序の規約**: `Add`命令でsrc1オペランド
     (operands[1])に`negate`フラグが立っている場合、
     `dest = src2_val - src1_val`(既存の`decode_shader_shape`と同じ
     「negated-addはsub」規約が、このチェーンクラス内でも成立することを
     実際に確認)。`Div`はこれと逆で`dest = src1_val / src2_val`
     (swapしない、モジュールのdocコメントに以前から記載されていた
     「divはadd/mulとオペランド順序が異なる」という記述が正しかったことを
     このチェーンクラスでも裏付けた)。
  3. **`spirv_gen.rs`の`decode_chain_shape`を拡張**: `Opcode::Add | Mul`
     のみを扱っていたマッチアームを`Add | Mul | Div`に拡張し、上記の
     規約に従って`RegExpr::BinOp`を構築するようにした。`mul`の
     negateフラグは今回のシェーダーでは発生しなかったため未検証のまま
     明示的に拒否する(正直な開示、誤って「対応している」というシグナルを
     出さない)。
  4. **新規実機テスト`dxbc_vector_sub_div_chain_matches_cpu_reference_on_
     real_vulkan_hardware`を追加**(`tests/vector_sub_div_chain_real_vulkan.rs`、
     既存の`vector_add_mul_chain_real_vulkan.rs`と同じ構成)。
  5. **実際に`cargo test -p directx-shader-translate --test
     vector_sub_div_chain_real_vulkan -- --nocapture`で確認した結果
     (誇張なし、実出力そのまま、NVIDIA GeForce GT 730)**:
     ```
     device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
     OK: DXBC(fxc.exe実コンパイル, 2項演算2回のチェーン sub+div)->SPIR-V(自前生成、式木の再帰翻訳)->実Vulkan経路が、CPU参照実装((a[i]-b[i])/a[i])と256要素すべてで数値一致した
     test dxbc_vector_sub_div_chain_matches_cpu_reference_on_real_vulkan_hardware ... ok
     test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.62s
     ```
  6. **ワークスペース全体の検証**: `cargo test --workspace`で既存の
     実機テスト群(グラフィックス3本+DXBC Compute4本+DXBCチェーン
     (add/mul)1本+DXIL Compute4本+今回のsub/divチェーン1本)全green、
     回帰なし。
  7. **正直な開示・まだ残る制約**: (1) `mul`のnegateフラグが立つケースは
     未検証のまま(遭遇時は明示的にエラー)。(2) このシェーダー1本
     (1回のsub+1回のdiv)以外の組み合わせ・オペランド並び(例: 3項以上の
     チェーンでのsub/divの重複使用)は未検証。(3) 境界チェック
     (`ult`/`if`/`endif`)は引き続きチェーンクラスの対象外(既存クラス側
     のみが対応)。
  - 次にすべきこと: (1) Compute/Graphics両経路でのVulkanデバイス共有の
    設計検討(前々回エントリから継続)、(2) 3項以上の実チェーンシェーダー
    でのsub/div組み合わせ検証、(3) `mul`のnegateフラグケースの検証。

- **2026-07-27 グラフィックスパイプライン側にもGPUベンダー診断を追加
  ——SET連携(open-directx/open-cuda/aruaru-llm)調査で判明していた
  「open-cudaのCompute経路には`vendor_from_id`によるベンダー判定がある
  のに、open-directx側のGraphics経路には同等の診断が無い」という非対称な
  機能ギャップを解消(ユーザー指示: 3リポジトリをSETとして連携の実用性・
  完成度を高める作業の一環)**:
  1. **`crates/directx-graphics-vulkan/src/lib.rs`に`enumerate_graphics_
     devices()`を新規追加**: `render_uniform_triangle_and_read_back`等が
     物理デバイスを選ぶのと同じ基準(グラフィックスキューファミリを
     持つこと)で全物理デバイスを列挙し、各デバイスの名前・vendor ID・
     ベンダー名(`vendor_name_from_id`、NVIDIA/AMD/Intel/Qualcomm/ARM/
     Imagination PowerVRのPCI vendor IDテーブル)を返す診断専用関数
     (論理デバイス生成・描画は一切行わない、既存の描画関数には無影響)。
     `opencuda-vulkan`への依存は追加していない——同リポジトリの
     `vendor_from_id`と同じ小さなテーブルを意図的に独立実装した(前回
     エントリで確認済みの「Compute経路とGraphics経路は完全に独立した
     Vulkanコンテキストを持つ」という設計方針を維持するため)。
  2. **新規実機テスト`enumerate_graphics_devices_reports_the_real_gpu_
     on_this_machine`を追加**(`tests/triangle_real_vulkan.rs`): 実際に
     この関数を呼び、実機(NVIDIA GeForce GT 730)が実際に列挙され、
     デバイス名が空でないことを確認。
  3. **検証(実測)**: `cargo test -p directx-graphics-vulkan`
     3件全green(新規1件含む)。`cargo test --workspace`でも既存の実機
     テスト群(DXBC Compute 4本+DXBCチェーン1本+DXIL Compute 4本+
     グラフィックス3本)全green、回帰なし。
  4. **正直な開示**: (1) 複数GPUベンダー環境での実機検証は今回もできて
     いない(このマシンにはNVIDIA GT 730 1台のみ)——`vendor_name_from_id`
     のAMD/Intel/Qualcomm/ARM/PowerVR分岐は`opencuda-vulkan`側と同様、
     型チェックのみで実機列挙では未検証。(2) Compute経路とGraphics経路の
     Vulkanコンテキスト自体の統合(前回エントリで「未検討」と記載した
     項目)には手を付けていない——今回はあくまで診断情報の並列実装に
     留めた。
  - 次にすべきこと: (1) Compute/Graphics両経路でのVulkanデバイス
    共有の設計検討(前回エントリから継続)、(2) DXBC Computeチェーン
    クラスのsub/div対応(前々回エントリから継続)。

- **2026-07-26(続き3) 異なる頂点色での補間検証(グラデーション三角形)を実装——直前2エントリの「次にすべきこと(1)」を解消(ユーザー指示: runo.tokyo/open-directx/open-cuda/aruaru-llm等7リポジトリの未着手・未完成事項の洗い出し→実装継続、DirectX12/GPU互換性の並行開発の一環)**:
  1. **`crates/directx-graphics-vulkan/src/lib.rs`を拡張**: 既存の`render_uniform_triangle_and_read_back(vs_spirv, ps_spirv, vertex_color: [f32;4], width, height)`を、内部の非公開関数`render_triangle_and_read_back(vs_spirv, ps_spirv, vertex_colors: [[f32;4];3], width, height)`(3頂点それぞれに個別の色を割り当てられるよう一般化)への薄いラッパーとして書き直し(`[vertex_color; 3]`を渡すだけ、既存の公開APIのシグネチャ・挙動は完全に維持——既存テスト`d3d11_triangle_draw_call_matches_passthrough_vertex_color_on_real_vulkan_hardware`は無変更のままgreen)。新規に公開関数`render_gradient_triangle_and_read_back(vs_spirv, ps_spirv, vertex_colors: [[f32;4];3], width, height)`を追加し、同じ内部関数へ委譲。頂点バッファ構築部分(210行目付近)を`vertex_colors[0/1/2]`をそれぞれ`(-1,-1)`/`(3,-1)`/`(-1,3)`の各頂点へ割り当てる形に変更(HLSL/SPIR-V側は既に`COLOR`セマンティクスをパススルーする構造だったため、シェーダー自体の変更は不要——前回の実機調査で判明していた「シェーダーは補間対応済みだがRust側APIが単色専用だった」というギャップに対する対応)。
  2. **新規実機テスト`d3d11_triangle_draw_call_interpolates_distinct_per_vertex_colors_on_real_vulkan_hardware`を追加**(`tests/triangle_real_vulkan.rs`): 3頂点にそれぞれ純色の赤`(1,0,0,1)`・緑`(0,1,0,1)`・青`(0,0,1,1)`を割り当てて8x8ピクセルで実描画。ピクセル位置とNDC座標の対応(ビューポートのY軸方向等)をハードコードして期待色を逆算する方式は、検証コード自体がレンダラと同じ前提のミスを共有しかねないため意図的に避け、代わりに次の2つのアフィン補間の性質を実際の読み戻しピクセルで検証する設計にした: (a) 純色R/G/Bは「単位分割」(`u+v+w=1`のときr+g+b=255の凸結合)をなすため、カバーされる全ピクセルで`r+g+b`が255±2に収まること(バリセントリック補間が正しくアフィンであることの直接証拠)、(b) 全ピクセルが同一色ではない(=真に位置ごとに補間されている、「常に頂点0の色を出すだけ」という縮退バグを(a)だけでは検出できないため追加した別の観点)。
  3. **実際に`cargo test -p directx-graphics-vulkan --test triangle_real_vulkan -- --nocapture`で確認した結果(誇張なし、実出力そのまま、NVIDIA GeForce GT 730)**:
     ```
     running 2 tests
     OK: D3D11 minimal graphics pipeline correctly interpolates distinct per-vertex colors (pure red/green/blue) across all 8x8 read-back pixels on the real GPU present on this machine — every pixel's r+g+b sums to ~255 and the image is not a single flat color.
     test d3d11_triangle_draw_call_interpolates_distinct_per_vertex_colors_on_real_vulkan_hardware ... ok
     OK: D3D11 minimal graphics pipeline (real ash-driven render pass + framebuffer + VkGraphicsPipelineCreateInfo) drew a full-viewport triangle using triangle_vs.dxbc/triangle_ps.dxbc's real translated SPIR-V, and all 4x4 read-back pixels matched the passthrough vertex color Rgba8 { r: 200, g: 100, b: 50, a: 255 } on the real GPU present on this machine.
     test d3d11_triangle_draw_call_matches_passthrough_vertex_color_on_real_vulkan_hardware ... ok
     test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s
     ```
  4. **ワークスペース全体の検証**: `cargo test --workspace`で既存の実機テスト7本(DXBC Compute 4本+DXBCチェーン1本+DXIL Compute 4本のうち一部重複カウント訂正、実際には既存8本)+新規1本、全green。既存の単色テストへの回帰なし。
  5. **正直な開示・まだやっていないこと**: (1) ピクセル位置とNDC座標の厳密な対応(ビューポートのY軸方向、`(row,col)`→NDC変換式)は今回検証していない——上記の設計判断により、その変換式に依存しない不変量(単位分割・非一様性)のみを検証した。ピクセル単位の厳密な期待値照合(例えば中心ピクセルが理論値とどれだけ一致するか)は次の増分候補として残る。(2) 深度バッファ・複数三角形・インデックスバッファ・テクスチャ・スワップチェーン・実ウィンドウ表示は引き続き一切未実装(前回・前々回エントリと同じ既知の制約)。
  - 次にすべきこと: (1) ピクセル位置↔NDC座標の変換式を明示的に検証するテスト(現在は変換式に依存しない不変量チェックのみ)、(2) DXBC Computeチェーンクラスのsub/div対応・3項以上の実シェーダーでの検証(前々回エントリから継続)、(3) 深度バッファ・複数三角形・インデックスバッファ等、より本格的なD3D11描画コマンドへの拡張。

- **2026-07-27(続き4) 「まず動かして見る」用のrunnableデモを追加(使いやすさ改善、ユーザー指示「open-directx と open-cuda と aruaru-llmのSETの完成度と実用性と使いやすさの向上をお願い」)**:
  1. **`crates/directx-graphics-vulkan/examples/render_triangle.rs`を新設**:
     `tests/triangle_real_vulkan.rs`と同じ実DXBC(fxc.exeコンパイル済み)→
     SPIR-V変換済みシェーダーを使い、実Vulkanハードウェア上にグラデーション
     三角形(赤/緑/青の頂点色)を256x256で描画し、読み戻したフレームバッファを
     `render_triangle.ppm`(PPM形式、追加の画像クレート依存無し)へ保存する。
     `cargo run -p directx-graphics-vulkan --example render_triangle`の
     1コマンドで実行可能。実Vulkanデバイスが無い環境では正直にエラーを
     出して終了する(モックでの「成功したふり」はしない、既存の実機テスト
     群と同じ方針)。
  2. **背景**: このリポジトリはライブラリ集合で`fn main`を持たず、新規に
     触る人が「何か1つ動かして確認する」手段がテストのソースコードを読む
     ことしかなかった(外部監査で指摘された使いやすさのギャップ)。この
     exampleがその最初の1コマンドになる。
  3. **`README.md`に「See it actually draw something」節を追加**し、上記
     コマンドをQuickstart的に案内。`.gitignore`に生成物
     (`render_triangle.ppm`/`.png`)を追加。
  4. **検証**: 実際に`cargo run -p directx-graphics-vulkan --example
     render_triangle`を実行し、NVIDIA GeForce GT 730上で
     `render_triangle.ppm`(256x256、正しいPPMヘッダ)が生成されることを
     確認。`cargo test --workspace`は既存の全テスト(実機テスト含む)が
     回帰なくgreenのままであることを確認済み。
  - 次にすべきこと: (1) 他の主要APIにも同様のrunnable exampleを追加するか
    検討(現状はグラフィックスパイプラインのみ)、(2) PNG変換を
    ワンステップで行いたい場合の軽量画像クレート導入の要否検討(今回は
    依存を増やさない方針を優先しPPM止まりとした)。
