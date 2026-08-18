# PORTING.md — ce qui est réutilisable, par qui, et comment (condensé)

> **Remarque** : Ceci est une traduction condensée. Le guide technique
> complet avec les détails de code et les pièges reste disponible
> uniquement dans le [PORTING.md](PORTING.md) original — consultez-le
> avant d'adopter réellement un motif.

Résumé des motifs d'implémentation réutilisables de ce projet, pour
quiconque souhaitant les porter dans un autre projet :

1. **`crates/directx-shader-translate`** : un parseur de
   conteneur/chunk DXBC (bytecode de shaders D3D9/10/11, SM<=5.1)
   (`parse_dxbc`), ainsi qu'un traducteur DXBC(SM5.0)→SPIR-V à portée
   étroite (`translate_shader`) pour exactement les formes d'opcodes
   réellement émises par `fxc.exe` pour les shaders propres à ce
   projet : opérations uniques (add/mul/sub-via-add-négé/div), chaînes
   d'opérations binaires séquentielles de longueur arbitraire (basées
   sur un arbre d'expression de registres, pas un codage en dur par
   longueur — vérifié jusqu'à 7 termes avec *zéro* changement de code
   de production nécessaire par nombre de termes supplémentaire), et
   variantes avec vérification des limites (`if (id.x < N)`) des
   mêmes, générant du véritable SPIR-V
   `OpSelectionMerge`/`OpBranchConditional`. Toute autre forme
   d'opcode est honnêtement rejetée via
   `SpirvGenError::UnsupportedShader` plutôt que mal traduite en
   silence — ceci **n'est pas** un décodeur SM5.0 général.
2. **Support DXIL (SM6+, D3D12)** (`src/dxil.rs`) :
   `parse_dxil_container` parcourt le bitstream LLVM brut (via la
   crate générique `llvm-bitcode`) pour résoudre la table de types et
   décoder les instructions de fonction ; les fonctions de la famille
   `resolve_vector_add_dxil_calls` désambiguïsent les appels
   intrinsèques `dx.op.*` de DXIL (CreateHandle/ThreadId/BufferLoad/
   BufferStore) par rapport aux numéros d'opcode documentés dans le
   DXIL.rst de Microsoft. `translate_dxil_..._to_spirv` partage un
   unique émetteur SPIR-V (`emit_spirv_for_kernel`) avec le backend
   DXBC, de sorte que les deux formats de conteneur produisent du
   SPIR-V via un seul chemin de code. La taille du groupe de travail
   (`numthreads`) est véritablement extraite du `METADATA_BLOCK` de
   DXIL (`dx.entryPoints` → `ShaderProperties`), pas codée en dur. La
   généralisation des chaînes avec vérification des limites (voir
   ci-dessus) a été vérifiée de bout en bout sur DXBC et DXIL pour des
   chaînes jusqu'à 7 termes.
3. **Pipeline graphique D3D11** : `translate_vertex_shader`/
   `translate_pixel_shader` émettent du véritable SPIR-V graphique
   pour une paire VS/PS pass-through fixe, validée de deux manières
   indépendantes (reparse rspirv + le véritable `spirv-val.exe` du
   Vulkan SDK, tous deux code de sortie 0). La nouvelle crate
   `crates/directx-graphics-vulkan` ajoute `ash` comme dépendance
   **directe** propre à ce projet (pas superposée sur
   `opencuda-vulkan`, dont l'audit de source a confirmé qu'il ne fait
   que du dispatch de calcul) et implémente une véritable render pass,
   un framebuffer, et un `VkGraphicsPipelineCreateInfo`. Elle dessine
   et relit des triangles et des scènes texturées/multi-sprites avec
   du blending alpha standard « over » et du chargement de texture
   depuis de véritables fichiers PNG (crate `png`), tous vérifiés sur
   du matériel réel NVIDIA GT 730 (Windows) *et* WSL2 Ubuntu/Mesa
   llvmpipe (Linux), avec des résultats correspondants sur les deux
   systèmes d'exploitation.
4. **`crates/directx-graphics-window`** : une véritable fenêtre + une
   véritable swapchain Vulkan + une véritable entrée clavier (winit +
   ash-window), maintenant sa propre instance/périphérique Vulkan
   indépendant séparé du contexte hors écran de
   `directx-graphics-vulkan` — garder les deux synchronisés si les
   deux sont utilisés (par exemple, le blending alpha n'est
   actuellement activé que dans le chemin hors écran). A produit une
   démo interactive de style Breakout, confirmée fonctionnelle par les
   propres yeux de l'utilisateur (mouvement de la raquette + rebond de
   la balle).
5. **Convention de dépendance par chemin** : cette crate dépend de
   `opencuda-core`/`opencuda-vulkan` d'`open-cuda` via des dépendances
   de chemin relatif, un niveau de répertoire plus profond que la
   convention des dépôts frères sous `F:\runo` utilisée ailleurs dans
   cet écosystème (par exemple `aruaru-llm`, `aruaru-db`) — ce sont des
   **dépendances de développement uniquement**, la bibliothèque
   publiée elle-même n'a aucune dépendance à `open-cuda`.
6. **Note sur la portée de l'anti-triche au niveau noyau** : pour
   quiconque porte ce projet vers un objectif « faire fonctionner de
   vrais jeux Windows sur Linux » — l'anti-triche en mode noyau (Riot
   Vanguard, BattlEye en mode noyau, etc.) bloque par conception les
   environnements de type Linux/Proton, indépendamment de la
   complétude atteinte par cette couche de traduction de shaders. Ceci
   n'est pas un défaut à corriger ; les titres utilisant un tel
   anti-triche sont hors de portée pour ce projet quelle que soit la
   complétude de la traduction.

## Ce qui n'est PAS encore réutilisable (lacunes honnêtes)

- Aucun décodeur d'instructions SM5.0 ou SM6.0 général — seules les
  classes spécifiques d'opcodes/formes décrites ci-dessus sont gérées ;
  tout le reste est honnêtement rejeté, pas mal traduit.
- Les couches de plus haut niveau de D3D12 (listes de commandes, tas
  de descripteurs, signatures racine) ne sont absolument pas
  implémentées.
- Aucun buffer de profondeur, aucune vérification d'interpolation
  entre sommets de couleurs différentes au-delà du cas du triangle en
  dégradé, et le matériel GPU AMD/Intel ainsi que l'exécution native
  macOS/bureau Linux restent non vérifiés (des chemins de code
  existent pour la détection d'ID fournisseur PCI AMD/Intel mais
  n'ont jamais été exécutés sur du véritable matériel AMD/Intel dans
  cet environnement de développement).

---

Pour le détail technique complet (séquences d'opcodes exactes, traces
de bitstream DXIL au niveau octet, extraits de code, et les exemples
complets de Cargo.toml pour la dépendance de chemin), voir le
[PORTING.md](PORTING.md) original (anglais, faisant autorité).
