# 設計思想＆開発方針＆開発環境ルール(open-directx)

> **📌 保留タスク(2026-08-06、次回セッションで着手予定)/ Pending task (added 2026-08-06, to be started next session)**:
> ユーザー指示により、**東芝の疑似量子コンピューター技術(Simulated
> Bifurcation Machine)**と**DeepSeekの技術**(インターネットニュースだけ
> でなく、論文〈DeepSeek-V3/R1テクニカルレポート等〉・実装ノウハウの
> ブログまで日英両言語でGoogle/GitHub調査)を、`dream-os`/`open-directx`/
> `open-cuda`/`aruaru-llm`/`open-web-server`/`RPoem`/`open-raid-z`/
> `aruaru-db`の8リポジトリへ組み込む構想がある。東芝SBMは`dream-os`
> (`sbm_ising`カーネル、64スピンPoC)に実装済み——他リポジトリへの適用は
> 各リポジトリで「何を最適化するか」を先に特定してから着手すること
> (このリポジトリ固有の候補は未検討、次回調査対象)。DeepSeekは前回調査で
> 「数千枚のGPUを1枚に圧縮する技術」という主張は確認できなかった(誤解・
> 誇張と判断済み)——今回は論文・実装ブログまで調査範囲を広げ、実在する
> 技術(MLA・DeepSeekMoE・FP8混合精度学習等)を特定してから適用箇所を
> 検討すること。詳細は`dream-os/CLAUDE.md`の同日HANDOFF参照。
>
> By user instruction, there is a plan to incorporate **Toshiba's
> pseudo-quantum-computer technology (Simulated Bifurcation Machine)**
> and **DeepSeek's technology** (researched via Google/GitHub in both
> Japanese and English, going beyond news articles to actual papers
> like the DeepSeek-V3/R1 technical reports and implementation-notes
> blogs) into 8 repositories: `dream-os`, `open-directx`, `open-cuda`,
> `aruaru-llm`, `open-web-server`, `RPoem`, `open-raid-z`, and
> `aruaru-db`. Toshiba SBM is already implemented in `dream-os` (the
   `sbm_ising` kernel, a 64-spin PoC) — applying it elsewhere requires
> first identifying a concrete optimization problem in each repo (not
> yet investigated for this repo). The previous DeepSeek research found
> no evidence for a "compress thousands of GPUs into one" technology
> (judged to be a misunderstanding/exaggeration) — this time, broaden
> the research to papers and implementation blogs, identify real
> techniques (MLA, DeepSeekMoE, FP8 mixed-precision training, etc.),
> then decide where they apply. See the same-day HANDOFF entry in
> `dream-os/CLAUDE.md` for details.


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

- **2026-08-08(続き5) 実Linux環境(WSL2 Ubuntu)でグラフィックスパイプライン
  デモを実際にビルド・実行——「dream-os/Linux上でopen-directxが実際に
  何かを動かせることを見せる」というユーザー要望に対応**:
  1. **背景**: ユーザーからarchive.orgのSpace Harrier(Internet Arcade)を
     引き合いに出された際、これはブラウザ内WASMエミュレータ
     (EmulatorJS/RetroArchのMAMEコア、WebGL/WebGPU描画)であり実際の
     Windows DirectXバイナリとは無関係(そもそもDirectX自体を使わない
     1985年のアーケード基板向けゲーム)であることを説明し、代わりに
     このリポジトリ既存の`triangle_vs.hlsl`/`triangle_ps.hlsl`
     (DXBC→SPIR-V変換・実描画まで実装済み)を実Linux機で動かす方が
     筋が通ると提案し、ユーザーの同意を得て着手した。
  2. **環境**: このマシンに実物理Linux機は無いため、既存の運用実績
     (`open-runo`/`open-web-server`等のCLAUDE.md参照)がある**WSL2
     Ubuntu**(`wsl -d Ubuntu`、カーネル`6.18.33.2-microsoft-standard-
     WSL2`)を実Linux ABI環境として使用した——これはWindows上の仮想化
     ではあるが、実際のLinuxカーネル・Linuxのシステムコール・Linux版
     Vulkanローダー/ドライバ(Windows版とは別物)を使うため、「Linux上で
     動くか」という問いに対する正当な検証環境である。
  3. **セットアップ**: `libvulkan1`(Vulkanローダー)・
     `mesa-vulkan-drivers`(Mesaの各種Vulkan ICD)をapt経由で導入
     (`vulkan-tools`のみ、対話的sudoパスワード入力が必要で今回の
     自動化セッションからは導入できなかった——ただし`vulkaninfo`
     CLIツール自体は実行に必須ではないため、実デモの実行には支障
     無かった)。
  4. **実行結果(誇張なし、実出力そのまま)**: `/mnt/f/runo/open-directx`
     (Windows側F:ドライブのリポジトリをWSL2からマウント経由でそのまま
     参照、追加clone不要)で`cargo run -p directx-graphics-vulkan
     --example render_triangle --release`を実際に実行し、**Windows向けに
     ビルドされたバイナリを一切使わず、Linux ELFバイナリとしてゼロから
     再コンパイル**した上で成功:
     ```
     描画成功: 256x256のグラデーション三角形を render_triangle.ppm に保存しました。
     ```
     生成された`render_triangle.ppm`(196,623バイト、正しいPPM P6
     ヘッダ`256 256`、実際の赤/緑/青グラデーションのRGBピクセルデータ)
     を確認した(検証後、生成物は削除済み・`.gitignore`対象のため
     コミット対象外)。
  5. **`cargo test -p directx-graphics-vulkan --release -- --nocapture`
     も実際にLinux上で実行し、既存の実機テスト5本すべてがgreen**
     (誇張なし、実出力そのまま):
     ```
     OK: enumerate_graphics_devices reported 1 graphics-capable device(s): [GraphicsDeviceInfo { name: "llvmpipe (LLVM 21.1.8, 256 bits)", vendor_id: 65541, vendor: "Unknown" }]
     test enumerate_graphics_devices_reports_the_real_gpu_on_this_machine ... ok
     test indexed_scene_with_depth_buffer_keeps_the_nearer_triangle_on_real_vulkan_hardware ... ok
     test d3d11_triangle_pixel_position_maps_to_the_expected_ndc_coordinate_on_real_vulkan_hardware ... ok
     test d3d11_triangle_draw_call_interpolates_distinct_per_vertex_colors_on_real_vulkan_hardware ... ok
     test d3d11_triangle_draw_call_matches_passthrough_vertex_color_on_real_vulkan_hardware ... ok
     test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     ```
     深度バッファ比較・ピクセル↔NDC変換式・頂点色補間・パススルー色
     一致という、Windows実機(NVIDIA GT 730)で既に検証済みの4つの
     強いアサーションすべてが、**同一のコード・同一のDXBC由来SPIR-V
     バイト列で、Linux上でも一致する結果を出した**ことを確認した——
     コードの再実装・条件分岐無し、`#[cfg(windows)]`のような
     プラットフォーム分岐も一切書いていない(既存のコード自体が
     元々プラットフォーム非依存であることの直接証明)。
  6. **正直な開示(誇張しない)**: (1) **実際に使われたVulkanデバイスは
     `llvmpipe`(Mesaのソフトウェアラスタライザ、CPU上でVulkanを
     エミュレート)であり、実GPUハードウェアではない**——WSL2から
     ホストのGPU(このマシンのNVIDIA GT 730)へVulkan経由でアクセス
     するには追加のドライバ設定(WSLg/DirectX-to-Vulkan変換層である
     `dozen`ICD等)が必要で、今回はそこまで到達していない。
     つまり今回証明できたのは「コードがLinux ABI上で正しく動作する
     こと」であり、「Linux上で実GPUハードウェアアクセラレーションが
     効くこと」ではない——後者は次回以降の課題として正直に残す。
     (2) `directx-shader-translate`クレート自体のCompute Shader系
     テスト(`opencuda-vulkan`への`cross-repo`パス依存を持つ)はこの
     パスではLinux上で実行していない(`open-cuda`リポジトリを
     WSL側にも配置する必要があり、今回はグラフィックス側のみに
     スコープを絞った)。(3) vulkan-tools(`vulkaninfo`)は対話的sudo
     認証が必要で導入できなかった——今後同様の検証を行う際は、
     ユーザー自身が事前に`sudo apt-get install vulkan-tools`を実行して
     おくと、より詳細なデバイス診断ができる。
  - 次にすべきこと: (1) WSL2からホストGPU(NVIDIA GT 730)への実
    ハードウェアVulkanアクセス(`dozen`ICD等)の設定・実機検証、
    (2) `directx-shader-translate`のCompute Shader系テストもLinux側で
    実行できるよう`open-cuda`リポジトリをWSL側に用意する、(3) 実際の
    物理Linux機(このマシン以外)での再現性確認、(4) 10項以上への
    境界チェック付きチェーン拡張、(5) テクスチャサンプリング・
    スワップチェーンへの拡張。

- **2026-08-08(続き4) `mul`のnegateフラグが立つケースを実装・実機検証
  (複数回のHANDOFFエントリで「未検証」と記録されていたギャップを解消)**:
  1. **新規シェーダー**: `shaders/vector_mul_negate.hlsl`
     (`Output[i] = InputA[i] * (-InputB[i])`)を`fxc.exe /T cs_5_0`で実
     コンパイル。
  2. **実SHEX命令を`examples/dump_shex`でダンプして確認**(推測実装では
     ない、既存方針の継続): `Mul`命令の第1ソースオペランド
     (`operands[1]`、Bのロード結果)に`negate: true`が実際に立つことを
     確認した——`sub`最適化(`Add`の第1オペランドへのnegate)とは別の
     独立したケースとして、`Mul`自体にもfxcが同じnegate機構を使うことを
     裏付けた。
  3. **実装**(`crates/directx-shader-translate/src/spirv_gen.rs`):
     `BinaryOp::MulNeg`(`A * (-B) = -(A*B)`)を新設。`decode_shader_shape`
     の`Opcode::Mul`分岐で両ソースオペランドのnegateを検査し、片方のみ
     negateなら`MulNeg`、両方negate(理論上は打ち消し合うはずだが実
     シェーダーで未確認)は`SpirvGenError::UnsupportedShader`で正直に
     拒否する。`emit_body`(単独シェーダー用)は`OpFMul`+`OpFNegate`で
     正しく計算するよう対応。チェーンクラス側(`emit_chain_spirv`の
     `emit_expr`)は`decode_chain_shape`が`Add`以外のnegateを既に拒否して
     いるため`MulNeg`が渡ってくることはなく、`unreachable!()`で明示。
  4. **実機検証(NVIDIA GT 730)**: 新規テスト
     `vector_mul_negate_real_vulkan.rs`で、256要素すべてがCPU参照実装
     `a[i]*-b[i]`と一致することを確認(`c[0]=-65, c[255]=-160.625`)。
     修正前のコード(`op = Some(BinaryOp::Mul)`固定)のままだったら
     negateを無視して`a[i]*b[i]`(符号違いの誤った値)を黙って返して
     いたはずであり、これは「誤ったSPIR-Vを黙って生成しない」という
     既存方針に対する実際の潜在バグだったことが今回の検証で判明した
     (今回のパスで実際に遭遇する前に発見・修正できた)。
  5. **ワークスペース全体の検証**: `cargo build --workspace --release`・
     `cargo clippy --workspace --all-targets --release -- -D warnings`
     いずれも警告0件。`cargo test --workspace --release`で全実機テスト
     (既存27本+今回1本の計28本)+unittests 50件すべてgreen、既存経路
     への回帰なし。
  6. **正直な開示**: (1) 両ソースオペランドが同時にnegateされるケース
     (理論上は`Mul`へ戻るはずだが実シェーダー未確認)は引き続き未検証・
     拒否のまま。(2) `Div`/`Sub`単体でのnegateバリエーション(例:
     `A / (-B)`)は今回のスコープ外、次回検討候補。(3) チェーン内での
     `Mul`のnegateは`decode_chain_shape`側でまだ拒否のまま(今回は単独
     `vector_mul`パターンのみ対応)。
  - 次にすべきこと: (1) チェーン内での`Mul`negate対応、(2) `Div`/`Sub`
    のnegateバリエーション、(3) 10項以上への境界チェック付きチェーン
    拡張、(4) テクスチャサンプリング・スワップチェーンへの拡張、
    (5) AMD/Intel・Linux/macOS実機検証、(6)
    `opencuda-vulkan::VulkanDevice::launch_kernel`のカーネル名
    ハードコード解消(open-cuda側変更要)、(7) 冒頭の東芝SBM/DeepSeek
    技術組み込み構想。

- **2026-08-08(続き3) 境界チェック付きチェーンを9項へ拡張、実機検証
  (DXBC/DXIL両方、コード変更は0行)。直前エントリの「次にすべきこと(1):
  9項以上への境界チェック付きチェーン拡張」を解消**:
  1. **新規シェーダー2本**: `shaders/vector_add_mul_div_sub_add_mul_div_
     sub_add_chain9_bounded.hlsl`(既存の8項〈add->mul->div->sub->add->
     mul->div->sub〉へaddをもう1回追加、`fxc.exe /T cs_5_0`で実
     コンパイル)・DXIL版(`dxc.exe -T cs_6_0`)。`tools/compile-dxbc-
     shaders.ps1`に両方追記済み。
  2. **既存インフラの再利用(新規デコーダロジックは0行)**: DXBC側
     `decode_chain_shape`・DXIL側`resolve_dxil_calls_and_chain`のいずれも
     無改修で9項に対応できることを実機テストで確認した——2項→3項→…→
     8項→9項と続く一貫したパターン。
  3. **実機検証(NVIDIA GT 730)**: 新規テスト2本(DXBC/DXIL)で、有効
     範囲256要素すべてがCPU参照実装と一致し(`c[0]=81.00015,
     c[255]=79.79022`)、境界外64要素はセンチネル値`-1`のまま、
     DXBC/DXIL両経路の値が完全一致することを確認した。
     `kernel.read_uav_bind_points.len() == 10`(N+1規則)も実測確認。
  4. **ワークスペース全体の検証**: `cargo build --workspace --release`・
     `cargo clippy --workspace --all-targets --release -- -D warnings`
     いずれも警告0件。`cargo test --workspace --release`で全実機テスト
     (既存25本+今回2本の計27本)+unittests 50件すべてgreen、既存経路
     への回帰なし。
  5. **正直な開示**: 10項以上への拡張は未着手。その他の未着手項目
     (`mul`のnegateケース、テクスチャ/スワップチェーン、AMD/Intel/
     Linux/macOS実機検証、`opencuda-vulkan`カーネル名ハードコード解消)
     は前回エントリから変更なし。
  - 次にすべきこと: (1) 10項以上への拡張、(2) `mul`のnegateフラグ
    ケースの検証、(3) テクスチャサンプリング・スワップチェーンへの
    拡張、(4) AMD/Intel・Linux/macOS実機検証、(5)
    `opencuda-vulkan::VulkanDevice::launch_kernel`のカーネル名
    ハードコード解消(open-cuda側変更要、ユーザー確認の上で)、
    (6) 冒頭の東芝SBM/DeepSeek技術組み込み構想。

