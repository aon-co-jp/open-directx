# open-directx

> 📌 **آخر التحديثات (2026-08-08، نموذج أولي لرسم الـ2D sprites)**:
> استجابةً لاقتراح المستخدم "تطوير نماذج أولية للألعاب/التعدين/LLM على
> جهاز GT730 عبر open-directx على dream-os/Linux"، بدأنا بشكل مُركّز
> بنموذج أولي لرسم sprites ثنائية الأبعاد. أول تنفيذ لأخذ عينات
> النسيج (`Texture2D.Sample`) ← دعم sprites متعددة/sprite sheets ←
> حلقة اللعبة (تحديث الموضع + فيزياء الارتداد) ← **نافذة حقيقية +
> swapchain فيتلكان حقيقي + إدخال لوحة مفاتيح حقيقي** (إنشاء حزمة
> `directx-graphics-window` جديدة، قام المستخدم نفسه بتشغيلها وأكّد
> بصريًا "تحركت المضرب وردّت الكرة") ← مزج ألفا (sprites شبه شفافة) ←
> تحميل نسيج من ملفات PNG حقيقية، وهي سلسلة من الخطوات التدريجية تم
> التحقق منها جميعًا على جهاز Windows حقيقي (NVIDIA GT 730)، وبعضها على
> جهاز Linux حقيقي أيضًا (WSL2 Ubuntu). المرشحون التاليون: sprites
> متعددة متحركة + كشف التصادم، ودعم تغيير حجم النافذة. للتفاصيل انظر
> [CLAUDE.md](CLAUDE.md).

> 📌 مهمة معلّقة (2026-08-06): هناك تصوّر لدمج تقنية Toshiba SBM
> وتقنية DeepSeek (مستهدفة 8 مستودعات منها dream-os). للتفاصيل انظر
> [CLAUDE.md](CLAUDE.md).

> 📌 **آخر التحديثات (2026-08-08)**: تمت إزالة عدم التناظر الذي كانت
> فيه سلسلة الـ7 حدود مع فحص الحدود (boundary check) موجودة فقط على
> جانب DXBC دون DXIL — تمت إضافة وتجميع فعلي بواسطة `dxc.exe` لملف
> `vector_add_mul_div_sub_add_mul_div_chain7_bounded_dxil.hlsl` الجديد،
> وتم التحقق على جهاز NVIDIA GT730 حقيقي من التطابق العددي مع التنفيذ
> المرجعي على وحدة المعالجة المركزية (CPU) وعمل فحص الحدود (إجمالي
> مساحة العمل: 50 اختبار وحدة + 22 اختبار على جهاز حقيقي، جميعها
> ناجحة، بدون أي تحذيرات). للتفاصيل انظر [CLAUDE.md](CLAUDE.md).

> 📌 **آخر التحديثات (2026-08-07)**: تم توسيع سلسلة DXBC/DXIL مع فحص
> الحدود إلى 6 حدود، وتم التحقق من عملها على جهاز NVIDIA GT730 حقيقي.
> تم بحث تعزيز التكامل مع dream-os/open-cuda/aruaru-llm (نقل SBM/
> DeepSeek وغيرها)، لكن تقرر عدم إجراء أي تغيير في الكود لأن التوسيع
> بدون فهم عميق لمنطق توليد سلسلة DXBC/DXIL الحالي يحمل خطر تفويت
> أخطاء عددية، وتم تسجيل نتائج البحث بصدق في [CLAUDE.md](CLAUDE.md).

> **تحديث 2026-07-25**: تمّت إعادة تسمية عنوان ملف سياسة التطوير
> (`CLAUDE.md`) من "سياسة التطوير وقواعد بيئة التطوير" إلى "فلسفة
> التصميم وسياسة التطوير وقواعد بيئة التطوير"، وذلك لفصل فلسفة تصميم
> المشروع (ما نُقدّره) عن سياسة التطوير (كيف نعمل) عن قواعد بيئة
> التطوير (الاتفاقيات التشغيلية الملموسة) بشكل أوضح. انظر `CLAUDE.md`
> للتفاصيل.


