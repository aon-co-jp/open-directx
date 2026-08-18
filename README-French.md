# open-directx

> 📌 **Mise à jour récente (2026-08-08, prototype de rendu de sprites 2D)** :
> À la suite de la proposition de l'utilisateur de développer des
> prototypes de jeu/minage/LLM pour la GT 730 via open-directx sur
> dream-os/Linux, le travail a commencé de manière ciblée avec un
> prototype de rendu de sprites 2D : d'abord le support de
> l'échantillonnage de texture, puis le support multi-sprites/feuille
> de sprites, une boucle de jeu (mise à jour de position + physique de
> rebond), une **véritable fenêtre + une véritable chaîne d'échange
> (swapchain) Vulkan + une véritable entrée clavier** (nouvelle crate
> `directx-graphics-window` — l'utilisateur l'a exécutée lui-même et a
> confirmé « la raquette a bougé et a renvoyé la balle »), le blending
> alpha, et le chargement de textures depuis de vrais fichiers PNG.
> Tout a été vérifié sur du matériel Windows réel (NVIDIA GT 730), et
> une partie également sur du matériel Linux réel (WSL2 Ubuntu).
> Prochains candidats : plusieurs sprites en mouvement avec détection
> de collision, support du redimensionnement de fenêtre. Voir
> [CLAUDE.md](CLAUDE.md) pour les détails.
>
> *日本語*: ユーザー提案「dream-os/Linux上でopen-directx経由でGT730の
> PCでGAME…の試作品を開発」を受け、まず2Dスプライト描画に絞って着手。
> テクスチャサンプリング(`Texture2D.Sample`)初実装→複数スプライト/
> スプライトシート対応→ゲームループ(位置更新+跳ね返り物理)→**実
> ウィンドウ+実Vulkanスワップチェーン+実キーボード入力**
> (`directx-graphics-window`クレート新設、ユーザー自身が実行し
> 「パドルが動いて玉を弾き返した」と目視確認済み)→アルファブレンド
> (半透明スプライト)→実PNGファイルからのテクスチャ読み込み、という
> 一連の増分をすべてWindows実機(NVIDIA GT 730)、一部はLinux実機
> (WSL2 Ubuntu)でも検証した。次の候補: 複数の動くスプライト+衝突判定、
> ウィンドウリサイズ対応。詳細は[CLAUDE.md](CLAUDE.md)参照。

> 📌 Tâche en attente (2026-08-06) : il existe un projet d'intégration
> des technologies Toshiba SBM et DeepSeek (concernant dream-os et 8
> dépôts au total). Voir [CLAUDE.md](CLAUDE.md) pour les détails.

> 📌 **Mise à jour récente (2026-08-08)** : concernant la chaîne à 7
> termes avec vérification des limites, l'asymétrie où seul le côté
> DXBC existait (sans équivalent DXIL) a été résolue — un nouveau
> fichier `vector_add_mul_div_sub_add_mul_div_chain7_bounded_dxil.hlsl`
> a été réellement compilé avec `dxc.exe`, et la correspondance
> numérique avec l'implémentation de référence CPU ainsi que le bon
> fonctionnement de la vérification des limites ont été confirmés sur
> du matériel réel NVIDIA GT730 (l'ensemble de l'espace de travail :
> 50 tests unitaires + 22 tests sur matériel réel, tous verts, zéro
> avertissement). Voir [CLAUDE.md](CLAUDE.md) pour les détails.
>
> *日本語*: 境界チェック付き7項チェーンについてDXBC側のみでDXIL側が
> 無いという非対称を解消——`vector_add_mul_div_sub_add_mul_div_
> chain7_bounded_dxil.hlsl`を新規に`dxc.exe`で実コンパイルし、NVIDIA
> GT730実機でCPU参照実装との数値一致・境界チェックの動作を確認済み
> (ワークスペース全体50単体テスト+実機テスト22本すべてgreen、警告0件)。
> 詳細は[CLAUDE.md](CLAUDE.md)参照。