- **2026-08-08(続き2) 境界チェック付きチェーンを8項へ拡張、実機検証
  (DXBC/DXIL両方、コード変更は0行——既存の`decode_chain_shape`/
  `resolve_dxil_calls_and_chain`一般化ロジックがそのまま通用することを
  再確認)。直前エントリの「次にすべきこと(1): 8項以上への境界チェック
  付きチェーン拡張」を解消**:
  1. **事前確認**: `git status`クリーン、直前の境界チェック付き7項DXIL
     対応がmainへ確定済みであることを確認してから着手。
  2. **新規シェーダー2本**: `shaders/vector_add_mul_div_sub_add_mul_div_
     sub_chain8_bounded.hlsl`(`if (i < ElementCount) { t1=A[i]+B[i];
     t2=t1*A[i]; t3=t2/B[i]; t4=t3-A[i]; t5=t4+B[i]; t6=t5*A[i];
     t7=t6/B[i]; Out[i]=t7-A[i]; }`、既存の境界チェック付き7項〈add->mul->
     div->sub->add->mul->div〉へsubをもう1回追加、`fxc.exe /T cs_5_0`で
     実コンパイル)・`shaders/vector_add_mul_div_sub_add_mul_div_sub_
     chain8_bounded_dxil.hlsl`(同一契約、`dxc.exe -T cs_6_0`で実
     コンパイル)。`tools/compile-dxbc-shaders.ps1`に両方追記済み。
  3. **既存インフラの再利用(新規デコーダロジックは0行)**: DXBC側
     `decode_chain_shape`(`spirv_gen.rs`)・DXIL側`resolve_dxil_calls_and_
     chain`(`dxil.rs`)のいずれも、命令列を1つずつ走査してreg_map/式木を
     更新する既存の一般ロジックのまま、境界チェック+8項の組み合わせを
     無改修で正しく処理できることを実機テストで確認した——2026-08-06〜
     2026-08-08の一連のエントリ(2項→3項→4項→5項→6項→7項)と同じ
     パターンが8項でも成立した。
  4. **実機検証(NVIDIA GT 730)**: 新規テスト
     `vector_add_mul_div_sub_add_mul_div_sub_chain8_bounded_real_vulkan.rs`
     (DXBC)・`vector_add_mul_div_sub_add_mul_div_sub_chain8_bounded_dxil_
     real_vulkan.rs`(DXIL)で、`cargo test -p directx-shader-translate
     --release --test <name> -- --nocapture`を実行し、有効範囲256要素
     すべてがCPU参照実装`(((((((a[i]+b[i])*a[i])/b[i]-a[i]+b[i])*a[i])/
     b[i])-a[i]))`と一致し(`c[0]=0.00015234947, c[255]=62.540222`)、
     境界外64要素はセンチネル値`-1`のまま(書き込まれなかった)ことを
     確認した。DXBC/DXIL両経路の`c[0]`/`c[255]`が完全に一致している
     (既存のチェーン系エントリと同じ、独立2経路の追加的な裏付け
     パターン)。`kernel.read_uav_bind_points.len() == 9`(N+1規則、
     7項チェーンの8から8項では9)・`kernel.bounds_check == true`・
     `kernel.local_size == (64,1,1)`も実測確認した。
  5. **ワークスペース全体の検証**: `cargo build --workspace --release`・
     `cargo clippy --workspace --all-targets --release -- -D warnings`
     いずれも警告0件。`cargo test --workspace --release`で全実機テスト
     (既存23本+今回2本の計25本)+unittests 50件すべてgreen、既存経路
     への回帰なし。
  6. **正直な開示・まだやっていないこと**: (1) 9項以上への拡張は
     未着手。(2) `mul`のnegateフラグが立つケース、境界チェック無し版
     での今回未検証の順序組み合わせは引き続き未検証(前回エントリから
     継続)。(3) `opencuda-vulkan::VulkanDevice::launch_kernel`のカーネル名
     ハードコード自体の解消は未着手(open-cuda側の変更が必要、ユーザー
     確認の上で着手する条件付きのため今回も見送り)。(4) テクスチャ
     サンプリング・スワップチェーン・AMD/Intel/Linux/macOS実機検証・
     各クレートのexample充足状況棚卸しは前回エントリから変更なし
     (未着手のまま)。(5) dream-os側(SBM Isingカーネル・
     flash-attentionブリッジ・RAID6/Z2ブリッジ・Android実機デモ)は
     このパスでは触れていない(担当スコープの別リポジトリ)。
  - 次にすべきこと: (1) 9項以上への境界チェック付きチェーン拡張、
    (2) `mul`のnegateフラグが立つケースの検証、(3) テクスチャ
    サンプリング・スワップチェーンへの拡張、(4) AMD/Intel・
    Linux/macOS実機検証、(5) `opencuda-vulkan::VulkanDevice::
    launch_kernel`のカーネル名ハードコード自体の解消(open-cuda側の
    変更が必要、ユーザー確認の上で着手すること)、(6) 冒頭の東芝
    SBM/DeepSeek技術組み込み構想。

- **2026-08-08(続き) 境界チェック付き7項チェーンのDXIL側を実装、
  DXBC/DXIL非対称を解消(rs-sync横断セッション、dream-os/open-directxを
  対象範囲としたインクリメント作業の一環)**: 直前の同日HANDOFFエントリ
  「正直な開示・まだやっていないこと」で明記されていた「DXIL側の同種
  テストは今回は追加していない」というギャップを埋めた。
  1. **新規シェーダー**: `shaders/vector_add_mul_div_sub_add_mul_div_
     chain7_bounded_dxil.hlsl`(既存の`vector_add_mul_div_sub_add_mul_
     chain6_bounded_dxil.hlsl`と同一の分離パターン、DXBC版
     `vector_add_mul_div_sub_add_mul_div_chain7_bounded.hlsl`と同一契約・
     同一演算内容)を実際に`dxc.exe -T cs_6_0 -E main`
     (`C:\VulkanSDK\1.4.350.0\Bin\dxc.exe`)でコンパイルし、実DXILバイト列
     (`vector_add_mul_div_sub_add_mul_div_chain7_bounded_dxil.dxil`、
     3536バイト)を得た。`tools/compile-dxbc-shaders.ps1`に追記済み。
  2. **既存インフラの再利用(新規デコーダロジックは0行)**: 既存の
     `resolve_dxil_calls_and_chain`/`translate_dxil_chain_to_spirv`
     (2026-08-05〜06付エントリで境界チェック付きチェーンをN項まで扱える
     よう既に一般化済み)がこの7項シェーダーにも無改修でそのまま適用
     できることを実機テストで確認した——DXBC側チェーンデコーダが3〜6項へ
     拡張されるたびに「コード変更は0行」だったのと同じパターンが、DXIL側
     でもこの増分で成立した。
  3. **実機検証(NVIDIA GT 730)**: 新規テスト
     `vector_add_mul_div_sub_add_mul_div_chain7_bounded_dxil_real_
     vulkan.rs`で、`cargo test -p directx-shader-translate --release
     --test vector_add_mul_div_sub_add_mul_div_chain7_bounded_dxil_real_
     vulkan -- --nocapture`を実行し、有効範囲256要素すべてがCPU参照実装
     `(((((a[i]+b[i])*a[i])/b[i]-a[i]+b[i])*a[i])/b[i])`と一致し
     (`c[0]=1.0001523, c[255]=89.04022`)、境界外64要素はセンチネル値
     `-1`のまま(`c[319]=-1`)であることを実際に確認した。
     `kernel.bounds_check == true`・`kernel.local_size == (64,1,1)`も
     実測確認した。
  4. **ワークスペース全体の検証**: `cargo build --workspace --release`・
     `cargo clippy --workspace --all-targets --release -- -D warnings`
     いずれも警告0件。`cargo test --workspace --release`で単体テスト
     50件+実機テスト22本(既存21本+今回の1本)すべてgreen、既存経路への
     回帰なし。
  5. **正直な開示・まだやっていないこと**: (1) 8項以上への拡張は未着手。
     (2) `mul`のnegateフラグが立つケース、境界チェック無し版での今回
     未検証の順序組み合わせは引き続き未検証(前回エントリから継続)。
     (3) `opencuda-vulkan::VulkanDevice::launch_kernel`のカーネル名
     ハードコード自体の解消は未着手(open-cuda側の変更が必要、ユーザー
     確認の上で着手する条件付きのため今回も見送り)。(4) テクスチャ
     サンプリング・スワップチェーン・AMD/Intel/Linux/macOS実機検証・
     各クレートのexample充足状況棚卸しは前回エントリから変更なし
     (未着手のまま)。(5) dream-os側(SBM Isingカーネル・
     flash-attentionブリッジ・RAID6/Z2ブリッジ・Android実機デモ)は
     このパスでは調査したが変更していない(担当スコープの別リポジトリ、
     このセッションでは並行して別増分を検討中)。
  - 次にすべきこと: (1) 8項以上への境界チェック付きチェーン拡張、
    (2) `mul`のnegateフラグが立つケースの検証、(3) テクスチャ
    サンプリング・スワップチェーンへの拡張、(4) AMD/Intel・
    Linux/macOS実機検証、(5) `opencuda-vulkan::VulkanDevice::
    launch_kernel`のカーネル名ハードコード自体の解消(open-cuda側の
    変更が必要、ユーザー確認の上で着手すること)、(6) 冒頭の東芝
    SBM/DeepSeek技術組み込み構想。

- **2026-08-08 境界チェック付きチェーンを7項へ拡張、実機検証(rs-sync横断
  セッション、4リポジトリの関連性・実用性・完成度向上の一環)**: 直前の
  2026-08-07 HANDOFFで「6項以上は未検証」と明記されていたギャップを
  埋めた。既存の6項(`vector_add_mul_div_sub_add_mul_chain6_bounded`)に
  さらに`div`を1個追加し、`vector_add_mul_div_sub_add_mul_div_chain7_
  bounded.hlsl`(新規)を`fxc.exe`(`cs_5_0`、Windows Kit 10.0.22621.0)で
  実コンパイルしてDXBCを生成、`translate_chain_shader`で自前SPIR-V生成
  経路へ通した。
  1. **実機検証(NVIDIA GT 730)**: 新規テスト
     `vector_add_mul_div_sub_add_mul_div_chain7_bounded_real_vulkan.rs`で、
     有効範囲256要素すべてがCPU参照実装
     `((((a[i]+b[i])*a[i])/b[i]-a[i]+b[i])*a[i])/b[i]`と一致し、境界外64
     要素はセンチネル値のまま(書き込まれなかった)ことを確認
     (`cargo test -p directx-shader-translate --release --test
     vector_add_mul_div_sub_add_mul_div_chain7_bounded_real_vulkan`)。
     `read_uav_bind_points.len() == 8`(N+1規則、7項チェーンでも成立)も
     実測確認した。DXIL側の同種テストは今回は追加していない(DXBC側の
     ギャップ埋めを優先、正直な開示)。
  2. **ワークスペース全体の検証**: `cargo build --workspace --release`・
     `cargo clippy --workspace --all-targets --release -- -D warnings`
     いずれも警告0件。`cargo test --workspace --release`は既存の全実機
     テスト(グラフィックス5本+DXBC/DXIL Compute単一演算+チェーン群
     〈2〜6項、境界チェック有無、DXBC/DXIL双方〉+今回の7項境界チェック
     付きチェーン1本)すべてgreenであることを確認(既存経路への回帰なし)。
  3. **今回あわせて修正した既存の粗**: `cargo clippy`実行時、pre-existing
     の2件を検出・修正(今回のチェーン拡張とは無関係、clippyのlintルール
     追加によるもの): (a) `directx_bridge.rs`の`n *
     std::mem::size_of::<f32>()`を`std::mem::size_of_val(a)`へ
     (`manual_slice_size_calculation`)——ただしこれは`dream-os`
     リポジトリ側の`crates/dream-os-kernel/src/directx_bridge.rs`
     (open-directxをpath依存で再利用しているファイル)の修正であり、
     本リポジトリ自体のファイルではない。(b) 本リポジトリでは該当なし
     ——念のためこのリポジトリ自体にも同種のclippy警告が無いことを
     `cargo clippy --workspace --all-targets --release -- -D warnings`
     で再確認済み(警告0件)。
  4. **正直な開示・まだやっていないこと**: 検証できたのは「境界チェック
     付き7項」のDXBC側1点のみ。DXIL側の7項・8項以上・`mul`のnegateフラグが
     立つケース・境界チェック無し版の7項以上は引き続き未検証。
     open-cuda・aruaru-llmとの連携状況は前回エントリから変化なし
     (このパスではopen-cuda/aruaru-llm側のファイルには一切触れていない)。
     テクスチャサンプリング・スワップチェーン・AMD/Intel/Linux/macOS
     実機検証・各クレートのexample充足状況棚卸しも前回エントリから
     変更なし(未着手のまま)。
  - 次にすべきこと: (1) DXIL側の7項境界チェック付きチェーン(DXBC側と
    対にする、既存の他チェーン系はDXBC/DXIL両方揃っているためこの7項
    だけ片方のみという非対称を解消する)、(2) 8項以上への拡張、または
    今回未検証の順序組み合わせ、(3) テクスチャサンプリング・
    スワップチェーンへの拡張、(4) AMD/Intel・Linux/macOS実機検証、
    (5) `opencuda-vulkan::VulkanDevice::launch_kernel`のカーネル名
    ハードコード自体の解消(open-cuda側の変更が必要、ユーザー確認の上で
    着手すること)、(6) 冒頭の東芝SBM/DeepSeek技術組み込み構想。