طبقة توافقية لـ DirectX (D3D9/10/11/12) عبر الأنظمة (cross-platform) —
بروح مشاريع DXVK / vkd3d-proton — تهدف إلى تشغيل تطبيقات DirectX
الخاصة بـ Windows دون تعديل على Linux (ومستقبلًا Android/macOS) عبر
ترجمة bytecode الخاص بالتظليل (shader) DXBC/DXIL إلى SPIR-V وإرساله
عبر خلفية حوسبة Vulkan موجودة بالفعل (`opencuda-vulkan` التابعة لمشروع
[open-cuda](https://github.com/aon-co-jp/open-cuda)).

انظر [`CLAUDE.md`](CLAUDE.md) للحصول على المبرر التصميمي الكامل،
والنطاق/خارطة الطريق الصادقة، وسجل تسليم الجلسات (HANDOFF) — هذا
الملف (README) يلخّص فقط الحالة الحالية المُتحقَّق منها.

### مصفوفة دعم المنصات والموردين (أُضيفت 2026-07-27، إفصاح صادق)

إن DirectX بحد ذاته هو API خاص بـ Windows/Xbox فقط — "عبر الأنظمة"
هنا يعني أن bytecode الخاص بـ DXBC/DXIL يُترجَم إلى SPIR-V ويُرسَل عبر
Vulkan، وهذا هو ما يصل فعليًا إلى المنصات غير Windows. لا يوجد اليوم
أي `cfg(windows)` أو أي بوابة خاصة بمنصة أخرى في كود هذا المستودع نفسه
(محلل DXBC، وتوليد كود SPIR-V، و`directx-graphics-vulkan` كلها Rust +
`ash` عادية ومحايدة للمنصة)، لذا فإن قابلية بناء/اختبار المشروع عبر
المنصات تتبع مدى وصول Vulkan نفسه:

| المنصة | المسار | الحالة |
|---|---|---|
| Windows | Vulkan أصلي (native) | **مُتحقَّق منه على جهاز حقيقي** (جهاز التطوير لهذا المستودع، NVIDIA GeForce GT 730) |
| Linux | Vulkan أصلي | يُفترض أن يُبنى/يُشغَّل دون تعديل (لا يوجد كود خاص بـ Windows يعيق ذلك) — **لم يُختبَر بعد على جهاز Linux حقيقي في هذه البيئة** |
| Android | Vulkan أصلي | تحقّق `open-cuda` من نجاح الترجمة المتقاطعة (cross-compilation) لـ `aarch64-linux-android` (حسب ملف CLAUDE.md الخاص به)؛ التشغيل الفعلي على جهاز حقيقي (`vkCreateInstance` على هاتف فعلي) لا يزال معلّقًا |
| macOS | Vulkan عبر [MoltenVK](https://github.com/KhronosGroup/MoltenVK) (يترجم إلى Metal) | لم تتم المحاولة بعد — MoltenVK طبقة ترجمة وليست Vulkan أصلية، لذا فهذا ضمان أضعف مقارنةً بـ Linux/Android |
| iOS / iPadOS (أُضيفت 2026-08-17) | Vulkan عبر MoltenVK (يترجم إلى Metal) | لم تتم المحاولة بعد. **ينطبق نفس تحفظ MoltenVK الخاص بـ macOS** — لا يعمل Vulkan أصليًا على iOS/iPadOS، بل فقط عبر طبقة الترجمة هذه، لذا فإن التكافؤ مع مسار Windows/Vulkan الأصلي غير مضمون إلى أن يتم تجربته فعليًا على جهاز. كما يتطلب برنامج Apple Developer للتوزيع الرسمي. |
| أنظمة UNIX/BSD المتنوعة (أُضيفت 2026-08-17) | Vulkan أصلي، على الأرجح | غير مبحوث — دعم Vulkan يختلف حسب التوزيعة/التعريف؛ يُتوقّع إعادة استخدام معظم مسار Linux بمجرد البحث فيه |
| Sony PlayStation 4/5/6/7 | غير منطبق | خارج النطاق صراحةً حاليًا — انظر ملاحظة "أهداف عائلة PlayStation" أدناه و`CLAUDE.md` |
| Nintendo Switch 2 / 3 (أُضيفت 2026-08-17) | غير منطبق | نفس حالة "الطموح المستقبلي، مؤجّل بانتظار SDK/NDA رسمي" الخاصة بـ PlayStation. **لم تُعلن Nintendo رسميًا عن Switch 3 حتى 2026-08-17 — إدراجه هنا مجرد عنصر نائب في حال/عند الإعلان عنه، وليس مبنيًا على معلومات منتج حقيقية.** |

تغطية موردي وحدات معالجة الرسوميات (GPU vendor) (مطابقة PCI vendor ID،
متسقة عبر هذا المستودع و`open-cuda`: NVIDIA `0x10DE`، AMD
`0x1002`/`0x1022`، Intel `0x8086`):

| المورّد | الحالة |
|---|---|
| NVIDIA | **مُتحقَّق منه على جهاز حقيقي** (GeForce GT 730) |
| AMD | كود مطابقة vendor ID موجود ويمر بفحص الأنواع (type-check)، لكن **لم يُشغَّل أبدًا على جهاز AMD حقيقي** في هذه البيئة — يُعامَل على أنه غير مُتحقَّق منه |
| Intel | نفس حالة AMD: الكود موجود، **لم يُتحقَّق منه أبدًا على جهاز Intel GPU حقيقي** |

لا حاجة لأي إصلاح لجعل معرّفات الموردين الثلاثة هذه *قابلة للكشف* —
الكود صحيح بالفعل ومتطابق عبر `open-directx`/`opencuda-vulkan`/
`opencuda-directx`. المفقود هو وجود جهاز AMD/Intel حقيقي لتشغيل ذلك
المسار فعليًا، وهو ما لا تملكه بيئة التطوير هذه.

## الحالة الحالية (2026-07-27، الأحدث: استيفاء التدرّج اللوني (gradient
interpolation)، تشخيص موردي GPU، سلاسل الطرح/القسمة)

هبطت ثلاث زيادات فوق خط أنابيب الرسوميات الأدنى (minimal graphics
pipeline) لـ D3D11 وعمل فئة سلاسل DXBC أدناه، وجميعها مُتحقَّق منها على
جهاز NVIDIA GT 730 الحقيقي لهذه الآلة: (1)
`render_gradient_triangle_and_read_back` — يمكن لخط أنابيب الرسوميات
الآن تخصيص لون مميز لكل رأس (vertex) (وليس فقط حالة اللون الموحّد
المتدهورة)، وتم التحقق من ذلك عبر فحص ثابت "تجزئة الوحدة" (partition
of unity) على وحدات بكسل مقروءة (readback) من الجهاز الحقيقي. (2)
`enumerate_graphics_devices()` — يسدّ فجوة في التكافؤ التشخيصي كان
فيها مسار الحوسبة (Compute) في `open-cuda` يملك كشف vendor ID بينما
مسار الرسوميات (Graphics) هنا لا يملكه؛ مستقل تمامًا، بدون أي اعتمادية
جديدة على `opencuda-vulkan`. (3) أصبح `decode_chain_shape` يدعم الآن
`sub`/`div` (كانا مرفوضين صراحةً سابقًا باعتبارهما غير قابلين للتحقق)
— تم تجميع تظليل جديد (`vector_sub_div_chain.hlsl`) فعليًا بواسطة
`fxc.exe` واستُخدم مُخرَج SHEX الخاص به للتأكد من ترتيب المعاملات
(operands) الدقيق، ثم تم التحقق من النهاية إلى النهاية مقابل تنفيذ
مرجعي على CPU على جهاز حقيقي. انظر سجل HANDOFF في `CLAUDE.md`
(مدخلات 2026-07-27) للسرد الكامل.

## الحالة الحالية (2026-07-25، الأحدث: اكتملت الشريحة الرأسية (vertical
slice) لـ DXIL على جهاز حقيقي)