> 📌 **Mise à jour récente (2026-08-07)** : la chaîne DXBC/DXIL avec
> vérification des limites a été étendue à 6 termes, vérifiée sur du
> matériel réel NVIDIA GT730. Une intégration renforcée avec dream-os/
> open-cuda/aruaru-llm (transplantation SBM/DeepSeek, etc.) a été
> envisagée, mais il a été décidé de ne pas étendre la logique
> existante de génération de chaînes DXBC/DXIL sans en avoir une
> compréhension approfondie, en raison du risque de laisser passer des
> erreurs numériques — aucun changement de code n'a été effectué, et
> les résultats de l'investigation ont été honnêtement consignés dans
> [CLAUDE.md](CLAUDE.md).
>
> *日本語*: 境界チェック付きDXBC/DXILチェーンを6項へ拡張し、NVIDIA
> GT730実機で動作確認。dream-os/open-cuda/aruaru-llmとの連携強化
> (SBM/DeepSeek移植等)を検討したが、既存のDXBC/DXILチェーン生成
> ロジックへの深い理解を伴わない拡張は数値的な誤りを見逃すリスクが
> あると判断し、コード変更は行わず調査結果を[CLAUDE.md](CLAUDE.md)へ
> 正直に記録した。

> **Mise à jour 2026-07-25** : le titre du fichier de politique de
> développement (`CLAUDE.md`) a été renommé de « Politique de
> développement et règles d'environnement de développement » à
> « Philosophie de conception et politique de développement et règles
> d'environnement de développement », afin de mieux distinguer la
> philosophie de conception du projet (ce que nous valorisons), la
> politique de développement (comment nous travaillons), et les règles
> d'environnement de développement (conventions opérationnelles
> concrètes). Voir `CLAUDE.md` pour les détails.