- **2026-08-07 dream-os/open-cuda/aruaru-llmとの関連性・連携性調査
  (ユーザー指示「4リポジトリの関連性・連携性・実用性・完成度を向上」、
  コード変更は無し、正直な開示)**: 4リポジトリを横断してCLAUDE.md・
  ソースツリーを再確認した。`nvidia-smi`でこのマシンのGPUが依然NVIDIA
  GeForce GT 730の1台のみであることを確認。このリポジトリ自身の
  「次にすべきこと」一覧(直前2026-08-06 HANDOFF参照)のうち、
  (1)6項以上の境界チェック付きチェーン・順序組み合わせの拡張、
  (2)テクスチャサンプリング・スワップチェーン対応、(3)AMD/Intel・
  Linux/macOS実機検証(実機が無く不可能)、(5)
  `opencuda-vulkan::VulkanDevice::launch_kernel`のカーネル名ハード
  コード解消(open-cuda側の変更が必要、ユーザー確認の上で着手する
  条件付きのため今回は見送り)、(6)東芝SBM/DeepSeek技術組み込み構想
  (dream-os側の技術詳細調査が前提)——のいずれも、限られた時間内で
  「型チェックのみで完了と報告しない」実機検証を伴う形で安全に完了
  させられる規模とは判断できなかった(特に(1)(2)は既存のDXBC/DXIL
  チェーン生成ロジックへの理解無しに変更すると、数値的に誤った
  カーネルを「動いた」と誤報告するリスクがあると判断)。一方、
  `open-cuda`側では今回`open-cuda-llm`のAttention経路への
  `flash_attention_with_spirv`配線(実機検証済み、詳細は`open-cuda/
  CLAUDE.md` 2026-08-07(続き5)HANDOFF参照)を実施した——これは
  `open-directx`側のファイルには一切触れていない、独立した増分。
  `cargo build --workspace`で既存の健全性(警告0件)のみ再確認した。
  - 次にすべきこと: 前回HANDOFFの(1)〜(6)から変更なし。特に(6)は
    `dream-os`側で`sbm_ising`の適用対象がこのリポジトリのどの計算
    (境界チェック付きチェーンの最適配置探索等、組合せ最適化として
    定式化できる部分)にあたるかを先に特定してから着手すべき、という
    判断は今回も変わらない。

- **2026-08-06 カーネルレベルアンチチートによる制約を正直に開示(ユーザー
  指摘、Wine/Proton/Lutris情報共有への返答)**: ユーザーから、Linux上で
  Windows 3Dオンラインゲームを動かす一般的な解決策(Steam Proton・
  Lutris・Wine)の情報共有があり、「多くの3Dオンラインゲームに導入
  されている一部のカーネルレベルのアンチチートシステム(例: Vanguard等)
  はLinux環境やProtonを非対応としている場合があり、起動できないことが
  ある」という重要な指摘を受けた。
  1. **open-directxとの関係の整理**: open-directx自体は既に
     Wine/Proton/vkd3d-protonと同じアプローチ(D3D API呼び出しの
     インターセプト+DXBC/DXILシェーダーの実行時翻訳→Vulkan実行)を
     2026-07-25の方針転換時に採用済み(この節冒頭「技術的な位置づけの
     訂正」参照)。つまりWine/Proton/Lutrisは競合ではなく、open-directxが
     参考にしている**既に実証済みの先行事例**という位置づけ。
  2. **正直なリスク開示(新規、今回追記)**: カーネルレベルの
     アンチチート(Riot Vanguard・BattlEye〈カーネルモード〉・
     Easy Anti-Cheat〈一部タイトル〉等)は、**技術的な変換能力とは
     無関係にゲーム開発元が意図的にLinux/Proton環境を検知・拒否する**
     設計になっている場合がある。これはopen-directxがDXBC/DXIL翻訳の
     完成度をどれだけ高めても、**回避できない・回避すべきでない制約**
     である(アンチチートの意図的な迂回は利用規約違反・不正行為に
     あたりうるため、本プロジェクトのスコープとして目指すべきでもない)。
  3. **結論**: open-directxが対象とすべき現実的なゲームの範囲は、
     「カーネルレベルアンチチートを持たない、またはLinux/Proton
     互換性を公式サポートしているタイトル」に限定される——この制約は
     Wine/Proton自体も同じく抱えている、業界共通の既知の限界であり、
     open-directx固有の欠陥ではない。今後、実際のゲームタイトルでの
     動作確認を検討する際は、対象タイトルのアンチチート方式を事前に
     確認することを推奨する。
  - 次にすべきこと: 特に新規実装は無し(調査・リスク開示の記録のみ)。
    引き続き前回HANDOFFエントリの「次にすべきこと」(6項以上のチェーン、
    テクスチャサンプリング/スワップチェーン等)を優先する。

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

- **2026-07-27(続き5) README.mdにプラットフォーム/ベンダー対応表を追加(ユーザー指示「WindowsのDirectXをLinuxやMACやAndoridやiPohneなどへの互換性とnVidiaとAMDとINTELの互換性を高めて」への対応)**:
  1. **調査結果(コード変更なし、監査結果)**: このリポジトリのコードには
     `cfg(windows)`等のプラットフォーム限定gateが一切存在せず(DXBCパーサー・
     SPIR-Vコード生成・`directx-graphics-vulkan`はいずれもプラットフォーム
     非依存のRust+ash実装)、Windows専用コードで塞がれている箇所は無いと
     確認した。GPUベンダーID(NVIDIA `0x10DE`・AMD `0x1002`/`0x1022`・
     Intel `0x8086`)も本リポジトリ・`opencuda-vulkan`・`opencuda-directx`の
     3箇所で完全に一致しており、不整合・誤りは無い。
  2. **README.mdに「Platform & vendor support matrix」節を新規追加**:
     Windows(実機NVIDIA GT 730で検証済み)・Linux(コード上はブロック
     要因なしだが実機未検証)・Android(open-cudaの`aarch64-linux-android`
     クロスコンパイル成功済みだが実機未検証)・macOS/iOS(MoltenVK経由、
     いずれも未着手——iOSはVulkanネイティブ非対応でMoltenVK経由である
     ことを明記、過大な主張をしない)を一覧化。ベンダー側もNVIDIA
     (検証済み)・AMD/Intel(コードは正しいが実機未検証)を明記。
  3. **正直な開示・今回はコード変更をしていない理由**: (1) このマシンには
     AMD/Intel GPU・Linux/Mac環境・実Android/iOS端末が無く、実機検証を
     伴わない「対応済み」表記は誇張になるため、ドキュメントでの現状整理に
     留めた。(2) ベンダーID自体は既に正しいため、修正すべきバグは無かった。
  - 次にすべきこと: (1) 実際にLinux環境でのビルド確認、(2) 実AMD/Intel
    GPU入手時の実機vendor_name_from_id検証、(3) 実Android端末での
    `vkCreateInstance`実行確認、(4) macOS実機でのMoltenVK経由ビルド確認。
  - **PlayStation/Dolby Visionについて(正直な開示、ユーザーへの再確認が必要)**:
    ユーザーから追加で「SONYのプレイステーション5/5PRO/6の4K120FPSや
    Dolby Vision 2 Ultraなどのハードウェアアクセラレータ」への対応も
    要望されたが、本CLAUDE.mdの2026-07-25付「PlayStation 4/5/6/7対応に
    ついて」節が既に、PlayStation SDKの非公開・NDA性質による法務リスク
    (非公式リバースエンジニアリングがDMCA等に抵触しうる)を理由に
    「将来的な野心としてロードマップに明記するに留め、現時点では設計・
    実装の対象に含めない」と明確に判断済み。Dolby Visionも同様に
    ライセンス・特許で保護された認証技術であり、無許諾での実装は
    同種の法的リスクを伴う。この既存の判断を変更する場合は、法的リスク
    評価を別途行った上でユーザーに再確認してから着手する方針を維持する
    (このパスでは実装に着手していない)。

- **2026-07-27(続き6) PlayStation 5 Pro/PS6の4K120FPS・Dolby Vision 2 Ultraについて、公開情報のみの調査を実施(ユーザーから「調査のみ許可」の明示的な回答を得た上で実施——設計・実装・SDK解析は行っていない)**:
  1. **調査結果(公開のマーケティング情報・技術記事のみ、非公開SDK・NDA資料には一切アクセスしていない)**:
     - **PS5 Pro**: PSSR(PlayStation Spectral Super Resolution、NVIDIA
       DLSS/AMD FSR相当のML方式アップスケーラー)を用いて、対応ゲームで
       レイトレーシング+AI超解像による4K相当・最大120fpsを実現。PSSR自体は
       非公開のML実装で、公開情報からは「何をしているか」の概要以上は
       分からない。
     - **PS6(未発売、噂ベース)**: AMD RDNA 5世代GPU(コードネーム
       "Orion"、52 CU/54 CU構成、最大3GHz)を搭載し、レイトレーシング性能は
       PS5比6〜12倍、ラスタライズ性能は2.5〜3倍という観測筋の推測がある。
       Performanceモードで4K/120fps、Qualityモードで4K/60fpsを狙うとされる
       (いずれも未確定の推測情報、Sony公式発表ではない)。
     - **Dolby Vision 2 / Dolby Vision 2 Ultra**: 2025年9月発表、2026年
       CESで本格公開されたHDR規格。新しい「Content Intelligence」画像
       エンジンにより、シーン単位でTVの実際の表示能力に合わせて輝度・
       色・コントラストを動的最適化する。上位ティア「Dolby Vision 2
       Max」も存在(ユーザーが言及した「Ultra」は現時点の公開資料では
       「Max」表記のみ確認、名称の対応関係は未確認)。ライセンス・
       特許で保護された認証技術であり、実装にはDolby社とのライセンス
       契約が必要。
  2. **結論(変更なし)**: いずれも(a) 非公開のML実装(PSSR)や非公開の
     コンソールSDK、(b) ライセンス・特許保護技術(Dolby Vision 2)を
     基盤としており、公開情報だけから実装可能な範囲を超える。本
     リポジトリの既存方針(2026-07-25付「PlayStation対応について」)を
     維持し、**設計・実装には着手しない**——「将来的な野心」として
     ロードマップに残すのみ。ユーザーには、この調査結果と既存方針の
     維持を報告済み。
  - 次にすべきこと: 特になし(この件は現状維持で完結。ユーザーから
    改めてライセンス取得・法的リスク評価を行った上での着手指示が
    あった場合のみ再検討する)。

- **2026-07-30 深度バッファ+複数三角形(インデックス描画)を実装——
  複数エントリにわたって「次にすべきこと(3)」に残っていた項目を解消
  (ユーザー指示「sync_integration.rsを完成させてその次にGPU系2リポジトリの
  本格開発を次に進めて」)**:
  1. **新規`render_indexed_scene_with_depth_and_read_back()`**
     (`crates/directx-graphics-vulkan/src/lib.rs`): D32_SFLOAT深度
     アタッチメント(depth_test_enable/depth_write_enable/
     CompareOp::LESS、D3D11既定の「小さい値=手前」規約)+任意個数の
     頂点/インデックスリスト+`vkCmdDrawIndexed`を実装。既存の
     `render_triangle_and_read_back`(3頂点固定・深度無し、実機テスト
     済みで壊したくない)はこのマシンにGPUが1台(NVIDIA GT 730)しか無く
     クロスチェックできないため、意図的にリファクタせず独立関数として
     追加(重複はあるが、doc commentに判断根拠を明記)。
  2. **実機テスト
     `indexed_scene_with_depth_buffer_keeps_the_nearer_triangle_on_real_vulkan_hardware`**:
     近い赤三角形(z=0.1)を先に描画し、同じ範囲を覆う遠い青三角形
     (z=0.9)を後から描画する、あえて「深度テストが無ければ後から
     描画した方が勝つ」(ペインターズアルゴリズムのバグ)はずの順序で
     構成。実際にNVIDIA GeForce GT 730上で赤64/64・青0/64を確認——
     描画順序に反して近い方が正しく勝つことを実証(単に「何か色が
     出た」ではなく、深度比較ロジックそのものを検証する強いアサーション)。
  3. **検証**: `cargo test --workspace`(unittests 37件+実機テスト9本、
     今回の新規1本を含む)全green、既存テストへの回帰なし。
  4. **正直な開示・まだ未着手**: (1) ピクセル位置↔NDC座標の厳密な
     変換式の検証(前々回エントリから継続、今回も不変量ベースの検証に
     留めた)、(2) テクスチャ・スワップチェーン・実ウィンドウ表示、
     (3) DXBC Computeチェーンクラスのsub/div対応(別エントリで既に
     対応済みのはずだが、このパスでは再確認していない)。
  - 次にすべきこと: (1) ピクセル位置↔NDC座標の厳密な変換式の検証、
    (2) テクスチャサンプリング・スワップチェーンへの拡張、
    (3) このマシンにGPUが1台しか無いため未検証のAMD/Intel実機・
    Linux/macOS実機での動作確認(前回エントリから継続)。

