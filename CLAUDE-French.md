# Philosophie de conception et politique de développement et règles d'environnement de développement (open-directx) — condensé

> **Remarque** : Ceci est une traduction condensée de l'état actuel. Le
> journal HANDOFF historique complet (des dizaines d'entrées depuis le
> 2026-07-25) reste disponible uniquement en japonais dans CLAUDE.md —
> voir là-bas pour le détail session par session.

Disque de travail : `F:\runo`. Cette section suit la pratique consistant
à traiter le `CLAUDE.md` d'[`open-raid-z`](https://github.com/aon-co-jp/open-raid-z)
comme la source de vérité et à le copier dans chaque projet. Dépôt
GitHub : [aon-co-jp/open-directx](https://github.com/aon-co-jp/open-directx).

**Date de début du développement : 2026-07-25** (le dépôt GitHub vide
lui-même avait été créé le 2026-07-01).

## Tâche en attente (ajoutée le 2026-08-06, non commencée)

Il existe un projet, sur instruction de l'utilisateur, d'étudier
l'intégration de la technologie Simulated Bifurcation Machine (calcul
pseudo-quantique) de Toshiba et des techniques de DeepSeek (MLA,
DeepSeekMoE, entraînement en précision mixte FP8, recherchées à partir
d'articles et de blogs d'implémentation, pas seulement des actualités)
dans 8 dépôts dont `open-directx`. Aucune cible d'optimisation concrète
n'a encore été identifiée pour ce dépôt — l'investigation est reportée
à une session future.

## Rôle de ce projet

Une couche de compatibilité DirectX (D3D9/10/11/12) multiplateforme
visant à faire fonctionner des applications/jeux existants écrits pour
l'API DirectX réservée à Windows sur Linux, Android, et éventuellement
macOS et la famille PlayStation.

Le 2026-07-25, par choix explicite de l'utilisateur, le projet s'est
engagé à poursuivre une **véritable couche de compatibilité en
direction inverse** (faire fonctionner des binaires/shaders DirectX
Windows non modifiés sur d'autres systèmes d'exploitation) plutôt que
l'alternative antérieure consistant à « exposer une API de type
DirectX au-dessus de Vulkan comme base commune ».

## Correction du positionnement technique (2026-07-25, important)

Une évaluation antérieure (2026-07-23) jugeait que DXVK/vkd3d-proton/
MoltenVK ne traduisent que dans un seul sens (DirectX → Vulkan/Metal)
sans exemple réel de la direction inverse, et étaient donc un mauvais
choix technique. Ceci était une **confusion entre deux axes différents**
et a été corrigé :

- DXVK/vkd3d-proton (la technologie derrière Proton de Valve /
  compatibilité des jeux DirectX Linux Steam) et CrossOver/Whisky
  (macOS) basés sur MoltenVK sont des exemples réels et fonctionnels
  exactement de la compatibilité inverse que l'utilisateur souhaite :
  faire fonctionner de véritables binaires/jeux DirectX existants
  (API réservée à Windows) sans modification sur Linux/macOS.
- La **direction de la traduction** (« les appels DirectX traduits en
  appels Vulkan ») et la **direction de l'expérience utilisateur
  final** (« une application DirectX ciblant Windows fonctionne-t-elle
  réellement sur Linux/macOS ») sont des axes séparés — le fait que le
  premier vise Vulkan n'empêche pas le second d'atteindre correctement
  « DirectX fonctionnant sur un autre système d'exploitation ».
- Par conséquent, utiliser Vulkan comme backend d'exécution interne
  n'est pas en conflit avec la poursuite d'une véritable couche de
  compatibilité en direction inverse — DXVK etc. en sont exactement le
  précédent. Ce projet adopte la même approche (interception des
  appels API D3D + traduction à l'exécution du bytecode de shaders
  DXBC/DXIL → exécution via Vulkan).

## Portée et feuille de route honnête

**Phase 0 (actuelle, étape de conception/recherche)** :
- Recherche de la structure des formats DXBC/DXIL (formats de bytecode
  de shaders DirectX).