Une couche de compatibilité DirectX (D3D9/10/11/12) multiplateforme —
dans l'esprit de DXVK / vkd3d-proton — visant à faire fonctionner sans
modification des applications DirectX Windows sur Linux (et
éventuellement Android/macOS) en traduisant le bytecode de shaders
DXBC/DXIL vers SPIR-V et en le dispatchant via un backend de calcul
Vulkan existant (le `opencuda-vulkan` d'[open-cuda](https://github.com/aon-co-jp/open-cuda)).

Voir [`CLAUDE.md`](CLAUDE.md) pour la justification complète de la
conception, la portée/feuille de route honnête, et le journal HANDOFF
des sessions — ce README ne résume que l'état actuel et vérifié.

### Matrice de support des plateformes et fournisseurs (ajoutée le 2026-07-27, divulgation honnête)

DirectX lui-même est une API réservée à Windows/Xbox — « multiplateforme »
signifie ici que le bytecode DXBC/DXIL est traduit en SPIR-V et
dispatché via Vulkan, ce qui est ce qui atteint réellement les
plateformes non-Windows. Aucun `cfg(windows)` ni autre verrouillage par
plateforme n'existe aujourd'hui dans le code propre de ce dépôt (le
parseur DXBC, la génération de code SPIR-V et `directx-graphics-vulkan`
sont tous du Rust pur et neutre vis-à-vis de la plateforme + `ash`),
donc la portabilité build/test suit la portée de Vulkan lui-même :

| Plateforme | Chemin | Statut |
|---|---|---|
| Windows | Vulkan natif | **Vérifié sur du matériel réel** (la machine de développement de ce dépôt, NVIDIA GeForce GT 730) |
| Linux | Vulkan natif | Devrait se compiler/s'exécuter sans modification (aucun code spécifique à Windows n'existe pour le bloquer) — **pas encore testé sur une véritable machine Linux dans cet environnement** |
| Android | Vulkan natif | `open-cuda` a vérifié que la compilation croisée `aarch64-linux-android` réussit (selon son CLAUDE.md) ; l'exécution sur un appareil réel (`vkCreateInstance` sur un vrai téléphone) reste en attente |
| macOS | Vulkan via [MoltenVK](https://github.com/KhronosGroup/MoltenVK) (traduit vers Metal) | Pas encore tenté — MoltenVK est une couche de traduction, pas du Vulkan natif, donc c'est une garantie plus faible que Linux/Android |
| iOS / iPadOS (ajouté le 2026-08-17) | Vulkan via MoltenVK (traduit vers Metal) | Pas encore tenté. **La même réserve MoltenVK que macOS s'applique** — Vulkan ne fonctionne pas nativement sur iOS/iPadOS, uniquement via cette couche de traduction, donc la parité avec le chemin Windows/Vulkan natif n'est pas garantie tant que cela n'a pas été réellement essayé sur un appareil. Nécessite également le programme Apple Developer pour une distribution officielle. |
| Divers UNIX/BSD (ajouté le 2026-08-17) | Vulkan natif, probablement | Non recherché — le support Vulkan varie selon la distribution/le pilote ; devrait pouvoir réutiliser la majeure partie du chemin Linux une fois étudié |
| Sony PlayStation 4/5/6/7 | n/a | Explicitement hors périmètre pour l'instant — voir la note « Cibles de la famille PlayStation » ci-dessous et `CLAUDE.md` |
| Nintendo Switch 2 / 3 (ajouté le 2026-08-17) | n/a | Même statut « ambition future, différé en attente d'un SDK/NDA officiel » que PlayStation. **La Switch 3 n'a pas été officiellement annoncée par Nintendo à la date du 2026-08-17 — son inclusion ici n'est qu'un espace réservé au cas où elle serait annoncée, et non basée sur des informations produit réelles.** |

Couverture des fournisseurs GPU (correspondance par ID fournisseur PCI,
cohérente entre ce dépôt et `open-cuda` : NVIDIA `0x10DE`, AMD
`0x1002`/`0x1022`, Intel `0x8086`) :

| Fournisseur | Statut |
|---|---|
| NVIDIA | **Vérifié sur du matériel réel** (GeForce GT 730) |
| AMD | Le code de correspondance d'ID fournisseur existe et compile, mais **n'a jamais été exécuté sur du matériel AMD réel** dans cet environnement — à considérer comme non vérifié |
| Intel | Identique à AMD : le code existe, **jamais vérifié sur du matériel GPU Intel réel** |

Aucune correction n'est nécessaire pour rendre ces trois ID fournisseurs
*détectables* — le code est déjà correct et identique entre
`open-directx`/`opencuda-vulkan`/`opencuda-directx`. Ce qui manque, c'est
du véritable matériel AMD/Intel pour réellement exercer ce chemin de
code, ce que cet environnement de développement ne possède pas.

## État actuel (2026-07-27, dernier point : interpolation de gradient, diagnostics du fournisseur GPU, chaîne sub/div)

Trois incréments ont été ajoutés par-dessus le pipeline graphique
minimal D3D11 et le travail sur les classes de chaîne DXBC ci-dessous,
tous vérifiés sur la véritable NVIDIA GT 730 de cette machine : (1)
`render_gradient_triangle_and_read_back` — le pipeline graphique peut
désormais assigner une couleur distincte par sommet (pas seulement le
cas dégénéré de couleur uniforme), vérifié via un contrôle d'invariant
de partition de l'unité sur des pixels de relecture matériels réels.
(2) `enumerate_graphics_devices()` — comble un écart de parité de
diagnostic où le chemin Compute d'`open-cuda` avait une détection
d'ID fournisseur mais le chemin Graphics ici n'en avait aucune ;
autonome, sans nouvelle dépendance à `opencuda-vulkan`. (3)
`decode_chain_shape` prend désormais en charge `sub`/`div`
(auparavant explicitement rejetés comme invérifiables) — un nouveau
shader (`vector_sub_div_chain.hlsl`) a été réellement compilé avec
`fxc.exe` et son dump SHEX utilisé pour confirmer l'ordre exact des
opérandes, puis vérifié de bout en bout par rapport à une référence
CPU sur du matériel réel. Voir le HANDOFF de `CLAUDE.md` (entrées du
2026-07-27) pour le récit complet.

## État actuel (2026-07-25, dernier point : tranche verticale DXIL complète sur matériel réel)

La tranche verticale du shader de calcul D3D12/DXIL atteint désormais
la parité complète avec celle de D3D11/DXBC : `vector_add.dxil` (sortie
réelle de `dxc.exe -T cs_6_0`) est décodé de bout en bout (conteneur ->
bitstream LLVM -> table de types -> instructions -> les 7
enregistrements `Call` désambiguïsés vers leur véritable signification
`dx.op.*`) et traduit en SPIR-V réel
(`directx_shader_translate::translate_dxil_vector_add_to_spirv`), que
`tests/vector_add_dxil_real_vulkan.rs` dispatche sur la véritable
NVIDIA GT 730 de cette machine et vérifie qu'il correspond numériquement
à la référence CPU `a[i]+b[i]`. Ceci reste une seule forme de shader
connue, pas un décodeur SM6.0 général — voir « Non implémenté (portée
honnête) » ci-dessous pour la limite précise. La taille du groupe de
travail SPIR-V est désormais véritablement extraite du `METADATA_BLOCK`
de DXIL (`dx.entryPoints` -> `ShaderProperties` -> `NumThreads`), et non
codée en dur — voir l'entrée HANDOFF du 2026-07-25 « suite 9 » dans
`CLAUDE.md` pour le récit complet, et « suite 7 » pour la réalisation
originale de la tranche verticale que cela a permis de compléter.

## État actuel (2026-07-25, suite : analyse DXIL au niveau du bitstream + analyse DXBC des VS/PS D3D11)

Deux nouveaux éléments se sont ajoutés à la tranche verticale du shader
de calcul de Phase 1 ci-dessous :

- **DXIL (D3D12/SM6+) — octets réels analysés, uniquement au niveau
  conteneur/bitstream.** `crates/directx-shader-translate/src/dxil.rs`
  (`parse_dxil_container`) analyse un véritable conteneur DXBC compilé
  par `dxc.exe -T cs_6_0` (`shaders/vector_add_dxil.hlsl` ->
  `shaders/vector_add.dxil`, produit par
  `tools/compile-dxbc-shaders.ps1`) : extrait le
  `DxilProgramHeader`/`DxilBitcodeHeader` du chunk `DXIL` (type de
  shader, SM6.0, version DXIL) via la crate `dxbc` existante, puis
  transmet le payload de bytecode LLVM brut à la crate `llvm-bitcode`
  (nouvelle dépendance ajoutée, lecteur générique de bitstream LLVM
  sans connaissance spécifique de DXIL) pour réellement décoder l'arbre
  de blocs/enregistrements. Confirmé par rapport aux octets réels : le
  magic wrapper LLVM `BC\xC0\xDE`, un unique `MODULE_BLOCK` de premier
  niveau (id 8), et les sous-blocs LLVM standards à l'intérieur —
  `TYPE_BLOCK_ID_NEW`(17), `PARAMATTR_GROUP_BLOCK`(10),
  `PARAMATTR_BLOCK`(9), `CONSTANTS_BLOCK`(11), `FUNCTION_BLOCK`(12, x5 —
  un par bloc de base de `main`), `VALUE_SYMTAB_BLOCK`(14),
  `METADATA_BLOCK`(15, x2). **Mise à jour (2026-07-25, suite, piste
  D3D12)** : la résolution de la table de types et le décodage
  grossier des instructions ont depuis été ajoutés
  (`resolve_type_table`/`decode_function_instructions` dans le même
  fichier), appliquant les tables d'enregistrements documentées de
  LLVM `TYPE_BLOCK`/`FUNC_CODE` aux octets réels de `vector_add.dxil`
  — confirmé une table de types à 22 entrées incluant `Float` et
  `StructNamed{"class.RWStructuredBuffer<float>"}`, et une séquence
  d'instructions réelle (`DeclareBlocks -> Call*5 -> ExtractValue ->
  Call -> ExtractValue -> BinOp -> Call -> Ret`). **Mise à jour
  (2026-07-25, suite 6)** : les 7 enregistrements `Call` sont désormais
  désambiguïsés. `resolve_vector_add_dxil_calls` résout les noms de
  fonctions du `VALUE_SYMTAB_BLOCK` (trouvés via `Record::take_payload()`,
  pas `fields()` — un véritable écart dans la compréhension de la crate
  de l'entrée précédente) et décode manuellement l'encodage d'opérande
  de valeur relative de LLVM (vérifié manuellement par rapport aux
  octets réels), donnant `[CreateHandle{range_id:2},
  CreateHandle{range_id:1}, CreateHandle{range_id:0}, ThreadId,
  BufferLoad{handle_range_id:0}, BufferLoad{handle_range_id:1},
  BufferStore{handle_range_id:2}]`. Les numéros d'opcode DXIL
  (`CreateHandle`=57, `BufferLoad`=68, `BufferStore`=69, `ThreadId`=93)
  ont été confirmés via une recherche web par rapport au
  `DirectXShaderCompiler/docs/DXIL.rst` de Microsoft, et non supposés
  de mémoire, et correspondaient exactement aux constantes décodées
  réelles. **Toujours aucune traduction DXIL-vers-SPIR-V** — c'est le
  prochain incrément. Voir « Non implémenté » ci-dessous.
- **Pipeline graphique D3D11 — génération SPIR-V réelle pour VS/PS
  atteinte et validée, pas encore de rasterizer/draw.**
  `shaders/triangle_vs.hlsl`/`shaders/triangle_ps.hlsl` (paire minimale
  de shaders vertex+pixel en pass-through, `POSITION`/`COLOR` en
  entrée, `SV_POSITION`/`SV_TARGET` en sortie) compilés avec les
  véritables `fxc.exe /T vs_5_0`/`/T ps_5_0`. `parse_dxbc` analyse les
  deux sans modification. `spirv_gen::translate_vertex_shader`/
  `translate_pixel_shader` (nouveau) décodent la séquence d'opcodes
  SHEX réelle et fixe (`dcl_input`x2/`dcl_output_siv`/`dcl_output`/
  `mov`x3/`ret` pour VS ; `dcl_input_ps`(linear)/`dcl_output`/`mov`/
  `ret` pour PS) et émettent du véritable SPIR-V graphique :
  `OpEntryPoint Vertex`/`Fragment` (pas `GLCompute`), des variables de
  classe de stockage `Input`/`Output` avec des décorations `Location`,
  `BuiltIn Position` sur la sortie `SV_POSITION` du vertex shader, et
  `OpExecutionMode ... OriginUpperLeft` sur le fragment shader. Validé
  de deux manières : (1) le propre chargeur de `rspirv` reparse les
  octets émis sans erreur, (2) le véritable `spirv-val.exe` du Vulkan
  SDK (`C:\VulkanSDK\1.4.350.0\Bin\spirv-val.exe`) a été exécuté sur
  les deux modules émis et a retourné le code de sortie 0 sans
  diagnostic pour les deux. `translate_shader`/`translate_chain_shader`
  (compute uniquement) rejettent toujours correctement les deux
  shaders. **Aucun rasterizer, aucun framebuffer, aucun appel de
  dessin Vulkan réel n'existe** — il a été confirmé (en lisant son
  code source réel) qu'`opencuda-vulkan` n'a aucun code
  `VkGraphicsPipelineCreateInfo`/render-pass/framebuffer, uniquement
  du dispatch de calcul, donc un pixel réellement rendu est hors
  périmètre pour cette passe. Voir le HANDOFF de `CLAUDE.md` pour la
  limite honnête du jalon.

## État actuel (2026-07-26, jalon du pipeline graphique minimal D3D11 atteint)

La nouvelle crate `crates/directx-graphics-vulkan` ajoute `ash` comme
dépendance **directe** de cet espace de travail (pas superposée sur
`opencuda-vulkan`, dont l'audit de source a confirmé qu'il ne fait que
du dispatch de calcul). Elle implémente une véritable render pass, un
framebuffer, et un `VkGraphicsPipelineCreateInfo`, réutilisant le
SPIR-V déjà généré et déjà validé par `spirv-val` provenant de
`translate_vertex_shader`/`translate_pixel_shader` ci-dessus (aucune
traduction de shader n'est réimplémentée). `render_uniform_triangle_
and_read_back` dessine un « grand triangle » couvrant tout le viewport
avec une seule couleur de sommet uniforme, relit l'image rendue via un
buffer de staging visible par l'hôte, et le test sur matériel réel
(`crates/directx-graphics-vulkan/tests/triangle_real_vulkan.rs`) vérifie
que chaque pixel relu correspond à la couleur de sommet pass-through
sur la véritable NVIDIA GT 730 présente sur cette machine (`cargo test
-p directx-graphics-vulkan --test triangle_real_vulkan -- --nocapture` :
1 réussi). La portée est intentionnellement étroite : une seule paire
de shaders fixe, un seul appel de dessin, pas de vérification de
buffer de profondeur/textures/swapchain/interpolation multi-triangle.
Voir le HANDOFF de `CLAUDE.md` (suite du 2026-07-26) pour la
divulgation honnête complète.

## État actuel (2026-07-25, tranche verticale de Phase 1 généralisée à 3 shaders connus)

`crates/directx-shader-translate` effectue désormais la tranche
verticale complète pour **trois shaders spécifiques connus**
(`vector_add.hlsl`, `vector_mul.hlsl`, `vector_sub_bounded.hlsl`) :
analyse DXBC -> décodage étroit du sous-ensemble d'opcodes SM5.0 ->
génération de code SPIR-V (via `rspirv`) -> dispatch Vulkan réel
(`opencuda-vulkan` d'`open-cuda`) -> correspondance numérique avec la
référence CPU, vérifiée sur la véritable NVIDIA GT 730 de cette
machine. **Ceci n'est toujours pas un décodeur SM5.0-vers-SPIR-V
général** — voir « Non implémenté » ci-dessous.

- `parse_dxbc` (Phase 0) : introspection de conteneur/chunk DXBC
  (présence RDEF/ISGN/OSGN/SHEX), inchangé par rapport au front-end
  original.
- `spirv_gen::translate_shader` (Phase 1, généralisé le 2026-07-25) :
  reconnaît 3 formes d'opcodes réellement émises par `fxc.exe`, toutes
  partageant un squelette commun (`dcl_globalFlags` -> optionnel
  `dcl_constantbuffer` -> 3x `dcl_uav_structured` -> `dcl_input` ->
  `dcl_temps` -> `dcl_thread_group` -> optionnel `ult`+`if` -> 2x
  `ld_structured` -> `add`/`mul` -> `store_structured` -> optionnel
  `endif` -> `ret`) :
  - `vector_add.hlsl` : `add`, sans vérification des limites.
  - `vector_mul.hlsl` : `mul` au lieu de `add`.
  - `vector_sub_bounded.hlsl` : `add` avec un drapeau `negate` sur son
    premier opérande source (confirmé en dumpant la sortie réelle de
    `fxc.exe` — `fxc` optimise `a - b` en `add dest, -b, a` plutôt que
    d'émettre un opcode `sub` dédié), plus une véritable vérification
    des limites `if (id.x < N)` (`ult` par rapport à un buffer de
    constantes + `if`/`endif`), que le SPIR-V émis implémente avec un
    véritable `OpSelectionMerge`/`OpBranchConditional`, utilisant la
    constante push `n` pour la comparaison.
  Tout autre forme d'opcode/opérande est rejetée via
  `SpirvGenError::UnsupportedShader` plutôt que mal traduite en
  silence. Les points de liaison UAV, la taille du groupe de threads,
  l'opérateur et la présence de vérification des limites sont tous
  extraits du DXBC réellement analysé, pas codés en dur.
  `translate_vector_add_shader` est conservé comme un alias mince
  rétrocompatible pour `translate_shader`.
- `tests/vector_add_real_vulkan.rs`, `tests/vector_mul_real_vulkan.rs`,
  `tests/vector_sub_bounded_real_vulkan.rs` : chacun dispatche son
  SPIR-V traduit via le véritable `opencuda-vulkan::VulkanDevice`
  d'`open-cuda` (`ash`, feature `real-vulkan`) et vérifie la sortie GPU
  par rapport à une référence CPU pour 256 éléments (epsilon 1e-3/1e-2).
  Le test de vérification des limites dispatche en plus 320 threads
  avec un nombre logique d'éléments de 256 et vérifie que les éléments
  256..320 ne sont jamais écrits (restent à une valeur sentinelle),
  prouvant que la branche `if (id.x < N)` dans le SPIR-V généré
  contrôle réellement l'exécution plutôt que de simplement compiler.
- `examples/dump_shex.rs` : un petit outil autonome
  (`cargo run -p directx-shader-translate --example dump_shex -- <path.dxbc>`)
  utilisé pendant cette session pour inspecter des flux d'opcodes SHEX
  réels avant d'écrire le support du décodeur pour eux ; conservé pour
  un futur travail de généralisation opcode par opcode.

**Depuis que le titre de cette section a été écrit**, un 4ème shader à
opération unique (`vector_div.hlsl`, simple `div`) a été ajouté à
`translate_shader` en suivant exactement le même schéma, et — plus
récemment — une classe de motif véritablement différente,
`spirv_gen::translate_chain_shader`, a été ajoutée à côté (sans le
remplacer) : elle décode un véritable arbre d'expression de registres
d'opérations binaires séquentielles (add/mul, sans flux de contrôle) au
lieu d'une seule opération fixe, vérifié par rapport à un shader
nouvellement compilé dont le SHEX réel s'est avéré réutiliser les
composantes d'un seul registre temporaire via l'élimination des
sous-expressions communes (CSE) de fxc plutôt que de déclarer des
registres temporaires supplémentaires. Voir l'entrée HANDOFF du
2026-07-25 « suite 9 » dans `CLAUDE.md` pour le récit complet et
actuel (cette section est laissée telle qu'elle a été écrite
originellement pour une exactitude historique concernant l'état de la
mi-journée du 2026-07-25).

## Build & test

```powershell
cargo build --workspace
cargo test --workspace -- --nocapture
```

### Voir quelque chose réellement se dessiner (ajouté le 2026-07-27)

Ce dépôt est un ensemble de bibliothèques sans `fn main` propre, donc
le moyen le plus rapide de *voir* le pipeline graphique fonctionner sur
votre propre GPU — plutôt que de lire le code source des tests — est :

```bash
cargo run -p directx-graphics-vulkan --example render_triangle
```

Ceci réutilise les mêmes shaders traduits DXBC → SPIR-V compilés avec
le véritable fxc.exe que `tests/triangle_real_vulkan.rs`, dessine un
triangle en dégradé (rouge/vert/bleu) sur du véritable matériel
Vulkan, relit le framebuffer, et l'écrit dans `render_triangle.ppm`
(PPM simple, aucune dépendance supplémentaire à une crate d'image
nécessaire — convertissez-le avec par exemple
`magick render_triangle.ppm render_triangle.png` ou ouvrez-le
directement dans la plupart des visionneuses d'images). Si aucun
périphérique/pilote Vulkan utilisable n'est présent, il affiche une
erreur honnête et se termine avec un code non nul plutôt que de
simuler un succès.

Sortie réellement observée (2026-07-25, cette machine, NVIDIA GeForce
GT 730) :

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

`cargo clippy --workspace --all-targets` : 0 avertissement.

Après le travail de décodage de table de types/instructions DXIL
(2026-07-25, suite, piste D3D12), `cargo test --workspace` exécute 23
tests au total (19 unitaires + 4 tests d'intégration Vulkan réels),
tous réussis, dont 3 nouveaux par rapport aux 20 précédents :
`dxil::tests::resolves_real_dxil_type_table_and_finds_float_and_
resource_struct`, `dxil::tests::decodes_real_dxil_function_block_into_
matching_vector_add_shape`, et `dxil::tests::shape_matcher_honestly_
rejects_unexpected_instruction_orderings`.

Pour régénérer les fixtures DXBC à partir de HLSL (nécessite le
`fxc.exe` du Windows SDK — noter que `dxc.exe` cible uniquement
DXIL/SM6+ et ne peut pas produire de DXBC) :

```powershell
pwsh tools/compile-dxbc-shaders.ps1
```

## Non implémenté (portée honnête)

- **Décodage général des instructions SM5.0.** Seules les 3 formes
  d'opcodes ci-dessus sont gérées ; tout autre shader de calcul D3D11
  (mise en page de ressources différente, autre flux de contrôle,
  autres intrinsèques, plus d'une vérification des limites,
  `div`/`sub` en tant que véritable opcode plutôt qu'`add` négatif,
  etc.) est rejeté, pas mal traduit. Construire un véritable décodeur
  général (ou adopter/porter un décodeur existant, par exemple étudier
  de plus près l'approche de `dxbc-spirv`/`dxil-spirv`) reste le
  véritable prochain jalon.
- **DXIL (Shader Model 6+, D3D12) : la tranche verticale de
  `vector_add.dxil` est désormais complète de bout en bout, sur du
  matériel réel — mais toujours uniquement pour cette seule forme de
  shader connue, pas SM6.0 en général.** `resolve_type_table`/
  `decode_function_instructions`/`resolve_vector_add_dxil_calls` dans
  `dxil.rs` décodent les véritables enregistrements
  `TYPE_BLOCK`/`FUNCTION_BLOCK`/`VALUE_SYMTAB_BLOCK` par rapport aux
  codes documentés de LLVM et désambiguïsent les 7 enregistrements
  `Call` vers leur véritable signification `dx.op.*`
  (`CreateHandle`/`ThreadId`/`BufferLoad`/`BufferStore`, avec les
  points de liaison UAV). `translate_dxil_vector_add_to_spirv`
  (nouveau) alimente cette sortie résolue dans l'`emit_spirv_for_
  kernel` partagé de `spirv_gen.rs` (extrait de l'`emit_spirv` du
  chemin DXBC pour que les deux backends émettent depuis un seul
  chemin de code) pour produire du véritable SPIR-V, que
  `tests/vector_add_dxil_real_vulkan.rs` dispatche sur la véritable
  NVIDIA GT 730 de cette machine via `opencuda-vulkan` et vérifie
  qu'il correspond à la référence CPU `a[i]+b[i]` pour les 256
  éléments — la même rigueur que le test DXBC `vector_add`. **La
  taille du groupe de travail est désormais réellement extraite, pas
  codée en dur** : `extract_numthreads_from_metadata` (`dxil.rs`)
  parcourt le véritable chemin `METADATA_BLOCK` `dx.entryPoints` ->
  tuple par point d'entrée -> `ShaderProperties` ->
  `kDxilNumThreadsTag` (=4, confirmé par rapport aux sources
  `DxilMetadataHelper.h`/`.cpp` du `DirectXShaderCompiler` de
  Microsoft) et résout le nœud `{x,y,z}` par rapport à la véritable
  liste de valeurs du module, donnant `(64,1,1)` à partir des octets
  réels de `vector_add.dxil` — le codage en dur connu de l'entrée
  précédente est clos, et un test de régression synthétique prouve que
  la logique d'extraction retourne une valeur *différente* lorsqu'on
  lui donne des métadonnées différentes (pas seulement « retourne
  64,1,1 quoi qu'il arrive »). Toute autre forme d'opcode/opérande
  (opération différente, blocs de base multiples, vérifications des
  limites) est toujours rejetée, pas mal traduite. Le support des
  listes de commandes/tas de descripteurs/signatures racine D3D12 (la
  couche au-dessus de la traduction de shaders) n'est pas touché.
- **Décodeur DXBC généralisé au-delà de 4 formes fixes à opération
  unique : gère désormais les chaînes d'opérations binaires
  séquentielles (sans flux de contrôle) via un véritable arbre
  d'expression de registres, pas une 5ème forme codée en dur.**
  `spirv_gen::translate_chain_shader`/`decode_chain_shape` parcourent
  `ld_structured`/`add`/`mul`/`store_structured` et construisent un
  véritable arbre d'expression clé par (registre temporaire,
  composante), afin de gérer 1 opération, 2 opérations, ou N
  opérations de la même manière — vérifié par rapport à un véritable
  shader nouvellement compilé (`vector_add_mul_chain.hlsl`,
  `t = A[i]+B[i]; Out[i] = t*A[i]`) dont le SHEX réel s'est avéré
  réutiliser les composantes `.x`/`.y` d'un seul registre temporaire
  (fxc a éliminé par CSE le chargement répété de `A[i]` au lieu de
  réémettre `ld_structured`) — une découverte réelle et non prédite
  que le décodeur basé sur arbre gère sans cas supplémentaire.
  Dispatché et vérifié sur la véritable NVIDIA GT 730 par rapport à la
  référence CPU `(a[i]+b[i])*a[i]`. `sub`/`div` à l'intérieur d'une
  chaîne sont intentionnellement toujours rejetés (leur sémantique
  d'ordre d'opérande n'a été vérifiée que pour le cas à opération
  unique). Les 4 formes originales à opération unique ne sont pas
  touchées et passent toujours sans modification.
- **Pipeline graphique D3D11 : analyse de conteneur DXBC confirmée
  fonctionnelle pour VS/PS, mais pas de génération de code SPIR-V, pas
  de rasterizer, pas de triangle réellement dessiné à l'écran.** Le
  pipeline complet (rasterizer, échantillonnage de texture, état de
  blending, output-merger) reste hors périmètre ; de même pour étendre
  le décodeur de formes d'opcodes étroit de `spirv_gen` afin de
  comprendre `dcl_output_siv`/`dcl_input_ps`/les modes d'interpolation.
- Cibles de la famille PlayStation — explicitement hors périmètre ;
  voir `CLAUDE.md` pour le raisonnement juridique/CGU.

## Projets liés

- [open-cuda](https://github.com/aon-co-jp/open-cuda) — le backend
  d'exécution de calcul Vulkan que ce projet est conçu pour dispatcher
  (`opencuda-core::GpuDevice`, `KernelSource::SpirV`). Contient aussi
  une crate `opencuda-directx` déjà fonctionnelle et sans rapport, qui
  exécute D3D12 **nativement sur Windows** — la direction opposée à
  celle de ce projet (qui exécute des shaders DirectX **sur des cibles
  non-Windows**).
- [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm) — aucune
  dépendance technique directe avec ce projet (vérifié, pas supposé).