- **2026-08-04 ピクセル位置↔NDC座標の厳密な変換式を実機で検証するテストを
  追加——複数エントリにわたって「次にすべきこと」に残っていた項目を解消
  (ユーザー指示「open-directxの完成度と実用性の全体の向上を進めて」、
  open-cuda/aruaru-llm連携強化作業の前提としての単体作業)。作業開始前に
  まず本セクション末尾を確認し、事前に判明していた4項目
  (DXBC Computeチェーンのsub/div対応、DXIL側の追従、ピクセル↔NDC変換式検証、
  D3D11グラフィックスの深度/複数三角形/インデックス/テクスチャ/スワップ
  チェーン)のうち、実装状況を1件ずつコード側で再確認した:**
  1. **事前確認で判明した「正直な訂正」**: 提示された未着手リストのうち
     (1)(DXBC Computeチェーンのsub/div対応)と(4)の一部(深度バッファ・
     複数三角形・インデックスバッファ)は、`crates/directx-shader-translate/
     src/spirv_gen.rs`の`decode_chain_shape`と
     `crates/directx-graphics-vulkan/src/lib.rs`の
     `render_indexed_scene_with_depth_and_read_back`を実際に読んだところ、
     既にそれぞれ2026-07-27・2026-07-30付のエントリで実装・実機検証済み
     だった(このCLAUDE.mdの本セクションを最後まで読めば分かる内容で、
     タスク側の「未着手リスト」の把握が古かった)。誇張を避けるため、
     今回はこの事実確認の上で、実際に手つかずだった(3)
     (ピクセル位置↔NDC座標の厳密な変換式の検証)に着手した。
  2. **`crates/directx-graphics-vulkan/tests/triangle_real_vulkan.rs`に
     新規実機テスト
     `d3d11_triangle_pixel_position_maps_to_the_expected_ndc_coordinate_on_real_vulkan_hardware`
     を追加**: `render_gradient_triangle_and_read_back`が使う「大三角形」
     (NDC頂点`(-1,-1)`/`(3,-1)`/`(-1,3)`)と、`src/lib.rs`内の実際の
     `vk::Viewport`構築(`x:0,y:0,width,height,min_depth:0,max_depth:1`、
     Yフリップ無し)をソースから直接確認した上で、ピクセル中心
     `(col+0.5, row+0.5)`→NDCの変換式`ndc_x=(col+0.5)/width*2-1`,
     `ndc_y=(row+0.5)/height*2-1`と、この三角形固有の重心座標の閉形式
     `l1=(ndc_x+1)/4`, `l2=(ndc_y+1)/4`, `l0=1-l1-l2`を導出。対称性で
     偶然一致しうる中心・対角ピクセルを避け、あえて非対称な
     `(col=2, row=5)`(8x8フレームバッファ)を選び、この閉形式から求めた
     期待色(R8G8B8A8_UNORMの`round(x*255)`量子化込み)と実際の読み戻し
     ピクセルを比較する、変換式そのものを直接検証するテストとした
     (既存の不変量ベースのテストとは独立に追加、既存テストは無変更)。
  3. **実際に`cargo test -p directx-graphics-vulkan --test
     triangle_real_vulkan -- --nocapture`で確認した結果(誇張なし、
     実出力そのまま、NVIDIA GeForce GT 730)**:
     ```
     OK: pixel (col=2, row=5) on an 8x8 framebuffer maps to NDC (-0.3750,0.3750) as derived, and the real read-back color Rgba8 { r: 127, g: 40, b: 88, a: 255 } matches the closed-form expected color (128,40,88) within tolerance 2 on the real GPU present on this machine.
     test d3d11_triangle_pixel_position_maps_to_the_expected_ndc_coordinate_on_real_vulkan_hardware ... ok
     ```
     計算誤差はr成分で1(UNORM丸めの範囲内、許容差2に収まる)のみで、
     導出した変換式が実際のVulkanパイプラインの挙動と一致することを
     実機で確認した。
  4. **ワークスペース全体の検証**: `cargo test --workspace`で全テスト
     (unittests+実機テスト、グラフィックス5本(新規1本含む)+DXBC Compute
     系+DXBCチェーン(add/mul/sub/div)+DXIL Compute系、全て)green、
     既存テストへの回帰なし。
  5. **正直な開示・今回やらなかったこと**: (1) DXIL側
     (`resolve_vector_add_dxil_calls`/`translate_dxil_vector_add_to_spirv`)
     がDXBC側のチェーンクラス一般化に追従できていない件は今回時間の
     都合で手を付けていない(`dxil.rs`に`translate_dxil_binary_op_to_spirv`
     という既存の一般化関数はあるが、DXBC側の`decode_chain_shape`相当の
     チェーン検出ロジックがDXIL側に存在するかは未確認のまま)。
     (2) テクスチャサンプリング・スワップチェーン・実ウィンドウ表示は
     引き続き未着手。(3) AMD/Intel実機・Linux/macOS実機での検証は
     このマシンにNVIDIA GT 730 1台しか無いため今回も未実施。
     (4) README/CLAUDE.mdのfeature flag説明の整理、公開APIのexample
     不足確認は時間の都合で今回は着手していない(`directx-graphics-vulkan`
     には既に`examples/render_triangle.rs`があるが、他クレートの
     example充足状況は未調査)。
  - 次にすべきこと: (1) DXIL側のチェーンクラス一般化への追従
    (`dxil.rs`の`translate_dxil_binary_op_to_spirv`が既存の一般化
    ロジックとどこまで重複/乖離しているかの調査から)、(2) テクスチャ
    サンプリング・スワップチェーンへの拡張、(3) AMD/Intel・Linux/macOS
    実機検証(前回・前々回エントリから継続)、(4) 各クレートの公開API
    example充足状況の棚卸し。

- **2026-08-05 上記(1)の事前調査のみ実施(コード変更なし、ユーザー
  指示「open-directx open-cuda aruaru-llmの連携・実用性・完成度を
  向上」、優先順位1位として着手したが時間の都合で調査止まり)**:
  1. **確認できたこと**: DXBC側(`spirv_gen.rs::decode_chain_shape`/
     `ChainShape`)は`RegExpr`という式木で複数演算のチェーン
     (`vector_add_mul_chain.hlsl`・`vector_sub_div_chain.hlsl`、
     いずれも実fxc.exe出力の`.dxbc`が既に存在)を扱えるのに対し、
     **DXIL側(`dxil.rs::ResolvedDxilBinOp`/
     `translate_dxil_binary_op_to_spirv`)は単一の二項演算
     (`lhs_range_id`/`rhs_range_id`の1組)しか扱えない**ことをソースを
     直接読んで確認した——`dxil.rs`に`chain`という語自体が1件も
     出現しないことも`grep`で確認済み。前回HANDOFFの想定通り、
     本当にDXIL側だけが取り残されているギャップだった。
  2. **`vector_add_mul_chain.hlsl`/`vector_sub_div_chain.hlsl`の
     DXIL(`.dxil`)版は未コンパイルのまま**(このパスでは`dxc.exe`の
     パス〈`C:\VulkanSDK\1.4.350.0\Bin\dxc.exe`〉を特定したところで
     時間切れとなり、実際のコンパイル・bitcode構造のダンプ・
     `FUNCTION_BLOCK`内の複数命令レコードの解釈には未着手)。
  3. **正直な見積もり**: これは単純な配線漏れではなく、DXIL
     (LLVM bitcode)側で複数命令から成る式木を`FUNCTION_BLOCK`の
     生レコード列から再構築する新規の解析ロジックが必要——DXBC側の
     `decode_chain_shape`実装時と同程度の探索的な作業(実際のbitcode
     出力を都度ダンプしながら1レコードずつ意味を確認する)が見込まれる
     ため、今回は無理に着手せず正直に次回へ持ち越した。
  - 次にすべきこと(変更なし、上記(1)は引き続き未着手のまま):
    (1) `dxc.exe -T cs_6_0`で`vector_add_mul_chain.hlsl`/
    `vector_sub_div_chain.hlsl`を実際にDXILへコンパイルし、
    `examples/dump_dxil.rs`で`FUNCTION_BLOCK`のレコード列を実際に
    ダンプして構造を確認するところから着手する、(2) テクスチャ
    サンプリング・スワップチェーンへの拡張、(3) AMD/Intel・Linux/macOS
    実機検証、(4) 各クレートの公開APIexample充足状況の棚卸し。

- **2026-08-05(続き) DXIL側にチェーン(N個の逐次2項演算)対応を実装——
  前回HANDOFFで正直に「次回へ持ち越し」としていたギャップ(DXBC側のみ
  `decode_chain_shape`/`RegExpr`でチェーンを扱え、DXIL側は単一BinOpのみ)を
  解消。実機(NVIDIA GT 730)で数値一致を確認済み**:
  1. **DXILコンパイル**: `vector_add_mul_chain_dxil.hlsl`/
     `vector_sub_div_chain_dxil.hlsl`(既存の`vector_add_mul_chain.hlsl`/
     `vector_sub_div_chain.hlsl`〈DXBC/fxc.exe版〉と同一契約・同一演算内容、
     `vector_add_dxil.hlsl`等の既存の分離パターンに合わせて別ファイル化)を
     実際に`dxc.exe -T cs_6_0 -E main`(`C:\VulkanSDK\1.4.350.0\Bin\dxc.exe`)
     でコンパイルし、`vector_add_mul_chain.dxil`/`vector_sub_div_chain.dxil`
     (実LLVM bitcode)を得た。`tools/compile-dxbc-shaders.ps1`に追記。
  2. **`examples/dump_dxil.rs`で実際にFUNCTION_BLOCKをダンプして確認した
     構造**(推測ではない、実バイト列): `Call`の個数は単一演算シェーダーと
     全く同じ7個(`CreateHandle`x3+`ThreadId`x1+`BufferLoad`x2+
     `BufferStore`x1)のままだった——**重要な発見**: HLSL側で`InputA[i]`を
     2回参照しても、DXBC側で確認済みだったのと同じ共通部分式除去(CSE)が
     LLVM側でも働き、2回目の`BufferLoad`は発行されず1回目の`ExtractValue`
     結果が2つ目の`BinOp`のオペランドとしてそのまま再利用されていた。
     `vector_add_mul_chain.dxil`の実フィールド値: `BinOp`1回目
     `fields=[1,3,0,31]`(lhs相対値=1,rhs相対値=3,opcode=0=add) ->
     `t = ExtractedBufferValue(u1) + ExtractedBufferValue(u0)`、`BinOp`2回目
     `fields=[1,4,2,31]`(lhs相対値=1=直前のBinOp1の結果、rhs相対値=4=
     1回目の`ExtractedBufferValue(u0)`、opcode=2=mul) -> `out = t * u0`——
     HLSLの`t = InputA[i]+InputB[i]; Output[i] = t*InputA[i];`と完全一致。
  3. **実装**(`crates/directx-shader-translate/src/dxil.rs`): 既存の
     `resolve_dxil_calls_and_binop`(単一の`ResolvedDxilBinOp`しか保持でき
     ない、複数BinOpに遭遇すると単純に上書きしてしまう設計)は変更せず、
     並行する新規関数`resolve_dxil_calls_and_chain`を追加した(既存の単一
     演算4シェーダー・既存テスト22件には無変更・無影響)。DXBC側の
     `RegExpr`(`spirv_gen.rs`)をそのまま再利用——`RegExpr`/
     `collect_loads`を`pub(crate)`化し、`emit_chain_spirv`本体を
     `emit_chain_spirv_for_kernel(thread_group, root, write_uav)`という
     DXBC固有の`ChainShape`型に依存しないパラメータのみを取る形へ切り出し
     (`emit_spirv_for_kernel`が`emit_spirv_impl`から切り出されたのと同じ
     パターン)、DXIL側からもそのまま呼べるようにした。`FUNCTION_BLOCK`を
     走査しながら、各`BinOp`のオペランドを「`ExtractedBufferValue`由来の
     `RegExpr::Load`」か「直前までのBinOp結果(`chain_exprs: HashMap<絶対
     値インデックス, RegExpr>`に記録済み)」のいずれかとして解決し、
     `RegExpr::BinOp`の木を組み立てる(単一演算版のような「2個目以降を
     無条件に上書き」ではなく、正しく式木として蓄積する点が核心の変更)。
     `translate_dxil_chain_to_spirv`(新設公開API)がこの式木+
     `extract_numthreads_from_metadata`(既存、決め打ちではなく実際に
     `METADATA_BLOCK`から抽出)+`emit_chain_spirv_for_kernel`を繋ぎ、DXBC側
     `translate_chain_shader`と対になるDXIL版のエントリポイントとなる。
  4. **正直な開示(このセクションのスコープ)**: 汎用N項チェーンデコーダ
     ではない。実際に確認したのは2回の逐次BinOp(add+mul、sub+div)のみ
     ——3回以上のチェーンは未検証(DXBC側`decode_chain_shape`の既存の
     限定と同じ)。各BinOpのオペランドが「直前のBinOp結果」か
     「`ExtractedBufferValue`」以外(例えば2つ前のBinOp結果を直接参照する
     等)の形状は`DxilCallResolutionError::UnexpectedShape`で正直に拒否
     する。境界チェック付きDXILチェーンは未検証。
  5. **単体テスト**(実際に`cargo test -p directx-shader-translate --lib`
     で確認、誇張なし、実出力そのまま):
     ```
     running 41 tests
     test dxil::tests::resolves_real_dxc_compiled_add_mul_chain_dxil_into_matching_regexpr_tree ... ok
     test dxil::tests::resolves_real_dxc_compiled_sub_div_chain_dxil_into_matching_regexpr_tree ... ok
     test dxil::tests::translate_dxil_chain_to_spirv_handles_add_mul_and_sub_div_chains ... ok
     test dxil::tests::translate_dxil_chain_to_spirv_also_accepts_the_pre_existing_single_op_vector_add_dxil ... ok
     (既存37件含め) test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     ```
     `translate_dxil_chain_to_spirv_also_accepts_the_pre_existing_single_op_
     vector_add_dxil`は、DXBC側`chain_translator_also_accepts_the_pre_
     existing_single_op_vector_add_shader`と同じ設計方針(チェーン版は
     排他的である必要はない、N=1の自明な場合として単一演算シェーダーも
     受理できる)をDXIL側でも裏付けた。
  6. **実機テスト2本を新規追加**(`tests/vector_add_mul_chain_dxil_real_
     vulkan.rs`・`tests/vector_sub_div_chain_dxil_real_vulkan.rs`、既存の
     DXBCチェーン実機テストと同じパターン)。実際に`cargo test --workspace
     -- --nocapture`で確認した結果(誇張なし、実出力そのまま、NVIDIA
     GeForce GT 730):
     ```
     device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
     OK: DXIL(dxc.exe実コンパイル、SM6.0、2項演算2回のチェーン)->SPIR-V(自前生成、resolve_dxil_calls_and_chainで式木を実解決)->実Vulkan経路が、CPU参照実装((a[i]+b[i])*a[i])と256要素すべてで数値一致した
     c[0]=65, c[255]=708.875
     test dxil_vector_add_mul_chain_matches_cpu_reference_on_real_vulkan_hardware ... ok

     device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
     OK: DXIL(dxc.exe実コンパイル、SM6.0、2項演算2回のチェーン sub+div)->SPIR-V(自前生成、resolve_dxil_calls_and_chainで式木を実解決)->実Vulkan経路が、CPU参照実装((a[i]-b[i])/a[i])と256要素すべてで数値一致した
     c[0]=-0.28000003, c[255]=0.99859154
     test dxil_vector_sub_div_chain_matches_cpu_reference_on_real_vulkan_hardware ... ok
     ```
     DXIL側の`c[0]`/`c[255]`はDXBC側の同名チェーンテスト
     (`dxbc_vector_add_mul_chain_matches_cpu_reference_on_real_vulkan_
     hardware`/`dxbc_vector_sub_div_chain_matches_cpu_reference_on_real_
     vulkan_hardware`)と完全に同じ値になっている——同一の入力データ生成式・
     同一の演算チェーンをDXBC/DXIL両経路で実行し、両方とも同じCPU参照実装
     に一致することを実機で確認した(DXBC/DXILの実装が独立に同じ正しい
     結果へ収束していることの追加的な裏付け)。
  7. **ワークスペース全体の検証**: `cargo test --workspace -- --nocapture`
     で全テスト(グラフィックス5本+DXBC Compute単一演算4本+DXBCチェーン
     (add/mul, sub/div)2本+DXIL Compute単一演算4本+**DXILチェーン
     (add/mul, sub/div)2本(新規)**、計17本の実機テスト+unittests多数)
     すべてgreen、既存経路への回帰なし。`cargo build --workspace`/
     `cargo clippy --workspace --all-targets`はいずれも警告0件。
  - 次にすべきこと: (1) 3項以上のDXIL/DXBC両チェーンでの実シェーダー
    検証(前々回エントリから継続)、(2) 境界チェック付きチェーン(DXBC側
    `decode_chain_shape`ですら未対応)、(3) テクスチャサンプリング・
    スワップチェーンへの拡張、(4) AMD/Intel・Linux/macOS実機検証、
    (5) 各クレートの公開APIexample充足状況の棚卸し。