وصلت الآن الشريحة الرأسية لمحوسِب التظليل (compute shader) لـ
D3D12/DXIL إلى تكافؤ كامل مع نظيرتها D3D11/DXBC: يتم فك ترميز
`vector_add.dxil` (مُخرَج حقيقي من `dxc.exe -T cs_6_0`) من النهاية إلى
النهاية (الحاوية -> بث LLVM بت (bitstream) -> جدول الأنواع ->
التعليمات -> فك غموض جميع سجلات `Call` السبعة إلى معنى `dx.op.*`
حقيقي) وتُترجَم إلى SPIR-V حقيقي
(`directx_shader_translate::translate_dxil_vector_add_to_spirv`)،
والذي يقوم `tests/vector_add_dxil_real_vulkan.rs` بإرساله على جهاز
NVIDIA GT 730 الحقيقي لهذه الآلة ويتحقق عدديًا من تطابقه مع التنفيذ
المرجعي على CPU `a[i]+b[i]`. لا يزال هذا شكل تظليل معروف واحد فقط،
وليس فك ترميز عام لـ SM6.0 — انظر "غير المُنفَّذ (نطاق صادق)" أدناه
للحدود الدقيقة. يتم الآن استخراج حجم مجموعة العمل (workgroup size)
الخاص بـ SPIR-V فعليًا من `METADATA_BLOCK` الخاص بـ DXIL
(`dx.entryPoints` -> `ShaderProperties` -> `NumThreads`)، وليس مُثبَّتًا
بشكل صلب (hardcoded) — انظر مدخل HANDOFF "المتابعة 9" بتاريخ
2026-07-25 في `CLAUDE.md` للسرد الكامل، و"المتابعة 7" للإنجاز الأصلي
للشريحة الرأسية الذي سدّ هذا فجوة معروفة فيه.

## الحالة الحالية (2026-07-25، متابعة: تحليل DXIL على مستوى بث البت +
تحليل DXBC لـ D3D11 VS/PS)

هبطت قطعتا عمل جديدتان فوق الشريحة الرأسية لمرحلة 1 لمحوسِب التظليل
أدناه:

- **DXIL (D3D12/SM6+) — تم تحليل بايتات حقيقية، على مستوى الحاوية/بث
  البت فقط.** تقوم `crates/directx-shader-translate/src/dxil.rs`
  (`parse_dxil_container`) بتحليل حاوية DXBC حقيقية مُجمَّعة بواسطة
  `dxc.exe -T cs_6_0` (`shaders/vector_add_dxil.hlsl` ->
  `shaders/vector_add.dxil`، مُنتَجة بواسطة
  `tools/compile-dxbc-shaders.ps1`): تستخرج `DxilProgramHeader`/
  `DxilBitcodeHeader` الخاصين بقطعة `DXIL` (نوع التظليل، SM6.0، إصدار
  DXIL) عبر حزمة `dxbc` الموجودة، ثم تسلّم حمولة LLVM bitcode الخام
  إلى حزمة `llvm-bitcode` (اعتمادية جديدة، قارئ بث بت LLVM عام بدون أي
  معرفة خاصة بـ DXIL) لفك ترميز شجرة الكتل/السجلات فعليًا. تم التأكد
  مقابل البايتات الحقيقية: علامة التغليف السحرية لـ LLVM
  `BC\xC0\xDE`، وكتلة `MODULE_BLOCK` واحدة على المستوى الأعلى (id 8)،
  وكتل فرعية قياسية من LLVM بداخلها — `TYPE_BLOCK_ID_NEW`(17)،
  `PARAMATTR_GROUP_BLOCK`(10)، `PARAMATTR_BLOCK`(9)،
  `CONSTANTS_BLOCK`(11)، `FUNCTION_BLOCK`(12، ×5 — واحدة لكل كتلة
  أساسية في `main`)، `VALUE_SYMTAB_BLOCK`(14)، `METADATA_BLOCK`(15،
  ×2). **تحديث (2026-07-25، متابعة، مسار D3D12)**: تمت منذ ذلك الحين
  إضافة حل جدول الأنواع وفك ترميز التعليمات الخشن (`resolve_type_table`
  /`decode_function_instructions` في الملف نفسه)، بتطبيق جداول سجلات
  `TYPE_BLOCK`/`FUNC_CODE` الموثّقة من LLVM على بايتات `vector_add.dxil`
  الحقيقية — تأكّد جدول أنواع من 22 مُدخلًا يتضمن `Float` و
  `StructNamed{"class.RWStructuredBuffer<float>"}`، وتسلسل تعليمات
  حقيقي (`DeclareBlocks -> Call×5 -> ExtractValue -> Call ->
  ExtractValue -> BinOp -> Call -> Ret`). **تحديث (2026-07-25، متابعة
  6)**: تم الآن فك غموض جميع سجلات `Call` السبعة. تقوم
  `resolve_vector_add_dxil_calls` بحل أسماء دوال
  `VALUE_SYMTAB_BLOCK` (المُكتشَفة عبر `Record::take_payload()`، وليس
  `fields()` — فجوة حقيقية في فهم الحزمة في المدخل السابق) وفك ترميز
  ترميز المعامل ذي القيمة النسبية (relative-value operand encoding)
  الخاص بـ LLVM يدويًا (تم التحقق منه يدويًا مقابل البايتات الحقيقية)،
  مما يعطي `[CreateHandle{range_id:2}, CreateHandle{range_id:1},
  CreateHandle{range_id:0}, ThreadId, BufferLoad{handle_range_id:0},
  BufferLoad{handle_range_id:1}, BufferStore{handle_range_id:2}]`. تم
  التأكد من أرقام أكواد عمليات DXIL (`CreateHandle`=57،
  `BufferLoad`=68، `BufferStore`=69، `ThreadId`=93) عبر بحث ويب مقابل
  `DirectXShaderCompiler/docs/DXIL.rst` الخاص بـ Microsoft، وليس
  افتراضًا من الذاكرة، وتطابقت تمامًا مع الثوابت الحقيقية المفكوكة.
  **لا تزال لا يوجد ترجمة DXIL-إلى-SPIR-V** — تلك هي الزيادة التالية.
  انظر "غير المُنفَّذ" أدناه.
- **خط أنابيب رسوميات D3D11 — تم الوصول إلى توليد SPIR-V حقيقي لـ
  VS/PS والتحقق منه، بدون rasterizer/draw بعد.** تم تجميع
  `shaders/triangle_vs.hlsl`/`shaders/triangle_ps.hlsl` (زوج تظليل
  رأسي وبكسل تمريري (passthrough) أدنى، `POSITION`/`COLOR` كمدخل،
  `SV_POSITION`/`SV_TARGET` كمخرج) بواسطة `fxc.exe /T vs_5_0`/
  `/T ps_5_0` حقيقي. يقوم `parse_dxbc` بتحليل كليهما بدون تعديل.
  يقوم `spirv_gen::translate_vertex_shader`/`translate_pixel_shader`
  (جديد) بفك ترميز تسلسل أكواد عمليات SHEX الثابت والحقيقي
  (`dcl_input`×2/`dcl_output_siv`/`dcl_output`/`mov`×3/`ret` لـ VS؛
  `dcl_input_ps`(linear)/`dcl_output`/`mov`/`ret` لـ PS) وإصدار
  SPIR-V رسوميات حقيقي: `OpEntryPoint Vertex`/`Fragment` (وليس
  `GLCompute`)، ومتغيرات صنف تخزين (storage class) `Input`/`Output`
  مع تزيينات `Location`، و`BuiltIn Position` على مخرج
  `SV_POSITION` للتظليل الرأسي، و`OpExecutionMode ...
  OriginUpperLeft` على تظليل الجزء (fragment shader). تم التحقق
  بطريقتين: (1) يعيد مُحمِّل `rspirv` نفسه تحليل البايتات المُصدَرة
  دون خطأ، (2) تم تشغيل `spirv-val.exe` الحقيقي المرفق مع Vulkan SDK
  (`C:\VulkanSDK\1.4.350.0\Bin\spirv-val.exe`) مقابل كلتا الوحدتين
  المُصدَرتين وأعاد رمز الخروج 0 بدون تشخيصات لكلتيهما. لا يزال
  `translate_shader`/`translate_chain_shader` (حوسبة فقط) يرفضان
  كلا التظليلين بشكل صحيح. **لا يوجد rasterizer، ولا framebuffer،
  ولا أي أمر رسم Vulkan فعلي** — تم التأكد (بقراءة مصدره الحقيقي) من
  أن `opencuda-vulkan` لا يحتوي على أي كود
  `VkGraphicsPipelineCreateInfo`/render-pass/framebuffer على
  الإطلاق، إرسال حوسبة (compute-dispatch) فقط، لذا فإن رسم بكسل فعلي
  خارج نطاق هذه المرحلة. انظر HANDOFF في `CLAUDE.md` للحد الصادق
  للمرحلة (milestone).