- Étude de l'architecture des implémentations OSS existantes (DXVK,
  vkd3d-proton, [dxil-spirv](https://github.com/HansKristian-Work/dxil-spirv)
  — l'outil de conversion DXIL→SPIR-V réellement utilisé par
  vkd3d-proton — SPIRV-Cross, naga) afin d'éviter de réinventer la
  roue et d'emprunter des décisions de conception éprouvées.
- Découpage d'une portée MVP réaliste : **un pipeline graphique
  complet (rasterizer, échantillonneur de texture, état de blending,
  etc.) est hors périmètre pour l'instant.** Commencer par une tranche
  verticale ne couvrant que le dispatch de D3D11 Compute Shader
  (DirectCompute) — un shader de calcul simple réellement traduit de
  DXBC/DXIL vers SPIR-V, dispatché via `opencuda-vulkan` d'`open-cuda`,
  et vérifié comme correspondant numériquement à une implémentation
  de référence CPU. Le travail sur le pipeline graphique commence
  comme phase suivante seulement une fois cette tranche verticale
  prouvée.

**Phase 1 et suivantes (non commencées)** :
1. Tranche verticale D3D11 Compute Shader (traduction DXBC/DXIL→SPIR-V
   + dispatch Vulkan).
2. Pipeline graphique minimal D3D11 (shaders vertex/pixel +
   rasterisation de base).
3. Support D3D12 (listes de commandes, tas de descripteurs, signatures
   racine).
4. Support Android (Vulkan lui-même étant natif sur Android, la
   majeure partie des ressources de la version Linux devrait être
   réutilisable — mais une couche d'émulation Win32/COM, équivalente
   à Wine, sera probablement nécessaire ; envisager une collaboration/
   réutilisation avec le projet Wine lui-même dans ce cas).
5. Support macOS / iPhone / iPad (via MoltenVK, même approche que
   CrossOver/Whisky ; iPhone/iPad nécessite le programme Apple
   Developer pour une distribution officielle — l'exécution native sur
   du matériel non officiel est impossible, la même contrainte
   identifiée par la recherche de `dream-os`).
6. Divers systèmes de la famille UNIX (BSD, etc.) — probablement
   capables de réutiliser la majeure partie du chemin Linux selon le
   support Vulkan (pas encore étudié).

**Concernant le support PlayStation 4/5/6/7 (divulgation honnête, au
2026-07-25)** : inclus dans la vision originale de l'utilisateur, mais
**des préoccupations juridiques/CGU existent indépendamment de la
difficulté technique** — les SDK de développement PlayStation sont
privés et couverts par des NDA, et la rétro-ingénierie non officielle
risque d'enfreindre diverses CGU et lois (par exemple le DMCA). Ce
projet ne note le support PS4-7 dans la feuille de route que comme une
**« ambition future »** et ne l'inclut pas actuellement dans la portée
de conception/implémentation. Le démarrer nécessiterait une évaluation
juridique du risque distincte et une confirmation renouvelée de
l'utilisateur.