- **2026-08-05(続き2) 直前エントリの次にすべきこと(1)を解消: DXBC/DXIL
  両チェーンデコーダが3個の逐次2項演算(add+mul+div)を実際に(コード変更
  無しで)扱えることを実バイト列で確認・実機検証まで到達**:
  1. **新規シェーダー2本**(既存の2項チェーン群と同じUAV3本・InputA/InputB
     多重参照パターン、演算を3回〈add→mul→div〉へ拡張): `shaders/
     vector_add_mul_div_chain3.hlsl`(`t1=InputA[i]+InputB[i]; t2=t1*InputA[i];
     Output[i]=t2/InputB[i];`、`fxc.exe /T cs_5_0`で実コンパイル)・
     `shaders/vector_add_mul_div_chain3_dxil.hlsl`(同一契約、`dxc.exe -T
     cs_6_0`で実コンパイル)。`tools/compile-dxbc-shaders.ps1`に両方追記済み。
  2. **実バイト列を確認してから着手(推測で実装しない、既存方針の継続)**:
     - **DXBC側**: `examples/dump_shex`で実SHEX命令列をダンプした結果、
       `ld_structured`x2 -> `add`(dest=temp.z) -> `mul`(dest=temp.x、
       `add`の結果とtemp.xに残っていた`InputA`ロード結果を掛けてtemp.xを
       上書き) -> `div`(dest=temp.x、さらに上書き、`InputB`ロード結果で
       割る) -> `store_structured`(temp.xを参照) -> `ret`という、既存の
       2項チェーン(`vector_add_mul_chain.dxbc`)と全く同じ「一時レジスタの
       コンポーネントを使い回すreg_map更新」パターンがそのまま3回に伸びた
       だけの形だった。
     - **DXIL側**: `examples/dump_dxil`で`FUNCTION_BLOCK`をダンプした結果、
       `Call`(code=34)は既存の単一/2項チェーンと変わらず**7個のまま**
       (同じ3バッファのため`CreateHandle`は3個から増えない)、`ExtractValue`
       (code=26)が2個(u0/u1それぞれ1回、CSEで再利用)、**`BinOp`(code=2)が
       3回連続で出現**(`fields=[1,3,0,31]`(add) -> `fields=[1,4,2,31]`
       (mul、lhs相対値1=直前のBinOp結果) -> `fields=[1,3,4,31]`(div、
       lhs相対値1=直前のBinOp結果、rhs相対値3=1回目の`ExtractedBufferValue
       (u1)`))という構造だった。
  3. **実装(コード変更は0行、確認のみ)**: DXBC側`decode_chain_shape`
     (`spirv_gen.rs`)・DXIL側`resolve_dxil_calls_and_chain`(`dxil.rs`)の
     いずれも、命令列を1つずつ走査して`reg_map`/`chain_exprs`を更新する
     という設計が最初からN個の逐次2項演算を想定した一般的なロジックに
     なっており、「2項までしか扱えない」ようなハードコードが存在しな
     かったため、**新規テストを実際に実行して初めて「3項でも無改修で動く」
     ことを確認した形**(タスク指示通りコードを書く前にまず実バイト列を
     確認したが、結果的にプロダクションコード自体への変更は不要だった)。
     `resolve_dxil_calls_and_chain`内の`resolved_calls.len() != 7`という
     チェックも、Call数がバッファ本数(3本固定)で決まり演算回数には依存
     しないため、そのまま通った。
  4. **正直な発見(既知の現象の再確認)**: DXIL側の3項チェーンテストで、
     最初`read_uav_bind_points`の期待値を`[0,1,0,1]`(add/mul/divの各
     オペランドを額面通りの順序で並べたもの)としたところ実際には
     `[1,0,0,1]`が返り、テストが失敗した——これは2026-07-26付HANDOFF
     エントリで既に確認済みの「addは可換演算のためdxc/LLVMの最適化パスが
     相対値参照の順序を並べ替える」現象がこのシェーダーでも再現した
     もので、数値的には`a+b=b+a`のため実行結果自体には影響しない(実際に
     実Vulkanで数値一致することで裏付け済み)。テストの期待値を実測順序
     `[1,0,0,1]`へ修正し、コメントで理由を明記した——**手計算・額面通りの
     期待だけでテストを書かず、必ず実行結果で裏取りする**という、この
     プロジェクトで過去にも同じ理由で修正が入った教訓を今回も踏襲した形。
  5. **単体テスト追加**(`cargo test -p directx-shader-translate --lib`、
     実際に確認、誇張なし):
     ```
     running 44 tests
     test spirv_gen::tests::translates_real_fxc_compiled_3op_chain_dxbc_to_valid_spirv ... ok
     test dxil::tests::resolves_real_dxc_compiled_3op_chain_dxil_into_matching_regexpr_tree ... ok
     test dxil::tests::translate_dxil_chain_to_spirv_handles_3op_chain ... ok
     (既存41件含め) test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     ```
     DXBC側テストは`read_uav_bind_points == [0,1,0,1]`(額面通りの順序、
     DXBCはSHEXレベルでの直接的な走査のためLLVM最適化パスの並べ替えを
     受けない)・`write_uav_bind_point == 2`・`local_size == (64,1,1)`を
     実バイト列から検証。DXIL側は式木が`Div(Mul(Add(Load,Load),Load),
     Load)`という形(`(a+b)*a/b`)であることをパターンマッチで検証。
  6. **実機テスト2本を新規追加**(`tests/vector_add_mul_div_chain3_real_
     vulkan.rs`〈DXBC〉・`tests/vector_add_mul_div_chain3_dxil_real_
     vulkan.rs`〈DXIL〉、既存の2項チェーン実機テストと同じパターン、
     ゼロ除算を避けるためbは常に正の非ゼロ値)。実際に`cargo test
     --workspace -- --nocapture`で確認した結果(誇張なし、実出力そのまま、
     NVIDIA GeForce GT 730):
     ```
     device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
     OK: DXBC(fxc.exe実コンパイル, 2項演算3回のチェーン add+mul+div)->SPIR-V(自前生成、式木の再帰翻訳)->実Vulkan経路が、CPU参照実装(((a[i]+b[i])*a[i])/b[i])と256要素すべてで数値一致した
     c[0]=1.0153847, c[255]=588.3
     test dxbc_vector_add_mul_div_chain3_matches_cpu_reference_on_real_vulkan_hardware ... ok

     device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
     OK: DXIL(dxc.exe実コンパイル、SM6.0、2項演算3回のチェーン add+mul+div)->SPIR-V(自前生成、resolve_dxil_calls_and_chainで式木を実解決)->実Vulkan経路が、CPU参照実装(((a[i]+b[i])*a[i])/b[i])と256要素すべてで数値一致した
     c[0]=1.0153847, c[255]=588.3
     test dxil_vector_add_mul_div_chain3_matches_cpu_reference_on_real_vulkan_hardware ... ok
     ```
     DXBC/DXIL両経路の`c[0]`/`c[255]`が完全に一致している(同一の入力
     データ生成式・同一の演算チェーンを独立した2つの実装経路で実行し、
     両方とも同じCPU参照実装に一致することを実機で確認した、既存の
     2項チェーンエントリと同じ追加的な裏付けパターン)。
  7. **ワークスペース全体の検証**: `cargo test --workspace -- --nocapture`
     で全テスト(unittests 44件+実機テスト17本〈グラフィックス5本+DXBC
     Compute単一演算4本+DXBCチェーン(add/mul, sub/div, **add/mul/div
     3項**)3本+DXIL Compute単一演算4本+DXILチェーン(add/mul, sub/div,
     **add/mul/div 3項**)3本〉)すべてgreen、既存経路への回帰なし。
     `cargo build --workspace`/`cargo clippy --workspace --all-targets`は
     いずれも警告0件。
  8. **正直な開示・まだやっていないこと(誇張しない)**:
     - **汎用N項チェーンデコーダとして正式に一般化・宣言したわけではない**。
       今回検証できたのは「2項」から「3項」への拡張が既存コードのままで
       動くことの1点のみ——4項以上、あるいは`sub`/`div`が3回以上混在する
       組み合わせ(例: `sub`→`div`→`add`のような順序)は実際にコンパイル・
       検証していない。既存コードが理屈上はN項まで対応できる形だとしても、
       「実際にテストしていない組み合わせについては保証しない」という
       このプロジェクトの一貫した方針を維持する。
     - **境界チェック付きチェーンは今回も未対応のまま**(DXBC側
       `decode_chain_shape`・DXIL側`resolve_dxil_calls_and_chain`のいずれも、
       `ult`/`if`/`endif`を伴うチェーン形状は検証していない)。
     - `mul`のnegateフラグが立つケースは引き続き未検証(2026-07-27付
       エントリから変更なし)。
     - テクスチャサンプリング・スワップチェーン・AMD/Intel/Linux/macOS
       実機検証・各クレートのexample充足状況棚卸しは、いずれも前回
       エントリから変更なし(未着手のまま)。
  - 次にすべきこと: (1) 4項以上のチェーン、または`sub`/`div`を含む
    より複雑な演算順序組み合わせでの実シェーダー検証(汎用N項デコーダと
    正式に呼べるかどうかの判断はこれらの追加検証を経てから行う)、
    (2) 境界チェック付きチェーン(DXBC/DXIL両方とも未対応)、
    (3) テクスチャサンプリング・スワップチェーンへの拡張、
    (4) AMD/Intel・Linux/macOS実機検証、(5) 各クレートの公開API
    example充足状況の棚卸し。