## الحالة الحالية (2026-07-26، تم الوصول إلى مرحلة خط أنابيب رسوميات
D3D11 الأدنى)

تضيف الحزمة الجديدة `crates/directx-graphics-vulkan` مكتبة `ash`
كاعتمادية **مباشرة** لمساحة عمل هذا المشروع (وليست مُرصوفة (layered)
فوق `opencuda-vulkan`، والتي تم التأكد عبر تدقيق المصدر أنها للإرسال
الحاسوبي فقط). تُنفِّذ ممر رسم (render pass) حقيقيًا، وإطار عمل
(framebuffer)، و`VkGraphicsPipelineCreateInfo`، مُعيدةً استخدام
SPIR-V المولَّد بالفعل والناجح في اختبار `spirv-val` من
`translate_vertex_shader`/`translate_pixel_shader` أعلاه (لا يُعاد
تنفيذ ترجمة التظليل). تقوم
`render_uniform_triangle_and_read_back` برسم "مثلث كبير" واحد يملأ
منفذ العرض بالكامل بلون رأس موحّد واحد، وتقرأ الصورة المرسومة مرة
أخرى عبر مخزن مؤقت مرحلي (staging buffer) مرئي للمضيف، ويتأكد
الاختبار على الجهاز الحقيقي
(`crates/directx-graphics-vulkan/tests/triangle_real_vulkan.rs`) من
أن كل بكسل مقروء يطابق لون الرأس التمريري على جهاز NVIDIA GT 730
الحقيقي الموجود على هذه الآلة (`cargo test -p directx-graphics-vulkan
--test triangle_real_vulkan -- --nocapture`: نجح 1). النطاق مُقيَّد
عمدًا: زوج تظليل واحد ثابت، وأمر رسم واحد، بدون مخزن عمق/نسيج/
swapchain/فحص استيفاء متعدد المثلثات. انظر HANDOFF في `CLAUDE.md`
(متابعة 2026-07-26) للإفصاح الصادق الكامل.

## الحالة الحالية (2026-07-25، تعميم الشريحة الرأسية لمرحلة 1 على 3
تظليلات معروفة)

تقوم `crates/directx-shader-translate` الآن بالشريحة الرأسية الكاملة
لـ**ثلاثة تظليلات معروفة محدَّدة** (`vector_add.hlsl`،
`vector_mul.hlsl`، `vector_sub_bounded.hlsl`): تحليل DXBC -> فك ترميز
مجموعة فرعية ضيقة من أكواد عمليات SM5.0 -> توليد كود SPIR-V (عبر
`rspirv`) -> إرسال Vulkan حقيقي (`opencuda-vulkan` التابعة لـ
`open-cuda`) -> تطابق عددي مع تنفيذ مرجعي على CPU، تم التحقق منه على
جهاز NVIDIA GT 730 الحقيقي لهذه الآلة. **هذا ليس بعد فك ترميز عام
لـ SM5.0-إلى-SPIR-V** — انظر "غير المُنفَّذ" أدناه.

- `parse_dxbc` (مرحلة 0): استقصاء حاوية/قطعة DXBC (وجود
  RDEF/ISGN/OSGN/SHEX)، دون تغيير عن الواجهة الأمامية الأصلية.
