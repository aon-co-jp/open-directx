# PORTING.md — 何が、誰にとって、どう再利用可能か(要約版)

> **注記**: これは要約版の翻訳です。コードの詳細や落とし穴まで含む
> 完全な技術ガイドは、原文の`PORTING.md`にのみ収録されています——実際に
> パターンを採用する前に、必ずそちらを確認してください。

このプロジェクトから他プロジェクトへ移植する人向けに、再利用可能な
実装パターンをまとめる:

1. **`crates/directx-shader-translate`**: DXBC(D3D9/10/11シェーダー
   バイトコード、SM<=5.1)のコンテナ/チャンクパーサー(`parse_dxbc`)、
   および本プロジェクト自身のシェーダーに対して`fxc.exe`が実際に出力する
   オペコード形状のみを狭く対象とした、DXBC(SM5.0)→SPIR-Vトランスレータ
   (`translate_shader`): 単一演算(add/mul/negated-add経由のsub/div)、
   任意長の逐次2項演算チェーン(長さごとのハードコードではなく、
   レジスタ式木ベース——項数ごとに本番コードの変更が*ゼロ*のまま7項まで
   検証済み)、境界チェック付き(`if (id.x < N)`)の各バリエーションに対応し、
   実際の`OpSelectionMerge`/`OpBranchConditional`のSPIR-Vを生成する。
   それ以外のオペコード形状は`SpirvGenError::UnsupportedShader`で
   正直に拒否され、黙って誤翻訳されることはない——これは**汎用**SM5.0
   デコーダでは**ない**。
2. **DXIL(SM6+、D3D12)対応**(`src/dxil.rs`): `parse_dxil_container`は
   (汎用の`llvm-bitcode`クレート経由で)生のLLVMビットストリームを
   走査し、型テーブルを解決して関数命令をデコードする。
   `resolve_vector_add_dxil_calls`系の関数群は、DXILの`dx.op.*`組み込み
   関数呼び出し(CreateHandle/ThreadId/BufferLoad/BufferStore)を、
   Microsoft公式のDXIL.rstオペコード番号と突き合わせて解決する。
   `translate_dxil_..._to_spirv`はDXBCバックエンドと1つのSPIR-Vエミッタ
   (`emit_spirv_for_kernel`)を共有しており、両方のコンテナ形式が単一の
   コードパスを通じてSPIR-Vを生成する。ワークグループサイズ
   (`numthreads`)は、DXILの`METADATA_BLOCK`(`dx.entryPoints` →
   `ShaderProperties`)から実際に抽出されており、ハードコードでは
   ない。境界チェック付きチェーンの一般化(上記参照)は、DXBC/DXIL
   両方で最大7項までエンドツーエンドで検証済み。
3. **D3D11グラフィックスパイプライン**: `translate_vertex_shader`/
   `translate_pixel_shader`は固定のパススルーVS/PSペア向けに実際の
   グラフィックスSPIR-Vを発行し、独立した2通りの方法(rspirvによる
   再パース+実Vulkan SDKの`spirv-val.exe`、いずれも終了コード0)で
   検証済み。新規クレート`crates/directx-graphics-vulkan`は`ash`を
   本プロジェクト自身の**直接**依存として追加(`opencuda-vulkan`の
   上には重ねていない——ソース監査によりコンピュートディスパッチ専用と
   確認済み)、実際のレンダーパス・フレームバッファ・
   `VkGraphicsPipelineCreateInfo`を実装する。三角形の描画・読み戻し、
   および標準的な"over"アルファブレンドを使ったテクスチャ付き/複数
   スプライトのシーンの描画・読み戻しと、実PNGファイルからのテクスチャ
   読み込み(`png`クレート)を行い、いずれも実NVIDIA GT 730ハードウェア
   (Windows)*と*WSL2 Ubuntu/Mesa llvmpipe(Linux)の両方で検証済み、
   両OSで結果が一致する。
4. **`crates/directx-graphics-window`**: 実ウィンドウ+実Vulkan
   スワップチェーン+実キーボード入力(winit+ash-window)。
   `directx-graphics-vulkan`のオフスクリーンコンテキストとは別に、
   独自に独立したVulkanインスタンス/デバイスを保持する——両方を使う
   場合は同期させておくこと(例えばアルファブレンドは現状オフスクリーン
   経路でのみ有効)。インタラクティブなBreakout風デモを作り、ユーザー
   自身の目視(パドルの動き+ボールの跳ね返り)で動作確認済み。
5. **パス依存の慣習**: このクレートは`open-cuda`の`opencuda-core`/
   `opencuda-vulkan`へ相対パス依存で依存しており、このエコシステムの
   他所(`aruaru-llm`、`aruaru-db`等)で使われている「`F:\runo`配下の
   兄弟リポジトリ」慣習より1階層深い——これらは**devdependenciesのみ**で、
   公開されるライブラリ自体は`open-cuda`への依存を持たない。
6. **カーネルレベルのアンチチートに関するスコープの注記**: このプロジェクト
   を「実際のWindowsゲームをLinux上で動かす」というゴールへ向けて移植
   しようとする人向け——カーネルモードのアンチチート(Riot Vanguard、
   カーネルモードBattlEye等)は、このシェーダー翻訳層がどれだけ完成度を
   高めても、それとは無関係に設計上Linux/Proton系の環境をブロックする。
   これは修正すべき欠陥ではなく、そうしたアンチチートを使うタイトルは
   翻訳の完成度に関わらずこのプロジェクトの手の届かない対象である。

## まだ再利用できないもの(正直なギャップ)

- 汎用SM5.0またはSM6.0の命令デコーダは無い——上記に記載した特定の
  オペコード/形状クラスのみが対応しており、それ以外は誤翻訳ではなく
  正直に拒否される。
- D3D12の上位レイヤー(コマンドリスト、ディスクリプタヒープ、
  ルートシグネチャ)は全く未実装。
- 深度バッファは無く、グラデーション三角形のケースを超えた
  異なる頂点色間の補間チェックも無い。またAMD/Intel GPUハードウェア、
  ネイティブmacOS/Linuxデスクトップでの実行は未検証のまま(AMD/Intelの
  PCIベンダーID検出のコードパスは存在するが、この開発環境では実AMD/
  Intelハードウェアに対して一度も実行されていない)。

---

完全な技術詳細(正確なオペコード列、バイトレベルのDXILビットストリーム
トレース、コードスニペット、完全なパス依存Cargo.toml例)については、
原文の`PORTING.md`(英語、authoritative)を参照。