- **2026-08-06 直前エントリの次にすべきこと(2)を解消: DXBC Computeチェーン
  クラス(`decode_chain_shape`)へ境界チェック(`dcl_constantbuffer`/`ult`/
  `if`/`endif`)対応を追加。あわせて`open-cuda`との連携性改善(カーネル名
  ハードコード問題の重複コメントを1箇所へ集約)を実施(ユーザー指示
  「open-directx、open-cuda、aruaru-llmの連携性と実用性と完成度を高めて」)。
  作業中、同一リポジトリを対象とする別セッション(4個の逐次2項演算
  `vector_sub_div_add_mul_chain4`シリーズ)が並行して稼働中であることが
  `git status`のuntracked file・編集中ファイルの内容変化から判明し、
  以下の作業はそのファイル群(`vector_sub_div_add_mul_chain4*`)には触れず
  進めた(詳細は5参照)**:
  1. **事前確認**: `../open-cuda/crates/opencuda-vulkan/src/real.rs`の
     `launch_kernel`実装を再確認し、カーネル名で引数配線を決める既知の齟齬
     (`"vector_add"`/`"vector_add_f32"`/`"matmul"`/`"matmul_f32"`/
     `"raid6_xor_parity"`/`"raid6_q_parity"`のみ認識)に変更が無いことを
     確認した(`open-cuda`側は今回も変更しない方針を継続)。`aruaru-llm`は
     過去の調査結果通り直接の技術的依存関係が無いことも再確認(CLAUDE.md/
     README.mdを読んだ範囲でopencuda-vulkan/opencuda-core/open-directxへの
     直接依存は見当たらない)。
  2. **連携性改善(open-cuda側齟齬の緩和、open-directx側でできる範囲)**:
     `crates/directx-shader-translate/src/lib.rs`に
     `pub const OPENCUDA_VULKAN_DISPATCH_KERNEL_NAME: &str = "vector_add"`
     を新設し、上記の齟齬に関する説明を1箇所に集約した。これまで
     `tests/*.rs`の13ファイルに個別にハードコードされていた
     `"vector_add"`リテラル+重複した説明コメントを、この定数の参照へ
     置き換えた(`vector_sub_div_add_mul_chain4*`の2ファイルは前述の並行
     セッションの作業中ファイルのため意図的に対象外とした)。`open-cuda`
     側のコードは一切変更していない——齟齬そのものの解消ではなく、
     open-directx側での重複を減らす緩和策である点を正直に開示する。
  3. **実用性改善(境界チェック付きチェーン、DXBC側)**: 新規シェーダー
     `shaders/vector_add_mul_chain_bounded.hlsl`(`if (i < N) { t = A[i]+B[i];
     Out[i] = t*A[i]; }`、UAV3本)を実際に`fxc.exe /T cs_5_0`でコンパイルし、
     `examples/dump_shex`で実SHEX命令列をダンプして確認した(推測実装
     ではない): `dcl_globalFlags` -> `dcl_constantbuffer`(b0) ->
     `dcl_uav_structured`x3 -> `dcl_input`(vThreadID) -> `dcl_temps`(1) ->
     `dcl_thread_group` -> `ult` -> `if` -> `ld_structured`x2 -> `add` ->
     `mul` -> `store_structured` -> `endif` -> `ret`、という構成——既存クラス
     側`decode_shader_shape`の境界チェック規約(cbuffer b0+ult+if+endifが
     全部揃うか全く無いか)と全く同じ規約がチェーンクラス側でも成立する
     ことを実バイト列で確認した。
  4. **実装**(`crates/directx-shader-translate/src/spirv_gen.rs`):
     `ChainShape`に`bounds_check: bool`フィールドを追加、`decode_chain_shape`
     に`DclConstantBuffer`/`Opcode::ULt`/`If`/`EndIf`の処理を既存クラス側と
     同じ規約で追加(既存のadd/mul/sub/div検出ロジックは無変更)。
     `ChainTranslatedKernel`に`bounds_check: bool`を追加。
     `emit_chain_spirv_for_kernel`(DXIL側`dxil.rs`とも共有する本体)に
     `bounds_check: bool`パラメータを追加し、真の場合は既存クラス側
     `emit_spirv_impl`と全く同じpush constant`Params{uint n}`+
     `OpSelectionMerge`/`OpBranchConditional`で`id.x < n`を実際にゲートする
     ようにした。DXIL側(`dxil.rs`)の呼び出しは境界チェック検出ロジックを
     持たないため、シグネチャ変更に伴い明示的に`bounds_check=false`を渡す
     よう更新(DXIL側の挙動自体は無変更、次にすべきことに残す)。
  5. **並行セッションとの整合性**: 作業中に`cargo build`が一時的に
     ビルドディレクトリのファイルロック待ちになったことや、他セッションの
     `dxil::tests::resolves_real_dxc_compiled_4op_chain_dxil_into_matching_
     regexpr_tree`/`translate_dxil_chain_to_spirv_handles_4op_chain`が
     一時的に失敗した状態を実際に観測した(このパスでの変更が原因ではない
     ことを`git diff`で確認済み——該当コードは`dxil.rs`の別関数で自分は
     触っていない)。最終的な`cargo test --workspace`実行時にはこれらは
     解消されており(並行セッション側が完了させたと推測される)全green
     だった。
  6. **実際に`cargo test --workspace -- --nocapture`で確認した結果
     (誇張なし、実出力そのまま、NVIDIA GeForce GT 730)**:
     ```
     test spirv_gen::tests::translates_real_fxc_compiled_bounded_chain_dxbc_to_valid_spirv_with_bounds_check_flag_set ... ok
     ...
     test result: ok. 47 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

     test dxbc_vector_add_mul_chain_bounded_matches_cpu_reference_and_respects_bounds_on_real_vulkan_hardware ... ok
     test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.58s
     ```
     新規実機テスト`tests/vector_add_mul_chain_bounded_real_vulkan.rs`は
     `vector_sub_bounded_real_vulkan.rs`と同じ検証パターン(ディスパッチ数
     320・論理要素数256、境界外64要素がセンチネル値`-1.0`のまま書き込まれ
     ないことを確認)で実行し、標準出力は
     `c[0]=81, c[255]=1132.875, c[319]=-1`——有効範囲全てが
     `(a[i]+b[i])*a[i]`のCPU参照実装と一致し、境界外はセンチネルのまま
     だった。ワークスペース全体(グラフィックス5本+DXBC Compute単一演算
     4本+DXBCチェーン(add/mul, sub/div, 3項, **境界チェック付き2項、
     新規**)4本+DXIL Compute単一演算4本+DXILチェーン(add/mul, sub/div,
     3項, 4項〈並行セッション側〉)4本、計17本の実機テスト+unittests 47件)
     すべてgreen、既存経路への回帰なし。`cargo build --workspace`/
     `cargo clippy --workspace --all-targets`はいずれも警告0件。
  7. **正直な開示・まだやっていないこと(誇張しない)**:
     - **DXIL側の境界チェック対応は未着手**(`resolve_dxil_calls_and_chain`
       は`ult`/`if`/`endif`相当の検出ロジックを持たない、
       `emit_chain_spirv_for_kernel`は`bounds_check=false`固定で呼ばれる)。
     - **`open-cuda`側のカーネル名ハードコード自体は解消していない**
       (指示通り`open-cuda`は変更しない方針、今回の対応はopen-directx側の
       重複コメントの集約に留まる緩和策)。
     - `mul`のnegateフラグが立つケース(2026-07-27付エントリから継続)、
       4項以上・複雑な順序組み合わせでの境界チェック付きチェーンの検証
       (今回は2項のみ)は未検証のまま。
     - テクスチャサンプリング・スワップチェーン・AMD/Intel/Linux/macOS
       実機検証・各クレートのexample充足状況棚卸しは前回エントリから
       変更なし(未着手のまま)。
  - 次にすべきこと: (1) DXIL側の境界チェック対応
    (`resolve_dxil_calls_and_chain`への`ult`/`if`/`endif`相当の検出ロジック
    追加、DXBC側`decode_chain_shape`の今回の拡張と対になる)、(2) 境界
    チェック付きチェーンの3項以上への拡張、(3) テクスチャサンプリング・
    スワップチェーンへの拡張、(4) AMD/Intel・Linux/macOS実機検証、
    (5) 各クレートの公開APIexample充足状況の棚卸し。

- **2026-08-06 直前エントリの次にすべきこと(1)を解消: 4個の逐次2項演算、
  かつ`sub`が先頭に来る新しい順序(`sub->div->add->mul`)での実シェーダー
  検証をDXBC/DXIL両方で実施(実バイト列確認→テスト実行の順、コード変更は
  0行)**:
  1. **新規シェーダー2本**(既存のチェーン群と同じUAV3本・InputA/InputB
     多重参照パターン、演算を4回・sub先頭の順序へ拡張): `shaders/
     vector_sub_div_add_mul_chain4.hlsl`(`t1=InputA[i]-InputB[i];
     t2=t1/InputA[i]; t3=t2+InputB[i]; Output[i]=t3*InputA[i];`、
     `fxc.exe /T cs_5_0`で実コンパイル)・`shaders/
     vector_sub_div_add_mul_chain4_dxil.hlsl`(同一契約、`dxc.exe -T
     cs_6_0`で実コンパイル)。`tools/compile-dxbc-shaders.ps1`に両方追記済み
     (このスクリプトは同時に別セッションが`vector_add_mul_chain_bounded`
     向けの行を追記していたため、`git diff --stat`で自分の追記行が
     壊れていないことを確認した上で進めた)。
  2. **既存方針の継続(実バイト列を確認してから着手)**:
     - **DXBC側**: `examples/dump_shex`で実SHEX命令列をダンプした結果、
       `ld_structured`x2 -> `Add`(第1オペランドに`negate`フラグが立った
       negated-add=既存の「sub最適化」規約通り、dest=temp.w) ->
       `Div`(dest=temp.w、さらに上書き) -> `Add`(dest=temp.x) ->
       `Mul`(dest=temp.x)という、一時レジスタのコンポーネントを使い回す
       reg_map更新パターンが4回連続で続く形だった。既存の
       `decode_chain_shape`(`spirv_gen.rs`)は`negate`フラグを`Add`のみで
       「sub最適化」として解釈する既存分岐で、この4項目チェーンでも
       追加のコード変更なしに正しく`Sub`へ変換した。
     - **DXIL側**: `examples/dump_dxil`で`FUNCTION_BLOCK`をダンプした結果、
       `Call`(code=34)は既存の単一/2項/3項チェーンと変わらず**7個のまま**、
       `ExtractValue`(code=26)が2個、**`BinOp`(code=2)が4回連続で出現**
       (`fields=[3,1,1,31]`(sub、第1オペランドが`negate`ではなく
       別のBinOp種別コード自体がSubを直接表す形——チェーン3項までの
       add最適化パターンとは異なり、`sub`がチェーンの先頭に来る場合は
       LLVM側がnegated-addへ変換せず素直にSub命令を出す実装だった)
       -> `fields=[1,4,4,31]`(div) -> `fields=[3,1,0,31]`(add) ->
       `fields=[1,6,2,31]`(mul))という構造だった。`resolve_dxil_calls_and_
       chain`(`dxil.rs`)はこの4項目・かつSub直接コードの形にも変更なしで
       対応できた。
  3. **実装(コード変更は0行、確認のみ)**: DXBC側`decode_chain_shape`・
     DXIL側`resolve_dxil_calls_and_chain`のいずれも、命令列を1つずつ走査
     して`reg_map`/`chain_exprs`を更新するという既存の一般ロジックのまま、
     4項・かつ`sub`が先頭に来る新しい順序をそのまま正しく処理できることを
     実際に単体テスト・実機テストの両方で確認した(タスク指示通りコードを
     書く前にまず実バイト列を確認したが、結果的にプロダクションコード自体
     への変更は不要だった、直前の2026-08-05エントリと同じ結論)。
  4. **正直な発見(既知の現象の再確認+新しい観察)**:
     - DXIL側の式木を`resolve_dxil_calls_and_chain`で実際に取得して
       `{:#?}`でダンプしたところ、額面通りの`Add(Div(Sub(a,b),a), b)`
       ではなく`Add(Load(b), Div(Sub(a,b),a))`(第3演算=addのオペランド
       順序が入れ替わった形)だった——これは2026-07-26/2026-08-05付
       HANDOFFエントリで既に確認済みの「addは可換演算のためdxc/LLVMの
       最適化パスが相対値参照の順序を並べ替える」現象が、subが先頭に来る
       この4項チェーンでも再現したもの(数値的には`a+b=b+a`のため実行
       結果に影響しない、実Vulkanで数値一致することで裏付け済み)。
       手計算・額面通りの期待だけでテストを書かず実行結果で裏取りする
       という教訓を今回も踏襲し、テストの期待値は実測した木の形
       (`Mul(Add(Load,Div(Sub(Load,Load),Load)),Load)`)に修正した。
     - 読み込みUAVバインドポイント一覧(`read_uav_bind_points`)の長さは
       4項チェーンで**5**(N+1規則、既存の3項チェーンで4=3+1だったのと
       同じ規則)になる——最初に書いたテストでは額面通り4を期待して
       実際に失敗し(`left: 5, right: 4`)、実測値に合わせて修正した。
       これも「実行して初めて分かった」正直な訂正。
  5. **単体テスト追加**(`cargo test -p directx-shader-translate --lib`、
     実際に確認、誇張なし):
     ```
     running 2 tests
     test dxil::tests::resolves_real_dxc_compiled_4op_chain_dxil_into_matching_regexpr_tree ... ok
     test dxil::tests::translate_dxil_chain_to_spirv_handles_4op_chain ... ok
     test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 45 filtered out
     ```
  6. **実機テスト2本を新規追加**(`tests/vector_sub_div_add_mul_chain4_
     real_vulkan.rs`〈DXBC〉・`tests/vector_sub_div_add_mul_chain4_dxil_
     real_vulkan.rs`〈DXIL〉、既存のチェーン実機テストと同じパターン、
     ゼロ除算を避けるためaは常に正の非ゼロ値)。実際に確認した結果
     (誇張なし、実出力そのまま、NVIDIA GeForce GT 730):
     ```
     device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
     OK: DXBC(fxc.exe実コンパイル, 2項演算4回のチェーン sub->div->add->mul)->SPIR-V(自前生成、式木の再帰翻訳)->実Vulkan経路が、CPU参照実装(((a[i]-b[i])/a[i]+b[i])*a[i])と256要素すべてで数値一致した
     c[0]=1, c[255]=58.375004
     test dxbc_vector_sub_div_add_mul_chain4_matches_cpu_reference_on_real_vulkan_hardware ... ok

     device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
     OK: DXIL(dxc.exe実コンパイル、SM6.0、2項演算4回のチェーン sub->div->add->mul)->SPIR-V(自前生成、resolve_dxil_calls_and_chainで式木を実解決)->実Vulkan経路が、CPU参照実装(((a[i]-b[i])/a[i]+b[i])*a[i])と256要素すべてで数値一致した
     c[0]=1, c[255]=58.375004
     test dxil_vector_sub_div_add_mul_chain4_matches_cpu_reference_on_real_vulkan_hardware ... ok
     ```
     DXBC/DXIL両経路の`c[0]`/`c[255]`が完全に一致している(既存のチェーン
     エントリと同じ、独立2経路の追加的な裏付けパターン)。
  7. **ワークスペース全体の検証**: 作業中、別セッションが同じワーク
     ツリーで並行して境界チェック付きチェーン(次にすべきこと(2))に
     着手しており(`spirv_gen.rs`が一時的にコンパイル不能な中間状態に
     なっていた場面に遭遇したため、自分の変更が絡んでいないことを
     `git diff --stat`で確認しつつ、相手の変更が安定するまで
     `cargo build`が通るようになるのを待ってから検証を実施——相手の
     作業ファイルへの介入は一切していない)、それが収束した後の状態で
     `cargo test --workspace -- --nocapture`を実行し、自分が追加した
     テストを含め全テストgreenであることを確認した(既存経路への回帰
     なし)。`cargo build --workspace`/`cargo clippy --workspace
     --all-targets`はいずれも警告0件。ただしこの実行結果には並行作業中の
     境界チェックチェーン関連テストも含まれている可能性があるため、
     このエントリでは自分が追加した範囲(4項チェーンのDXBC/DXIL実機・
     単体テスト計4本)の合格を主張の根拠とする。
  8. **正直な開示・まだやっていないこと(誇張しない)**:
     - 検証できたのは「4項・かつsub先頭の順序」の1点のみ。5項以上、
       あるいは他の順序組み合わせ(例: `mul`が複数回連続する、`div`が
       先頭に来る等)は実際にコンパイル・検証していない。
     - 境界チェック付きチェーンは自分では未着手(別セッションが並行して
       着手している形跡を確認したが、その内容自体はこのエントリの
       範囲外であり、自分では検証・追記していない)。
     - `mul`のnegateフラグが立つケースは引き続き未検証(2026-07-27付
       エントリから変更なし)。
     - テクスチャサンプリング・スワップチェーン・AMD/Intel/Linux/macOS
       実機検証・各クレートのexample充足状況棚卸しは、いずれも前回
       エントリから変更なし(未着手のまま)。
  - 次にすべきこと: (1) 5項以上、または今回未検証の順序組み合わせ
    (`div`が先頭に来る等)での追加検証、(2) 境界チェック付きチェーン
    (別セッションが並行して着手した形跡があるため、次回はまずその
    セッションの成果が確定しているか`git status`/`git log`で確認する
    こと)、(3) テクスチャサンプリング・スワップチェーンへの拡張、
    (4) AMD/Intel・Linux/macOS実機検証、(5) 各クレートの公開API
    example充足状況の棚卸し。