**Concernant le support Nintendo Switch 2/3 (ajouté le 2026-08-17,
divulgation honnête)** : Enregistré de manière similaire uniquement
comme une « ambition future ». La Switch 2 nécessite le matériel de
développement officiel/NDA de Nintendo (la même préoccupation
juridique que PS4-7). **La Switch 3 n'a pas été officiellement
annoncée par Nintendo à la date du 2026-08-17 — son inclusion ici
n'est qu'un espace réservé au cas où elle serait annoncée, et non
basée sur des informations produit réelles** (noté explicitement pour
éviter d'exagérer).

## Projets de base (sur instruction de l'utilisateur, 2026-07-25)

- **[open-cuda](https://github.com/aon-co-jp/open-cuda)** : utilise
  `opencuda-vulkan` (backend d'exécution Vulkan Compute, vérifié sur
  du matériel réel NVIDIA GT 730) comme backend d'exécution de
  shaders. Prévoit de réutiliser l'abstraction `opencuda-core::
  GpuDevice` (alloc/memcpy/launch_kernel) telle quelle, en passant les
  kernels traduits DXBC/DXIL→SPIR-V comme `KernelSource::SpirV`
  (détails exacts de l'API à confirmer par rapport à `opencuda-core`).
  Distinct d'`opencuda-directx` (un backend D3D12 réservé à Windows,
  Phase 1&2 déjà implémentée) — celui-ci exécute DirectX nativement
  *sur* Windows, la direction opposée à ce projet (exécuter DirectX
  *sur d'autres systèmes d'exploitation*).
- **[aruaru-llm](https://github.com/aon-co-jp/aruaru-llm)** : aucune
  dépendance technique directe actuellement (aruaru-llm est un service
  d'inférence LLM ; ce projet est une couche de compatibilité d'API
  graphique). L'intention exacte derrière le fait que l'utilisateur
  liste aruaru-llm comme une « base » n'est pas confirmée —
  possiblement au sens de suivre son motif partagé de service
  « bunshin-no-jutsu » (clone/tenant) (par exemple appliquer son motif
  d'API de gestion de type `TenantRegistry` à une surface de gestion
  de ce projet, telle qu'un serveur de cache de traduction). À mettre
  à jour une fois que des points d'intégration concrets seront
  identifiés.

## Politique de développement (résumé à l'échelle de l'écosystème)

- Implémentation basée sur Rust. Utilise la crate `windows` (API
  Windows) et les bindings Vulkan (`ash`, etc., correspondant à ce
  qu'utilise `opencuda-vulkan`).
- « Ne jamais rapporter comme terminé sur la seule base du succès du
  type-check/build » — ne rapporter quelque chose comme fonctionnel
  qu'après avoir réellement traduit du véritable bytecode DXBC/DXIL,
  dispatché via du véritable Vulkan, et confirmé une correspondance
  numérique avec une implémentation de référence CPU sur du matériel
  réel (une discipline à l'échelle de l'écosystème).
- Les fonctionnalités non implémentées/stub ne doivent jamais signaler
  faussement « supporté » (en suivant le motif `supports_dxil()` d'
  `opencuda-directx`).
- Confirmer avec l'utilisateur avant toute décision impliquant de
  nouveaux fichiers, de nouvelles crates, de nouveaux dépôts, ou des
  jugements de nommage/placement (leçon du 2026-07-23, voir le
  CLAUDE.md d'`open-raid-z`).

## HANDOFF (entrées les plus récentes uniquement — voir CLAUDE.md pour le journal complet)

- **2026-08-20 : tranche verticale de compute shader pour un GEMM
  (2x2 fixe) implémentée et vérifiée sur matériel réel**, première
  étape vers l'accélération matérielle pour l'inférence LLM. Nouvelle
  fonction `translate_gemm2x2_shader` dans `directx-shader-translate` :
  un shader HLSL GEMM 2x2x2 a été réellement compilé avec `fxc.exe`,
  son DXBC décodé, traduit en SPIR-V, puis exécuté sur du Vulkan réel
  (NVIDIA GT 730) en réutilisant le contrat de kernel de
  `opencuda-vulkan`. La sortie GPU correspondait exactement à la
  référence CPU. Limite honnête : taille fixe 2x2 uniquement — le GEMM
  général à taille variable (avec boucles), les opérations d'Attention
  et le branchement vers `aruaru-llm` restent hors périmètre.
- **2026-08-19 : étude de faisabilité d'un mécanisme de mise à jour
  automatique (façon self-update GitHub Releases d'`open-english`) —
  reportée.** Le seul binaire `fn main` du workspace est
  `directx-graphics-window` (démo Breakout pour inspection visuelle),
  sans service persistant ni endpoint `/healthz` ; le motif ne
  s'applique donc pas.
- **2026-08-19 : examen de l'objection « l'absence de service
  persistant est un défaut » — conclusion : non.** Le vrai DirectX de
  Microsoft est distribué sous forme de DLL runtime liées dynamiquement
  par les applications, pas comme service d'arrière-plan indépendant ;
  la conception actuelle (bibliothèque + binaire de démo uniquement)
  correspond à l'architecture de DirectX original.
- **2026-08-20 : mémo d'idée `open-cg-cad` consigné (non commencé).**
  Concept d'un futur outil de modélisation 3D avec capture de
  mouvement, modification de spécifications en langage naturel pilotée
  par chat IA, et support DirectX/OpenGL/WebGL/WebGPU.
- **2026-08-20 (suite) : concept « immobilier IA × constructeur IA »
  consigné**, combinant `open-cg-cad` et `aruaru-llm` pour générer
  automatiquement des modèles 3D (maisons, immeubles, ponts, tunnels,
  etc.) à partir de données de terrain ; étendu ensuite aux trains à
  sustentation magnétique et aux modèles CAO de semi-conducteurs
  (CPU/GPU/NPU), avec une mise en garde honnête sur la difficulté de
  couvrir architecture, génie civil, matériel ferroviaire et conception
  de semi-conducteurs dans un seul système. Simple mémo d'idée, non
  commencé.

- **2026-08-08 (suite 12) : chargement de texture depuis un véritable
  fichier PNG implémenté, vérifié sur du matériel Windows + Linux
  réel.** Un nouveau module `png_loader.rs` (utilisant la crate `png`,
  série 0.17.x) implémente `load_png_rgba8`, normalisant les images
  RGB/RGBA/niveaux de gris/niveaux de gris+alpha/palette vers RGBA8
  (l'expansion de palette et la normalisation 16 bits→8 bits sont
  toutes deux déléguées aux propres transformations de la crate `png`,
  pas codées à la main). Un véritable asset de test
  (`assets/sample_sprite.png`, un damier 2x2 avec un quadrant
  semi-transparent) a été généré et commité. Vérifié sur du matériel
  Windows réel (NVIDIA GT 730) et Linux réel (WSL2 Ubuntu/Mesa
  llvmpipe) : les quadrants opaques correspondent exactement, et le
  quadrant semi-transparent produit exactement la couleur composite à
  blending alpha prédite par la formule standard « over ». Espace de
  travail complet : `cargo build`/`clippy` propres (0 avertissement) ;
  `cargo test --workspace --release` réussit les 33 tests sur matériel
  réel + 56 tests unitaires, aucune régression. Divulgation honnête :
  les PNG entrelacés et les PNG à 16 bits par canal ne sont pas
  réellement testés (uniquement pris en charge via la gestion
  automatique de la crate `png`) ; `directx-graphics-window` (la démo
  en fenêtre réelle) n'appelle pas encore ce chargeur (toujours une
  texture 1x1 de couleur unie).

- **2026-08-08 (suite 11) : blending alpha (sprites semi-transparents)
  implémenté, vérifié sur du matériel Windows + Linux réel.** Le
  blending alpha standard « over » a été activé dans la construction
  du pipeline de `render_sprites_and_read_back` (`SRC_ALPHA`/
  `ONE_MINUS_SRC_ALPHA`/`ADD`). Comme `src.a=1.0` est numériquement
  équivalent à `blend_enable=false`, tous les tests de sprites opaques
  existants ont été confirmés comme produisant des résultats
  identiques (changement non-destructif et additif). Un nouveau test a
  vérifié le résultat de la formule standard « over » sur du matériel
  réel, correspondant exactement sur les deux systèmes
  d'exploitation. Divulgation honnête : ce blending n'est activé que
  dans le chemin hors écran `render_sprites_and_read_back` —
  `directx-graphics-window` (la démo en fenêtre réelle) n'a pas encore
  été mise à jour pour correspondre ; seul le blending « over » est
  supporté (pas de modes additif/multiplicatif) ; l'interaction entre
  test de profondeur/ordre de blending n'est pas testée (cette passe
  n'a pas de buffer de profondeur, sprites 2D uniquement).

Pour l'historique complet session par session (y compris le jalon
antérieur du pipeline graphique D3D11, la tranche verticale DXIL, la
boucle de jeu en fenêtre réelle + swapchain + entrée clavier, et les
nombreux incréments de longueur de chaîne DXBC/DXIL avec vérification
des limites), voir [CLAUDE.md](CLAUDE.md) (japonais, faisant autorité).