- `spirv_gen::translate_shader` (مرحلة 1، عُمِّمَت بتاريخ
  2026-07-25): تتعرّف على 3 أشكال أكواد عمليات تصدرها فعليًا
  `fxc.exe`، وجميعها تشترك في هيكل عام مشترك (`dcl_globalFlags` ->
  `dcl_constantbuffer` اختياري -> ×3 `dcl_uav_structured` ->
  `dcl_input` -> `dcl_temps` -> `dcl_thread_group` -> `ult`+`if`
  اختياري -> ×2 `ld_structured` -> `add`/`mul` -> `store_structured`
  -> `endif` اختياري -> `ret`):
  - `vector_add.hlsl`: `add`، بدون فحص حدود.
  - `vector_mul.hlsl`: `mul` بدلًا من `add`.
  - `vector_sub_bounded.hlsl`: `add` مع علامة `negate` على معامل
    مصدره الأول (تم التأكد بتفريغ مُخرَج `fxc.exe` الحقيقي — تُحسِّن
    `fxc` عملية `a - b` إلى `add dest, -b, a` بدلًا من إصدار كود
    عملية `sub` مخصص)، بالإضافة إلى فحص حدود حقيقي `if (id.x < N)`
    (`ult` مقابل مخزن مؤقت للثوابت + `if`/`endif`)، والذي يُنفِّذه
    SPIR-V المُصدَر بواسطة `OpSelectionMerge`/`OpBranchConditional`
    فعلي، مستخدمًا الثابت الدافع (push constant) `n` للمقارنة.
  يتم رفض أي شكل/كود عملية آخر عبر
  `SpirvGenError::UnsupportedShader` بدلًا من ترجمته خطأً بصمت. تُستخرَج
  نقاط ربط UAV، وحجم مجموعة الخيوط، والعملية (operator)، ووجود فحص
  الحدود، جميعها من DXBC المُحلَّل الحقيقي، وليست مُثبَّتة بشكل صلب.
  تُبقى `translate_vector_add_shader` كاسم مستعار (alias) رقيق متوافق
  خلفيًا لـ `translate_shader`.
- `tests/vector_add_real_vulkan.rs`،
  `tests/vector_mul_real_vulkan.rs`،
  `tests/vector_sub_bounded_real_vulkan.rs`: يقوم كل منها بإرسال
  SPIR-V المُترجَم الخاص به عبر `opencuda-vulkan::VulkanDevice`
  الحقيقي (`ash`، ميزة `real-vulkan`) التابع لـ `open-cuda`، ويتحقق
  من مُخرَج GPU مقابل تنفيذ مرجعي على CPU لـ 256 عنصرًا (بحدود خطأ
  1e-3/1e-2). يقوم اختبار فحص الحدود إضافيًا بإرسال 320 خيطًا (thread)
  بعدد عناصر منطقي قدره 256 ويتأكد أن العناصر من 256..320 لا تُكتَب
  أبدًا (تبقى عند قيمة حارسة/sentinel)، مما يثبت أن فرع `if (id.x <
  N)` في SPIR-V المُولَّد يتحكم فعليًا في التنفيذ وليس مجرد التجميع.
- `examples/dump_shex.rs`: أداة صغيرة مستقلة
  (`cargo run -p directx-shader-translate --example dump_shex --
  <path.dxbc>`) استُخدمت خلال هذه الجلسة لفحص تدفقات أكواد عمليات
  SHEX الحقيقية قبل كتابة دعم فك الترميز لها؛ مُبقاة لعمل التعميم
  المستقبلي كودًا بكود.

**منذ كتابة عنوان هذا القسم**، تمت إضافة تظليل رابع أحادي العملية
(`vector_div.hlsl`، `div` عادي) إلى `translate_shader` باتباع نفس
النمط تمامًا، ثم — لاحقًا — تمت إضافة فئة نمط مختلفة حقًا،
`spirv_gen::translate_chain_shader` (بجانبها وليس بدلًا منها): تفك
ترميز شجرة تعبير سجل (register-expression tree) فعلية لعمليات ثنائية
متسلسلة (جمع/ضرب، بدون تدفق تحكم) بدلًا من عملية ثابتة واحدة، وتم
التحقق منها مقابل تظليل مُجمَّع حديثًا اتضح أن SHEX الحقيقي الخاص به
يعيد استخدام مكونات سجل مؤقت واحد عبر CSE الخاصة بـ fxc بدلًا من
الإعلان عن سجلات مؤقتة إضافية. انظر مدخل HANDOFF "المتابعة 9" بتاريخ
2026-07-25 في `CLAUDE.md` للسرد الكامل والحالي (يُترَك هذا القسم كما
كُتب أصلًا للدقة التاريخية بشأن حالة منتصف يوم 2026-07-25).

## البناء والاختبار

```powershell
cargo build --workspace
cargo test --workspace -- --nocapture
```

### شاهد المشروع يرسم شيئًا فعليًا (أُضيف 2026-07-27)

هذا المستودع عبارة عن مجموعة من المكتبات بدون `fn main` خاصة به، لذا
فإن أسرع طريقة *لرؤية* خط أنابيب الرسوميات يعمل على وحدة معالجة
الرسوميات (GPU) الخاصة بك — بدلًا من قراءة كود الاختبار — هي:

```bash
cargo run -p directx-graphics-vulkan --example render_triangle
```

يُعيد هذا استخدام نفس تظليلات DXBC → SPIR-V الحقيقية المُجمَّعة
بواسطة fxc.exe والمُترجَمة والمستخدمة في
`tests/triangle_real_vulkan.rs`، ويرسم مثلثًا متدرجًا (أحمر/أخضر/
أزرق) على جهاز Vulkan حقيقي، ويقرأ الـ framebuffer مرة أخرى، ويكتبه
إلى `render_triangle.ppm` (تنسيق PPM عادي، بدون الحاجة إلى أي
اعتمادية إضافية على حزمة صور — حوّله مثلًا باستخدام
`magick render_triangle.ppm render_triangle.png` أو افتحه مباشرةً في
معظم عارضات الصور). إذا لم يوجد جهاز/برنامج تشغيل Vulkan قابل
للاستخدام، فإنه يطبع خطأ صادقًا ويخرج برمز غير صفري بدلًا من
تزييف النجاح.

المُخرَج المُلاحَظ فعليًا (2026-07-25، هذه الآلة، NVIDIA GeForce GT
730):

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

`cargo clippy --workspace --all-targets`: 0 تحذير.

بعد عمل حل جدول أنواع/فك ترميز تعليمات DXIL (2026-07-25، متابعة، مسار
D3D12)، يُشغِّل `cargo test --workspace` إجمالي 23 اختبارًا (19 اختبار
وحدة + 4 اختبارات تكامل على Vulkan حقيقي)، جميعها ناجحة، بما في ذلك 3
اختبارات جديدة فوق الـ20 السابقة: `dxil::tests::resolves_real_dxil_
type_table_and_finds_float_and_resource_struct`،
`dxil::tests::decodes_real_dxil_function_block_into_matching_vector_
add_shape`، و`dxil::tests::shape_matcher_honestly_rejects_unexpected_
instruction_orderings`.

لإعادة توليد ثوابت (fixtures) DXBC من HLSL (يتطلب `fxc.exe` الخاصة
بـ Windows SDK — لاحظ أن `dxc.exe` تستهدف فقط DXIL/SM6+ ولا يمكنها
إنتاج DXBC):

```powershell
pwsh tools/compile-dxbc-shaders.ps1
```

## غير المُنفَّذ (نطاق صادق)

- **فك ترميز تعليمات SM5.0 العام.** يُعالَج فقط أشكال أكواد العمليات
  الثلاثة أعلاه؛ يُرفَض أي تظليل حوسبة D3D11 آخر (تخطيط موارد مختلف،
  تدفق تحكم آخر، حدوس (intrinsics) أخرى، أكثر من فحص حدود واحد،
  `div`/`sub` كأكواد عمليات حقيقية بدلًا من `add` منفية، إلخ)، ولا
  يُترجَم خطأً. يبقى بناء فك ترميز عام حقيقي (أو تبنّي/نقل أحد
  الموجودين، مثل دراسة نهج `dxbc-spirv`/`dxil-spirv` عن قرب أكثر)
  الهدف الفعلي التالي.
- **DXIL (Shader Model 6+، D3D12): اكتملت الآن الشريحة الرأسية لـ
  `vector_add.dxil` من النهاية إلى النهاية، على جهاز حقيقي — لكن لا
  تزال لهذا الشكل الواحد المعروف للتظليل فقط، وليست SM6.0 عامة.**
  تفك `resolve_type_table`/`decode_function_instructions`/
  `resolve_vector_add_dxil_calls` في `dxil.rs` ترميز سجلات
  `TYPE_BLOCK`/`FUNCTION_BLOCK`/`VALUE_SYMTAB_BLOCK` الحقيقية مقابل
  أكواد LLVM الموثّقة وتفك غموض جميع سجلات `Call` السبعة إلى معناها
  الحقيقي `dx.op.*` (`CreateHandle`/`ThreadId`/`BufferLoad`/
  `BufferStore`، مع نقاط ربط UAV). تُغذّي
  `translate_dxil_vector_add_to_spirv` (جديدة) ذلك المُخرَج المحلول
  إلى `emit_spirv_for_kernel` المشتركة في `spirv_gen.rs` (المستخرَجة
  من `emit_spirv` الخاصة بمسار DXBC حتى تُصدِر كلتا الخلفيتين من
  مسار كود واحد) لإنتاج SPIR-V حقيقي، والذي يُرسِله
  `tests/vector_add_dxil_real_vulkan.rs` على جهاز NVIDIA GT 730
  الحقيقي لهذه الآلة عبر `opencuda-vulkan` ويتحقق من تطابقه مع
  التنفيذ المرجعي على CPU `a[i]+b[i]` لجميع الـ256 عنصرًا — نفس
  الصرامة كاختبار `vector_add` الخاص بـ DXBC. **يُستخرَج الآن حجم
  مجموعة العمل فعليًا، وليس مُثبَّتًا بشكل صلب**: تسير
  `extract_numthreads_from_metadata` (`dxil.rs`) على مسار
  `METADATA_BLOCK` الحقيقي `dx.entryPoints` -> tuple لكل نقطة دخول
  -> `ShaderProperties` -> `kDxilNumThreadsTag` (=4، تم التأكد منها
  مقابل مصادر `DxilMetadataHelper.h`/`.cpp` الخاصة بـ Microsoft
  `DirectXShaderCompiler`) وتحل عقدة `{x,y,z}` مقابل قائمة قيم
  الوحدة الحقيقية، فتُنتِج `(64,1,1)` من البايتات الفعلية لملف
  `vector_add.dxil` — تم سدّ التثبيت الصلب المعروف من المدخل
  السابق، ويثبت اختبار انحدار (regression) صناعي أن منطق الاستخراج
  يُرجِع قيمة *مختلفة* عند إعطائه بيانات وصفية (metadata) مختلفة
  (وليس فقط "يُرجِع 64,1,1 مهما كانت المدخلات"). لا يزال يُرفَض أي
  شكل عملية/معامل آخر (عملية مختلفة، كتل أساسية متعددة، فحوص حدود)،
  وليس يُترجَم خطأً. دعم قائمة أوامر D3D12/كومة الواصفات
  (descriptor heap)/توقيع الجذر (root signature) (الطبقة فوق ترجمة
  التظليل) لم يُمَس.