- **2026-08-06(続き) 直前エントリの次にすべきこと(2)を解消: DXIL側の
  境界チェック付きチェーン対応を追加(DXBC側は2026-08-06の別エントリで
  既に対応済み、DXIL側だけ`bounds_check=false`固定のまま残っていた
  ギャップを埋めた)。ユーザー指示「open-directx、open-cuda、aruaru-llmの
  連携性と実用性と完成度を高めて」の一環**:
  1. **事前確認**: `git status`はクリーン、`git log`直近8件を確認し、
     並行セッションの4項チェーン・境界チェックDXBC対応が両方とも
     mainへ確定済みであることを確認してから着手した(直前エントリの
     「次にすべきこと(2)」の指示通り)。
  2. **新規シェーダー**: `shaders/vector_add_mul_chain_bounded_dxil.hlsl`
     (`vector_add_mul_chain_bounded.hlsl`と同一契約、cbuffer(b0)の
     `ElementCount`+`if (i < ElementCount) { t=a[i]+b[i]; c[i]=t*a[i]; }`)を
     実`dxc.exe -T cs_6_0 -E main`でコンパイル
     (`tools/compile-dxbc-shaders.ps1`に追記済み)。
  3. **実バイト列を確認してから着手**(推測で実装しない、既存方針の継続):
     `examples/dump_dxil`で実際にダンプした結果、境界チェック無しの
     単一/チェーン系(`Call`7個)に対し、この境界チェック付きシェーダーは
     `DeclareBlocks(3)`(基本ブロック3個、if/then/merge) -> `Call`
     (CreateHandle)x4(UAV3本+cbuffer) -> `Call`(ThreadId) -> `Call`
     (`dx.op.cbufferLoadLegacy.i32`、opcode=59、初出) -> `ExtractValue`
     (ElementCount) -> `Cmp2`(`FUNC_CODE_INST_CMP2`、code=28、初出、
     述語36=LLVM`ICMP_ULT`) -> `Br`(`FUNC_CODE_INST_BR`、code=11、初出、
     3フィールド=条件分岐) -> `Call`(BufferLoad)x2 -> `ExtractValue`x2 ->
     `BinOp`(add) -> `BinOp`(mul) -> `Call`(BufferStore) -> `Br`
     (1フィールド=無条件分岐、mergeブロックへ) -> `Ret`という構造だった。
     `Call`命令の合計は9個(既存の7個+cbuffer用CreateHandle+
     cbufferLoadLegacy)。
  4. **実装**(`crates/directx-shader-translate/src/dxil.rs`):
     - `DxilInstruction`に`Cmp2 { fields }`(code=28)・`Br { fields }`
       (code=11)を新規追加、`decode_function_instructions`でデコード。
       既存の単一演算専用デコーダ`decode_vector_add_dxil_shape`側では
       これらを想定外として正直に拒否するよう網羅性チェックを更新。
     - `DxilValue`に`ConstantBufferLoadAggregate{cbuffer_range_id}`・
       `ExtractedConstantBufferValue{cbuffer_range_id}`・`CmpResult`を
       追加。`resolve_dxil_calls_and_chain`に`"dx.op.cbufferLoadLegacy.i32"`
       呼び出しの解決(opcode=59検証、引数3個)、`ExtractValue`の
       `ConstantBufferLoadAggregate`由来ケースへの対応、`Cmp2`
       (述語ult=36の検証+オペランドが`ThreadIdResult`と
       `ExtractedConstantBufferValue`の組であることの検証、額面通りの
       順序を仮定せずどちらの順でも受け付ける)、`Br`(3フィールド=条件分岐
       は直前に`icmp ult`があることだけ検証、1フィールド=無条件分岐)を
       追加。
     - `resolve_dxil_calls_and_chain`の戻り値を`(Vec<ResolvedDxilCall>,
       DxilRegExpr)`から`(Vec<ResolvedDxilCall>, DxilRegExpr, bool)`
       (第3要素が`bounds_check`)へ変更。Call数が7個(境界チェック無し)
       または9個(境界チェック付き、かつcbuffer関連4フラグ全部揃う)の
       いずれか以外、または個数とフラグの組み合わせが中途半端な場合は
       `UnexpectedShape`で正直に拒否する(DXBC側`decode_chain_shape`の
       `has_cbuffer != bounds_check`等と同じ設計方針)。
     - `translate_dxil_chain_to_spirv`: 以前は`bounds_check=false`固定で
       `emit_chain_spirv_for_kernel`(DXBC/DXIL共有の本体、2026-08-06の
       別エントリで既に`bounds_check`パラメータ対応済み)を呼んでいたが、
       `resolve_dxil_calls_and_chain`が返す実測値をそのまま渡すよう修正
       (このシグネチャ拡張以外、`emit_chain_spirv_for_kernel`自体は
       無改修——DXBC側が先に用意した境界チェックSPIR-V生成をDXIL側からも
       再利用できた)。
  5. **単体テスト追加**(`cargo test -p directx-shader-translate --lib`、
     実際に確認、誇張なし):
     ```
     test dxil::tests::resolves_real_dxc_compiled_bounded_chain_dxil_and_detects_bounds_check ... ok
     test dxil::tests::translate_dxil_chain_to_spirv_handles_bounded_chain ... ok
     test dxil::tests::translate_dxil_chain_to_spirv_keeps_bounds_check_false_for_unbounded_chain ... ok
     (既存47件含め) test result: ok. 50 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
     ```
     3件目は「境界チェック無しの既存チェーンを渡しても`bounds_check`が
     常にtrueに固定されていないか」の回帰防止(裏取り)。
  6. **実機テスト新規追加**(`tests/vector_add_mul_chain_bounded_dxil_real_
     vulkan.rs`、DXBC側`vector_add_mul_chain_bounded_real_vulkan.rs`と
     同一パターン、ディスパッチ数320・論理要素数256で境界外64要素が
     センチネル値のまま書き込まれないことを検証)。実際に
     `cargo test --workspace -- --nocapture`で確認した結果(誇張なし、
     実出力そのまま、NVIDIA GeForce GT 730):
     ```
     device: OpenCUDA Vulkan Device (NVIDIA GeForce GT 730)
     OK: DXIL(dxc.exe実コンパイル、SM6.0、境界チェック付き2項演算チェーン)->SPIR-V(自前生成、resolve_dxil_calls_and_chainで境界チェックも実解決)->実Vulkan経路が、CPU参照実装((a[i]+b[i])*a[i])と有効範囲256要素すべてで数値一致し、境界外の64要素はセンチネル値のまま(書き込まれなかった)ことを確認した
     c[0]=81, c[255]=1132.875, c[319]=-1
     test dxil_vector_add_mul_chain_bounded_matches_cpu_reference_and_respects_bounds_on_real_vulkan_hardware ... ok
     ```
     DXBC側の同シェーダー(`vector_add_mul_chain_bounded_real_vulkan.rs`)
     の出力`c[0]=81, c[255]=1132.875, c[319]=-1`と完全一致——同一契約の
     シェーダーをDXBC/DXIL独立2経路で翻訳・実行し、両方とも同じ結果に
     収束していることの追加的な裏付け(既存のチェーン系エントリと同じ
     検証パターン)。
  7. **ワークスペース全体の検証**: `cargo test --workspace -- --nocapture`
     で全テスト(unittests 50件+実機テスト19本〈グラフィックス5本+DXBC
     Compute単一演算4本+DXBCチェーン(add/mul, sub/div, 3項, 4項, 境界
     チェック付き2項)5本+DXIL Compute単一演算4本+DXILチェーン(add/mul,
     sub/div, 3項, 4項, **境界チェック付き2項、新規**)5本〉)すべてgreen、
     既存経路への回帰なし。`cargo build --workspace`/`cargo clippy
     --workspace --all-targets`はいずれも警告0件。
  8. **正直な開示・まだやっていないこと(誇張しない)**:
     - **境界チェック付きチェーンは2項のみ検証済み**(DXBC側と同じ限定、
       3項以上の境界チェック付きチェーンは未検証)。
     - **`open-cuda`側のカーネル名ハードコード自体は解消していない**
       (前回エントリから継続、`open-cuda`は変更しない方針)。
     - `mul`のnegateフラグが立つケース、5項以上・複雑な順序組み合わせは
       引き続き未検証(前回エントリから継続)。
     - テクスチャサンプリング・スワップチェーン・AMD/Intel/Linux/macOS
       実機検証・各クレートのexample充足状況棚卸しは前回エントリから
       変更なし(未着手のまま)。
  - 次にすべきこと: (1) 境界チェック付きチェーンの3項以上への拡張
    (DXBC/DXIL両方とも未検証)、(2) 5項以上・今回未検証の順序組み合わせ
    (`div`が先頭に来る等)、(3) テクスチャサンプリング・スワップチェーンへの
    拡張、(4) AMD/Intel・Linux/macOS実機検証、(5) 各クレートの公開API
    example充足状況の棚卸し。

- **2026-08-06(続き2) 直前エントリの次にすべきこと(1)を解消: 境界チェック
  付きチェーンを3項へ拡張して実検証(DXBC/DXIL両方、コード変更は0行——
  既存の`decode_chain_shape`/`resolve_dxil_calls_and_chain`一般化ロジックが
  そのまま通用することを確認)。ユーザー指示「open-directx、open-cuda、
  aruaru-llmの連携性と実用性と完成度を高めて」の一環**:
  1. **事前確認**: `git status`クリーン、直前のDXIL側境界チェック対応
    (13aede6)がmainへ確定済みであることを確認してから着手。
  2. **新規シェーダー2本**: `shaders/vector_add_mul_div_chain3_bounded.hlsl`
    (`if (i < ElementCount) { t1=A[i]+B[i]; t2=t1*A[i]; Out[i]=t2/B[i]; }`、
    UAV3本+cbuffer(b0)、`fxc.exe /T cs_5_0`で実コンパイル)・
    `shaders/vector_add_mul_div_chain3_bounded_dxil.hlsl`(同一契約、
    `dxc.exe -T cs_6_0`で実コンパイル)。`tools/compile-dxbc-shaders.ps1`に
    両方追記済み。
  3. **実バイト列を確認してから着手(既存方針の継続)**:
    - DXBC側: `examples/dump_shex`で実SHEX命令列をダンプ(18命令、
      `dcl_globalFlags`→`dcl_constantbuffer`(b0)→`dcl_uav_structured`x3→
      `dcl_input`(vThreadID)→`dcl_temps`(1)→`dcl_thread_group`→`ult`→
      `if`→`ld_structured`x2→`add`→`mul`→`div`→`store_structured`→
      `endif`→`ret`)——既存の境界チェック付き2項チェーン(`ult`/`if`/
      `endif`)と境界チェック無し3項チェーン(3回の逐次2項演算)の規約が
      そのまま組み合わさった形であることを確認した。
    - DXIL側: `examples/dump_dxil`で`FUNCTION_BLOCK`をダンプ(基本ブロック3
      個、`Call`合計9個(CreateHandle x4〈UAV3+cbuffer〉+ThreadId+
      cbufferLoadLegacy+BufferLoad x2+BufferStore)・`ExtractValue`2個・
      `Cmp2`(icmp ult)1個・`Br`2個(条件分岐+無条件分岐)・`BinOp`3個
      (add/mul/div)・`Ret`)——既存の境界チェック付き2項チェーン(Call計9個)
      と境界チェック無し3項チェーン(BinOp3個)の規約がそのまま組み合わさった
      形であることを確認した。
  4. **実装**: DXBC側`decode_chain_shape`・DXIL側`resolve_dxil_calls_and_
    chain`のいずれも、命令列を1つずつ走査してreg_map/式木を更新する既存の
    一般ロジックのまま、境界チェック+3項という組み合わせをそのまま正しく
    処理できることを実際に単体テスト・実機テストの両方で確認した
    (プロダクションコードへの変更は0行、直前2026-08-06エントリ〈4項
    チェーン〉と同じ結論)。
  5. **単体テストは既存の28件+境界チェック関連22件(計50件)から変化なし**
    (今回の増分は`translate_chain_shader`/`translate_dxil_chain_to_spirv`の
    既存呼び出しパスをそのまま通すのみのため、新規の単体テストは追加せず
    実機テストのみ追加——実機テスト自体が`kernel.read_uav_bind_points.len()
    == 4`・`kernel.bounds_check == true`等のアサーションを含むため、単体
    レベルの回帰防止としても機能する)。
  6. **実機テスト2本を新規追加**(`tests/vector_add_mul_div_chain3_bounded_
    real_vulkan.rs`〈DXBC〉・`tests/vector_add_mul_div_chain3_bounded_dxil_
    real_vulkan.rs`〈DXIL〉、既存の境界チェック付きチェーン実機テストと同じ
    パターン、ディスパッチ数320・論理要素数256、境界外64要素がセンチネル値
    のまま書き込まれないことを検証)。実際に確認した結果(誇張なし、実出力
    そのまま、NVIDIA GeForce GT 730):
    ```
    OK: DXBC(fxc.exe実コンパイル, 境界チェック付き3項演算チェーン add->mul->div)->SPIR-V(自前生成、式木の再帰翻訳+OpSelectionMerge/OpBranchConditional)->実Vulkan経路が、CPU参照実装(((a[i]+b[i])*a[i])/b[i])と有効範囲256要素すべてで数値一致し、境界外の64要素はセンチネル値のまま(書き込まれなかった)ことを確認した
    c[0]=1.0123457, c[255]=67.210144, c[319]=-1
    test dxbc_vector_add_mul_div_chain3_bounded_matches_cpu_reference_and_respects_bounds_on_real_vulkan_hardware ... ok

    OK: DXIL(dxc.exe実コンパイル、SM6.0、境界チェック付き3項演算チェーン add->mul->div)->SPIR-V(自前生成、resolve_dxil_calls_and_chainで境界チェックも実解決)->実Vulkan経路が、CPU参照実装(((a[i]+b[i])*a[i])/b[i])と有効範囲256要素すべてで数値一致し、境界外の64要素はセンチネル値のまま(書き込まれなかった)ことを確認した
    c[0]=1.0123457, c[255]=67.210144, c[319]=-1
    test dxil_vector_add_mul_div_chain3_bounded_matches_cpu_reference_and_respects_bounds_on_real_vulkan_hardware ... ok
    ```
    DXBC/DXIL両経路の`c[0]`/`c[255]`/`c[319]`が完全に一致している
    (既存のチェーン系エントリと同じ、独立2経路の追加的な裏付けパターン)。
    `read_uav_bind_points.len() == 4`(境界チェック無し3項チェーンと同じ
    N+1規則)も実測で確認した。
  7. **ワークスペース全体の検証**: `cargo test --workspace`で全テスト
    (unittests 50件、変化なし+実機テスト21本〈既存19本+今回2本〉)すべて
    green、既存経路への回帰なし。`cargo build --workspace`/`cargo clippy
    --workspace --all-targets`はいずれも警告0件。
  8. **正直な開示・まだやっていないこと(誇張しない)**:
    - **検証できたのは「境界チェック付き3項」の1点のみ**。4項以上の境界
      チェック付きチェーンは未検証。
    - `mul`のnegateフラグが立つケース、5項以上・今回未検証の順序組み合わせ
      (境界チェック無し版)は引き続き未検証(前回エントリから継続)。
    - **open-cuda・aruaru-llmとの連携**: 今回のセッションで両方のCLAUDE.md
      を再確認したが、2026-07-25付エントリの結論(open-cudaの
      `opencuda-vulkan`をVulkan実行バックエンドとして既に再利用済み〈実配線
      済み、cross-repoパス依存〉、aruaru-llmとの直接の技術的依存は無し)から
      変化は無かった。`opencuda-vulkan::VulkanDevice::launch_kernel`の
      カーネル名ハードコード(`"vector_add"`/`"matmul"`のみ認識、
      `OPENCUDA_VULKAN_DISPATCH_KERNEL_NAME`定数で重複コメントは集約済み
      〈2026-08-06別エントリ〉)が実用性上のボトルネックとして残っている点も
      変化なし——open-cuda側の変更は今回のタスク範囲では不要と判断した
      (境界チェック付き3項チェーンの検証は既存の配線をそのまま再利用
      できたため)。
    - テクスチャサンプリング・スワップチェーン・AMD/Intel/Linux/macOS
      実機検証・各クレートのexample充足状況棚卸しは前回エントリから
      変更なし(未着手のまま)。
  - 次にすべきこと: (1) 4項以上の境界チェック付きチェーン、(2) 5項以上・
    今回未検証の順序組み合わせ(境界チェック無し版、`div`が先頭に来る等)、
    (3) テクスチャサンプリング・スワップチェーンへの拡張、(4) AMD/Intel・
    Linux/macOS実機検証、(5) 各クレートの公開APIexample充足状況の棚卸し、
    (6) `opencuda-vulkan::VulkanDevice::launch_kernel`のカーネル名
    ハードコード自体の解消(open-cuda側の変更が必要、ユーザー確認の上で
    着手すること)。

