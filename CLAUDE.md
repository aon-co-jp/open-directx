# 開発方針＆開発環境ルール(open-directx)

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