- **فك ترميز DXBC مُعمَّم إلى ما بعد 4 أشكال ثابتة أحادية العملية:
  يعالج الآن سلاسل من عمليات ثنائية متسلسلة (بدون تدفق تحكم) عبر
  شجرة تعبير سجل (register-expression tree) حقيقية، وليس شكلًا
  خامسًا مُثبَّتًا بشكل صلب.** يسير
  `spirv_gen::translate_chain_shader`/`decode_chain_shape` على
  `ld_structured`/`add`/`mul`/`store_structured` ويبنيان شجرة تعبير
  فعلية مفهرَسة بـ (سجل مؤقت، مكوّن)، بحيث تُعالَج عملية واحدة أو
  عمليتان أو N عملية بنفس الطريقة — تم التحقق منها مقابل تظليل
  حقيقي مُجمَّع حديثًا (`vector_add_mul_chain.hlsl`،
  `t = A[i]+B[i]; Out[i] = t*A[i]`) اتضح أن SHEX الحقيقي الخاص به
  يعيد استخدام مكونات `.x`/`.y` الخاصة بسجل مؤقت واحد (قامت fxc
  بـ CSE على تحميل `A[i]` المتكرر بدلًا من إعادة إصدار
  `ld_structured`) — اكتشاف حقيقي لم يكن متوقعًا يعالجه فك الترميز
  المعتمد على الشجرة بدون حالات إضافية. تم إرساله والتحقق منه على
  جهاز NVIDIA GT 730 الحقيقي مقابل التنفيذ المرجعي على CPU
  `(a[i]+b[i])*a[i]`. لا يزال `sub`/`div` داخل السلسلة مرفوضين
  عمدًا (لم يُتحقَّق من دلالة ترتيب معاملاتهما إلا في حالة العملية
  الواحدة). الأشكال الأربعة الأصلية أحادية العملية لم تُمَس ولا
  تزال تنجح دون تعديل.
- **خط أنابيب رسوميات D3D11: تم التأكد من عمل تحليل حاوية DXBC لـ
  VS/PS، لكن بدون توليد كود SPIR-V، وبدون rasterizer، وبدون أي
  مثلث فعلي مرسوم على الشاشة.** يبقى خط الأنابيب الكامل (rasterizer،
  أخذ عينات النسيج، حالة المزج (blend state)، دامج المُخرَج
  (output-merger)) خارج النطاق؛ وكذلك توسيع فك الترميز الضيق لأشكال
  أكواد العمليات في `spirv_gen` لفهم `dcl_output_siv`/
  `dcl_input_ps`/أنماط الاستيفاء.
- أهداف عائلة PlayStation — خارج النطاق صراحةً؛ انظر `CLAUDE.md`
  للتعليل القانوني/شروط الخدمة.

## مشاريع ذات صلة

- [open-cuda](https://github.com/aon-co-jp/open-cuda) — خلفية تنفيذ
  الحوسبة على Vulkan التي صُمِّم هذا المشروع لإرسال العمل من خلالها
  (`opencuda-core::GpuDevice`، `KernelSource::SpirV`). تحتوي أيضًا
  على حزمة `opencuda-directx` منفصلة وتعمل بالفعل دون علاقة، تُشغِّل
  D3D12 **أصليًا على Windows** — الاتجاه المعاكس لهذا المشروع (الذي
  يُشغِّل تظليلات DirectX **على منصات غير Windows**).
- [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm) — لا اعتمادية
  تقنية مباشرة على هذا المشروع (تم التحقق من ذلك، وليس افتراضًا).