- **2026-08-06(続き3) 直前エントリの次にすべきこと(1)を解消: 境界チェック
  付きチェーンを4項へ拡張して実検証(DXBC/DXIL両方、コード変更は0行——
  既存の`decode_chain_shape`/`resolve_dxil_calls_and_chain`一般化ロジックが
  そのまま通用することを再確認)**:
  1. **事前確認**: `git status`クリーン、直前の境界チェック付き3項チェーン
    対応(cbd0947)がmainへ確定済みであることを確認してから着手。
  2. **新規シェーダー2本**: `shaders/vector_add_mul_div_sub_chain4_bounded.hlsl`
    (`if (i < ElementCount) { t1=A[i]+B[i]; t2=t1*A[i]; t3=t2/B[i];
    Out[i]=t3-A[i]; }`、UAV3本+cbuffer(b0)、境界チェック付き3項〈add/mul/
    div〉へsubをもう1回追加、`fxc.exe /T cs_5_0`で実コンパイル)・
    `shaders/vector_add_mul_div_sub_chain4_bounded_dxil.hlsl`(同一契約、
    `dxc.exe -T cs_6_0`で実コンパイル)。`tools/compile-dxbc-shaders.ps1`に
    両方追記済み。
  3. **実バイト列を確認してから着手(既存方針の継続)**: DXBC側
    `examples/dump_shex`で実SHEX命令列をダンプ(20命令、境界チェック付き
    3項チェーンの構成〈`dcl_constantbuffer`→`ult`→`if`→...→`endif`〉に
    4回目の演算(`sub`、実際には第1オペランドに`negate`フラグが立った
    `add`——既存のnegated-add規約通り)が追加されただけの形だった。DXIL側
    `examples/dump_dxil`で`FUNCTION_BLOCK`をダンプし、`Call`合計9個
    (境界チェック付き3項チェーンと同じ、UAV3本+cbuffer由来で演算回数には
    依存しない)・`BinOp`4個(add/mul/div/sub)という構造を確認した。
  4. **実装**: DXBC側`decode_chain_shape`・DXIL側`resolve_dxil_calls_and_
    chain`のいずれも無改修で境界チェック+4項の組み合わせを正しく処理
    できることを単体テスト(既存パス経由、新規追加なし)・実機テストの
    両方で確認した(プロダクションコードへの変更は0行、直前2エントリと
    同じ結論)。
  5. **実機テスト2本を新規追加**(`tests/vector_add_mul_div_sub_chain4_
    bounded_real_vulkan.rs`〈DXBC〉・`tests/vector_add_mul_div_sub_chain4_
    bounded_dxil_real_vulkan.rs`〈DXIL〉、既存の境界チェック付きチェーン
    実機テストと同じパターン、ディスパッチ数320・論理要素数256)。実際に
    確認した結果(誇張なし、実出力そのまま、NVIDIA GeForce GT 730)、
    DXBC/DXIL両経路とも完全に同じ値:
    ```
    c[0]=0.012345672, c[255]=40.710144, c[319]=-1
    test dxbc_vector_add_mul_div_sub_chain4_bounded_matches_cpu_reference_and_respects_bounds_on_real_vulkan_hardware ... ok
    test dxil_vector_add_mul_div_sub_chain4_bounded_matches_cpu_reference_and_respects_bounds_on_real_vulkan_hardware ... ok
    ```
    有効範囲256要素すべてでCPU参照実装`((a[i]+b[i])*a[i])/b[i]-a[i]`と
    一致し、境界外64要素はセンチネル値のまま(書き込まれなかった)ことを
    確認した。
  6. **ワークスペース全体の検証**: `cargo test --workspace`で全テスト
    (unittests 50件、変化なし+実機テスト計23本〈既存21本+今回2本〉)すべて
    green、既存経路への回帰なし。`cargo build --workspace`/`cargo clippy
    --workspace --all-targets`はいずれも警告0件。
  7. **正直な開示・まだやっていないこと**: 検証できたのは「境界チェック
    付き4項」の1点のみ。5項以上は未検証。`mul`のnegateフラグが立つケース、
    今回未検証の順序組み合わせ(境界チェック無し版)は引き続き未検証。
    open-cuda・aruaru-llmとの連携状況は前回エントリから変化なし。
    テクスチャサンプリング・スワップチェーン・AMD/Intel/Linux/macOS実機
    検証・各クレートのexample充足状況棚卸しも前回エントリから変更なし
    (未着手のまま)。
  - 次にすべきこと: (1) 5項以上の境界チェック付きチェーン、または今回
    未検証の順序組み合わせ(境界チェック無し版、`div`が先頭に来る等)、
    (2) テクスチャサンプリング・スワップチェーンへの拡張、(3) AMD/Intel・
    Linux/macOS実機検証、(4) 各クレートの公開APIexample充足状況の棚卸し、
    (5) `opencuda-vulkan::VulkanDevice::launch_kernel`のカーネル名
    ハードコード自体の解消(open-cuda側の変更が必要、ユーザー確認の上で
    着手すること)。

- **2026-08-07 直前エントリの次にすべきこと(1)を解消: 境界チェック付き
  チェーンを5項へ拡張して実検証(DXBC/DXIL両方、コード変更は0行——
  既存の`decode_chain_shape`/`resolve_dxil_calls_and_chain`一般化ロジックが
  そのまま通用することを再確認)。ユーザー指示「dream-os・open-directx・
  open-cuda・aruaru-llmの連携性強化・実用性向上・利便性向上・完成度向上」
  (4リポジトリ並列作業、このリポジトリ内で完結)の一環**:
  1. **事前確認**: `git status`クリーン、直前の境界チェック付き4項
    チェーン対応がmainへ確定済みであることを確認してから着手。
  2. **新規シェーダー2本**: `shaders/vector_add_mul_div_sub_add_chain5_
    bounded.hlsl`(`if (i < ElementCount) { t1=A[i]+B[i]; t2=t1*A[i];
    t3=t2/B[i]; t4=t3-A[i]; Out[i]=t4+B[i]; }`、UAV3本+cbuffer(b0)、境界
    チェック付き4項〈add/mul/div/sub〉へaddをもう1回追加、`fxc.exe /T
    cs_5_0`で実コンパイル)・`shaders/vector_add_mul_div_sub_add_chain5_
    bounded_dxil.hlsl`(同一契約、`dxc.exe -T cs_6_0`で実コンパイル)。
    `tools/compile-dxbc-shaders.ps1`に両方追記済み。
  3. **実バイト列を確認してから着手(既存方針の継続)**: DXBC側
    `examples/dump_shex`で実SHEX命令列をダンプ(22命令、境界チェック付き
    4項チェーンの構成に5回目の演算(素直な`add`)が追加されただけの形
    だった)。DXIL側`examples/dump_dxil`で`FUNCTION_BLOCK`をダンプし、
    `Call`合計9個(境界チェック付き4項チェーンと同じ、UAV3本+cbuffer由来で
    演算回数には依存しない)・`ExtractValue`3個・`Cmp2`1個・`Br`2個・
    `BinOp`5個(add/mul/div/negated-add-as-sub/add)という構造を確認した。
  4. **実装**: DXBC側`decode_chain_shape`・DXIL側`resolve_dxil_calls_and_
    chain`のいずれも無改修で境界チェック+5項の組み合わせを正しく処理
    できることを実機テストで確認した(プロダクションコードへの変更は
    0行、直前3エントリと同じ結論)。
  5. **実機テスト2本を新規追加**(`tests/vector_add_mul_div_sub_add_chain5_
    bounded_real_vulkan.rs`〈DXBC〉・`tests/vector_add_mul_div_sub_add_
    chain5_bounded_dxil_real_vulkan.rs`〈DXIL〉、既存の境界チェック付き
    チェーン実機テストと同じパターン、ディスパッチ数320・論理要素数256)。
    実際に確認した結果(誇張なし、実出力そのまま、NVIDIA GeForce GT 730)、
    DXBC/DXIL両経路とも完全に同じ値:
    ```
    c[0]=81.012344, c[255]=57.960144, c[319]=-1
    test dxbc_vector_add_mul_div_sub_add_chain5_bounded_matches_cpu_reference_and_respects_bounds_on_real_vulkan_hardware ... ok
    test dxil_vector_add_mul_div_sub_add_chain5_bounded_matches_cpu_reference_and_respects_bounds_on_real_vulkan_hardware ... ok
    ```
    有効範囲256要素すべてでCPU参照実装`((a[i]+b[i])*a[i])/b[i]-a[i]+b[i]`
    と一致し、境界外64要素はセンチネル値のまま(書き込まれなかった)ことを
    確認した。`read_uav_bind_points.len() == 6`(N+1規則、5項チェーンでも
    成立)もDXBC側で実測確認した。
  6. **ワークスペース全体の検証**: `cargo build --workspace`/`cargo clippy
    --workspace --all-targets`はいずれも警告0件。`cargo test --workspace`
    は既存の全実機テスト(グラフィックス5本+DXBC/DXIL Compute単一演算計8本
    +DXBC/DXILチェーン群〈add/mul, sub/div, 3項, 4項, 4項〈sub先頭〉,
    境界チェック付き2/3/4項〉計多数+今回の5項境界チェック付きチェーン2本)
    +unittests(既存50件、今回の増分は既存呼び出しパスをそのまま通すのみ
    のため単体テストは追加せず実機テストのみ追加)すべてgreenであることを
    確認した(既存経路への回帰なし)。
  7. **正直な開示・まだやっていないこと**: 検証できたのは「境界チェック
    付き5項」の1点のみ。6項以上は未検証。`mul`のnegateフラグが立つケース、
    5項以外の順序組み合わせ(境界チェック無し版、`div`が先頭に来る等)は
    引き続き未検証。open-cuda・aruaru-llmとの連携状況は前回エントリから
    変化なし(このパスではopen-cuda/aruaru-llm側のファイルには一切触れて
    いない、指示通り担当リポジトリ内で完結させた)。テクスチャサンプリング・
    スワップチェーン・AMD/Intel/Linux/macOS実機検証・各クレートの
    example充足状況棚卸しも前回エントリから変更なし(未着手のまま)。
    dream-os由来の東芝SBM/DeepSeek技術組み込み構想(本CLAUDE.md冒頭の
    保留タスク)にも今回は着手していない(dream-os側の技術詳細調査が
    前提のため、今回は既存の境界チェックチェーン一般化路線の延長を優先
    した——次回の優先度判断はユーザー確認の上で行う)。
  - 次にすべきこと: (1) 6項以上の境界チェック付きチェーン、または今回
    未検証の順序組み合わせ(境界チェック無し版、`div`が先頭に来る等)、
    (2) テクスチャサンプリング・スワップチェーンへの拡張、(3) AMD/Intel・
    Linux/macOS実機検証、(4) 各クレートの公開APIexample充足状況の棚卸し、
    (5) `opencuda-vulkan::VulkanDevice::launch_kernel`のカーネル名
    ハードコード自体の解消(open-cuda側の変更が必要、ユーザー確認の上で
    着手すること)、(6) 冒頭の東芝SBM/DeepSeek技術組み込み構想(調査から
    着手が必要、dream-os側CLAUDE.mdの詳細確認が前提)。
