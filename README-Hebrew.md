# open-directx

> 📌 **עדכון אחרון (2026-08-08, אב-טיפוס לרינדור ספרייטים דו-ממדיים)**:
> בעקבות הצעת המשתמש "לפתח אב-טיפוס של GAME... על מחשב עם GT730 דרך
> open-directx על dream-os/Linux", הוחל תחילה בצמצום להיקף של רינדור
> ספרייטים דו-ממדיים בלבד. מימוש ראשוני של דגימת טקסטורות
> (`Texture2D.Sample`) → תמיכה במספר ספרייטים/גיליון ספרייטים →
> לולאת משחק (עדכון מיקום + פיזיקת החזרה) → **חלון אמיתי + שרשרת
> החלפה (swapchain) אמיתית של Vulkan + קלט מקלדת אמיתי** (חבילה
> (crate) חדשה בשם `directx-graphics-window`, המשתמש עצמו הריץ ואישר
> בעיניו "המחבט זז והחזיר את הכדור") → מיזוג אלפא (blend, ספרייטים
> חצי-שקופים) → טעינת טקסטורות מקבצי PNG אמיתיים — סדרת תוספות שכולן
> אומתו על מחשב Windows אמיתי (NVIDIA GT 730), וחלקן גם על מחשב Linux
> אמיתי (WSL2 Ubuntu). מועמדים הבאים: מספר ספרייטים נעים + זיהוי
> התנגשויות, תמיכה בשינוי גודל חלון. לפרטים ראו [CLAUDE.md](CLAUDE.md).

> 📌 משימה ממתינה (2026-08-06): קיים רעיון לשילוב טכנולוגיית ה-SBM
> (Simulated Bifurcation Machine) של טושיבה וטכנולוגיית DeepSeek (8
> מאגרים כולל dream-os וכו'). לפרטים ראו [CLAUDE.md](CLAUDE.md).

> 📌 **עדכון אחרון (2026-08-08)**: סגירת אי-הסימטריה בשרשרת בת 7 איברים
> עם בדיקת גבולות — עד כה קיימת רק בצד DXBC וחסרה בצד DXIL — נוספה
> קומפילציה אמיתית עם `dxc.exe` של
> `vector_add_mul_div_sub_add_mul_div_chain7_bounded_dxil.hlsl`, ואומתה
> על חומרת NVIDIA GT730 אמיתית שהתוצאה תואמת למימוש הייחוס ב-CPU
> ושבדיקת הגבולות פועלת (כל מרחב העבודה: 50 בדיקות יחידה + 22 בדיקות
> חומרה אמיתית, כולן ירוקות, אפס אזהרות). לפרטים ראו
> [CLAUDE.md](CLAUDE.md).

> 📌 **עדכון אחרון (2026-08-07)**: הרחבת שרשרת DXBC/DXIL עם בדיקת
> גבולות ל-6 איברים, אומתה על חומרת NVIDIA GT730 אמיתית. נבחנה
> אינטגרציה מעמיקה יותר עם dream-os/open-cuda/aruaru-llm (העברת
> SBM/DeepSeek), אך הוחלט שלא לבצע הרחבות מבוססות ניחוש ללוגיקת יצירת
> שרשרות DXBC/DXIL הקיימת ללא הבנה מעמיקה של התחום — לא בוצע שינוי קוד,
> הממצאים תועדו ביושר ב-[CLAUDE.md](CLAUDE.md).

> **עודכן ב-2026-07-25**: כותרת קובץ מדיניות הפיתוח (`CLAUDE.md`) שונתה
> מ"מדיניות פיתוח וכללי סביבת פיתוח" ל"פילוסופיית עיצוב ומדיניות פיתוח
> וכללי סביבת פיתוח", כדי להפריד בבירור בין פילוסופיית העיצוב של
> הפרויקט (מה אנחנו מעריכים), מדיניות הפיתוח (איך אנחנו עובדים), וכללי
> סביבת הפיתוח (מוסכמות תפעוליות קונקרטיות). לפרטים ראו `CLAUDE.md`.


שכבת תאימות DirectX (D3D9/10/11/12) חוצת-פלטפורמות — ברוח הפרויקטים
DXVK / vkd3d-proton — שמטרתה להריץ יישומי DirectX של Windows ללא שינוי
על Linux (ובעתיד גם Android/macOS), על ידי תרגום bytecode של שיידרים
מסוג DXBC/DXIL ל-SPIR-V והפעלתם דרך backend קיים לחישוב Vulkan
([open-cuda](https://github.com/aon-co-jp/open-cuda)'s
`opencuda-vulkan`).

ראו [`CLAUDE.md`](CLAUDE.md) לרציונל העיצוב המלא, להיקף/מפת הדרכים
הכנה, וליומן ה-HANDOFF של הסשנים — קובץ README זה מסכם רק את המצב
הנוכחי המאומת.

### טבלת תמיכה בפלטפורמות ובספקים (נוספה ב-2026-07-27, גילוי כן)

DirectX עצמה היא API בלעדי ל-Windows/Xbox — "חוצת פלטפורמות" כאן
משמעו ש-bytecode של DXBC/DXIL מתורגם ל-SPIR-V ומופעל דרך Vulkan, וזה
מה שבפועל מגיע לפלטפורמות שאינן Windows. אין קיים היום בקוד המאגר הזה
עצמו שום `cfg(windows)` או שער (gate) פלטפורמה אחר (מנתח ה-DXBC, יצירת
קוד ה-SPIR-V, ו-`directx-graphics-vulkan` — כולם Rust + `ash` פשוט
ובלתי-תלוי-פלטפורמה), כך שניידות הבנייה/הבדיקה עוקבת אחר טווח ההגעה
של Vulkan עצמו:

| פלטפורמה | נתיב | סטטוס |
|---|---|---|
| Windows | Vulkan ילידי (native) | **אומת על חומרה אמיתית** (מכונת הפיתוח של מאגר זה, NVIDIA GeForce GT 730) |
| Linux | Vulkan ילידי | אמור להיבנות/לרוץ ללא שינוי (אין קוד ספציפי ל-Windows שחוסם זאת) — **טרם נבדק על מכונת Linux אמיתית בסביבה זו** |
| Android | Vulkan ילידי | `open-cuda` אימת שקומפילציה חוצת-פלטפורמות ל-`aarch64-linux-android` מצליחה (לפי ה-CLAUDE.md שלו); הרצה על מכשיר אמיתי (`vkCreateInstance` על טלפון אמיתי) עדיין ממתינה |
| macOS | Vulkan דרך [MoltenVK](https://github.com/KhronosGroup/MoltenVK) (מתרגם ל-Metal) | טרם נוסה — MoltenVK היא שכבת תרגום, לא Vulkan ילידי, כך שזו ערבות חלשה יותר מ-Linux/Android |
| iOS / iPadOS (נוסף ב-2026-08-17) | Vulkan דרך MoltenVK (מתרגם ל-Metal) | טרם נוסה. **אותה הסתייגות MoltenVK כמו ב-macOS חלה גם כאן** — Vulkan אינו רץ באופן ילידי על iOS/iPadOS, רק דרך שכבת תרגום זו, כך שאין ערבות לשוויון עם הנתיב הילידי Windows/Vulkan עד שינוסה בפועל על מכשיר. דורש גם את Apple Developer Program להפצה רשמית. |
| מערכות UNIX/BSD שונות (נוסף ב-2026-08-17) | Vulkan ילידי, ככל הנראה | לא נחקר — תמיכת Vulkan משתנה לפי הפצה/דרייבר; צפוי לעשות שימוש חוזר ברוב נתיב ה-Linux לאחר חקירה |
| Sony PlayStation 4/5/6/7 | לא רלוונטי | מחוץ להיקף במפורש כרגע — ראו את ההערה "יעדי משפחת PlayStation" למטה ו-`CLAUDE.md` |
| Nintendo Switch 2 / 3 (נוסף ב-2026-08-17) | לא רלוונטי | אותו סטטוס "שאיפה עתידית, נדחית עד ל-SDK/NDA רשמי" כמו PlayStation. **Switch 3 טרם הוכרז רשמית על ידי נינטנדו נכון ל-2026-08-17 — הכללתו כאן היא רק מציין מקום (placeholder) למקרה של הכרזה, ואינה מבוססת על מידע מוצר אמיתי.** |

כיסוי ספקי GPU (התאמת PCI vendor ID, עקבי בין מאגר זה ל-`open-cuda`:
NVIDIA `0x10DE`, AMD `0x1002`/`0x1022`, Intel `0x8086`):

| ספק | סטטוס |
|---|---|
| NVIDIA | **אומת על חומרה אמיתית** (GeForce GT 730) |
| AMD | קוד התאמת vendor ID קיים ועובר type-check, אך **מעולם לא רץ מול חומרת AMD אמיתית** בסביבה זו — יש להתייחס אליו כלא-מאומת |
| Intel | כמו AMD: הקוד קיים, **מעולם לא אומת מול חומרת Intel GPU אמיתית** |

אין צורך בתיקון כדי להפוך את שלושת מזהי הספקים האלה ל*ניתנים לזיהוי* —
הקוד כבר נכון וזהה בין `open-directx`/`opencuda-vulkan`/
`opencuda-directx`. מה שחסר הוא חומרת AMD/Intel אמיתית כדי להפעיל בפועל
את נתיב הקוד הזה, ולסביבת פיתוח זו אין כזו.

## מצב נוכחי (2026-07-27, אחרון: אינטרפולציית גרדיאנט, אבחון ספק GPU, שרשרת חיסור/חילוק)

שלוש תוספות נחתו על גבי צנרת הגרפיקה המינימלית של D3D11 ועבודת מחלקת
השרשראות של DXBC למטה, כולן אומתו על ה-NVIDIA GT 730 האמיתי של מכונה
זו: (1) `render_gradient_triangle_and_read_back` — צנרת הגרפיקה יכולה
כעת להקצות צבע נפרד לכל קודקוד (לא רק מקרה הצבע האחיד המנוון), אומת
באמצעות בדיקת אינווריאנטה של "חלוקת יחידה" (partition-of-unity) על
פיקסלים שנקראו חזרה מחומרה אמיתית. (2) `enumerate_graphics_devices()`
— סוגר פער אבחוני שבו נתיב Compute של `open-cuda` כלל זיהוי vendor-ID
בעוד נתיב ה-Graphics כאן לא כלל זאת; עצמאי, ללא תלות חדשה ב-
`opencuda-vulkan`. (3) `decode_chain_shape` תומך כעת ב-`sub`/`div`
(שקודם נדחו במפורש כבלתי-ניתנים-לאימות) — שיידר חדש
(`vector_sub_div_chain.hlsl`) קומפל בפועל עם `fxc.exe` ופלט ה-SHEX שלו
שימש לאישור סדר האופרנדים המדויק, ולאחר מכן אומת מקצה-לקצה מול ייחוס
CPU על חומרה אמיתית. ראו את ה-HANDOFF ב-`CLAUDE.md` (רשומות 2026-07-27)
לתיאור המלא.

## מצב נוכחי (2026-07-25, אחרון: פרוסה אנכית של DXIL הושלמה על חומרה אמיתית)

פרוסת compute shader של D3D12/DXIL מגיעה כעת לשוויון מלא עם זו של
D3D11/DXBC: `vector_add.dxil` (פלט אמיתי של `dxc.exe -T cs_6_0`) מפוענח
מקצה לקצה (מיכל -> LLVM bitstream -> טבלת טיפוסים -> הוראות -> כל 7
רשומות `Call` מפורשות לפירוש `dx.op.*` אמיתי) ומתורגם ל-SPIR-V אמיתי
(`directx_shader_translate::translate_dxil_vector_add_to_spirv`), אותו
`tests/vector_add_dxil_real_vulkan.rs` שולח למכונה זו על ה-NVIDIA GT
730 האמיתי שלה ומאמת שהתוצאה תואמת מספרית לייחוס ה-CPU `a[i]+b[i]`.
זוהי עדיין צורת שיידר ידועה אחת בלבד, לא מפענח SM6.0 כללי — ראו
"אינו ממומש (היקף כן)" למטה לגבול המדויק. גודל קבוצת העבודה של SPIR-V
מופק כעת באמת מ-`METADATA_BLOCK` של DXIL (`dx.entryPoints` ->
`ShaderProperties` -> `NumThreads`), ולא קבוע (hardcoded) — ראו את
רשומת ה-HANDOFF "המשך 9" מ-2026-07-25 ב-`CLAUDE.md` לתיאור המלא,
ואת "המשך 7" להישג הפרוסה האנכית המקורי שפער ידוע זה נסגר בו.

## מצב נוכחי (2026-07-25, המשך: פענוח DXIL ברמת bitstream + פענוח DXBC של VS/PS ב-D3D11)

שני פיסות עבודה חדשות נחתו על גבי פרוסת ה-compute shader האנכית של
שלב 1 למטה:

- **DXIL (D3D12/SM6+) — נתונים בפועל פוענחו, רק ברמת container/bitstream.**
  `crates/directx-shader-translate/src/dxil.rs`
  (`parse_dxil_container`) מפענח מיכל DXBC אמיתי שקומפל עם
  `dxc.exe -T cs_6_0` (`shaders/vector_add_dxil.hlsl` ->
  `shaders/vector_add.dxil`, שהופק על ידי
  `tools/compile-dxbc-shaders.ps1`): מחלץ את `DxilProgramHeader`/
  `DxilBitcodeHeader` של ה-chunk `DXIL` (סוג שיידר, SM6.0, גרסת DXIL)
  דרך חבילת `dxbc` הקיימת, ואז מעביר את מטען ה-LLVM bitcode הגולמי
  לחבילת `llvm-bitcode` (תלות חדשה שנוספה, קורא LLVM bitstream כללי
  ללא ידע ספציפי ל-DXIL) כדי לפענח בפועל את עץ הבלוקים/הרשומות. אושר
  מול bytes אמיתיים: קסם עטיפת ה-LLVM `BC\xC0\xDE`, `MODULE_BLOCK`
  יחיד ברמה עליונה (id 8), ותת-בלוקי LLVM סטנדרטיים בתוכו —
  `TYPE_BLOCK_ID_NEW`(17), `PARAMATTR_GROUP_BLOCK`(10),
  `PARAMATTR_BLOCK`(9), `CONSTANTS_BLOCK`(11), `FUNCTION_BLOCK`(12, x5 —
  אחד לכל בלוק בסיסי של `main`), `VALUE_SYMTAB_BLOCK`(14),
  `METADATA_BLOCK`(15, x2). **עדכון (2026-07-25, המשך, מסלול D3D12)**:
  פענוח טבלת הטיפוסים ופענוח הוראות גס נוספו מאז
  (`resolve_type_table`/`decode_function_instructions` באותו קובץ),
  תוך יישום טבלאות הרשומות `TYPE_BLOCK`/`FUNC_CODE` המתועדות של LLVM על
  ה-bytes האמיתיים של `vector_add.dxil` — אושרה טבלת טיפוסים בת 22
  ערכים הכוללת `Float` ו-
  `StructNamed{"class.RWStructuredBuffer<float>"}`, ורצף הוראות אמיתי
  (`DeclareBlocks -> Call*5 -> ExtractValue -> Call -> ExtractValue ->
  BinOp -> Call -> Ret`). **עדכון (2026-07-25, המשך 6)**: כל 7 רשומות
  ה-`Call` מפורשות כעת. `resolve_vector_add_dxil_calls` מפענח שמות
  פונקציות מ-`VALUE_SYMTAB_BLOCK` (נמצא באמצעות
  `Record::take_payload()`, לא `fields()` — פער אמיתי בהבנת הרשומה
  הקודמת את החבילה) ומפענח ידנית את קידוד האופרנד היחסי-ערך של LLVM
  (אומת ידנית מול ה-bytes האמיתיים), ונותן
  `[CreateHandle{range_id:2}, CreateHandle{range_id:1},
  CreateHandle{range_id:0}, ThreadId, BufferLoad{handle_range_id:0},
  BufferLoad{handle_range_id:1}, BufferStore{handle_range_id:2}]`.
  מספרי אופקוד ה-DXIL (`CreateHandle`=57, `BufferLoad`=68,
  `BufferStore`=69, `ThreadId`=93) אושרו בחיפוש רשת מול
  `DirectXShaderCompiler/docs/DXIL.rst` של Microsoft, לא הונחו מהזיכרון,
  והתאימו בדיוק לקבועים המפוענחים האמיתיים. **עדיין אין תרגום
  DXIL-ל-SPIR-V** — זו התוספת הבאה. ראו "אינו ממומש" למטה.
- **צנרת גרפיקה של D3D11 — יצירת SPIR-V אמיתית ל-VS/PS הושגה ואומתה,
  עדיין ללא ראסטריזציה/ציור.** `shaders/triangle_vs.hlsl`/
  `shaders/triangle_ps.hlsl` (זוג vertex+pixel shader מינימלי של מעבר
  ישיר, `POSITION`/`COLOR` בכניסה, `SV_POSITION`/`SV_TARGET` ביציאה)
  קומפלו עם `fxc.exe /T vs_5_0`/`/T ps_5_0` אמיתי. `parse_dxbc` מפענח
  את שניהם ללא שינוי. `spirv_gen::translate_vertex_shader`/
  `translate_pixel_shader` (חדש) מפענחים את רצף אופקוד ה-SHEX האמיתי
  והקבוע (`dcl_input`x2/`dcl_output_siv`/`dcl_output`/`mov`x3/`ret`
  ל-VS; `dcl_input_ps`(linear)/`dcl_output`/`mov`/`ret` ל-PS) ומפיקים
  SPIR-V גרפי אמיתי: `OpEntryPoint Vertex`/`Fragment` (לא `GLCompute`),
  משתני storage class מסוג `Input`/`Output` עם דקורציות `Location`,
  `BuiltIn Position` על היציאה `SV_POSITION` של ה-vertex shader,
  ו-`OpExecutionMode ... OriginUpperLeft` על ה-fragment shader. אומת
  בשתי דרכים: (1) הטוען העצמי של `rspirv` מפענח מחדש את הבתים
  שהופקו ללא שגיאה, (2) `spirv-val.exe` האמיתי של Vulkan SDK
  (`C:\VulkanSDK\1.4.350.0\Bin\spirv-val.exe`) הורץ כנגד שני המודולים
  שהופקו והחזיר קוד יציאה 0 ללא אבחונים לשניהם.
  `translate_shader`/`translate_chain_shader` (compute בלבד) ממשיכים
  לדחות נכון את שני השיידרים. **אין ראסטריזטור, אין framebuffer, אין
  קריאת ציור Vulkan אמיתית** — אושר ש-`opencuda-vulkan` (על ידי קריאת
  הקוד המקור האמיתי שלו) לא כולל בכלל קוד
  `VkGraphicsPipelineCreateInfo`/render-pass/framebuffer, רק ריצת
  compute, כך שפיקסל שצויר בפועל מחוץ להיקף בשלב זה. ראו את ה-HANDOFF
  ב-`CLAUDE.md` לגבול האבן-דרך הכן.

## מצב נוכחי (2026-07-26, אבן דרך צנרת גרפיקה מינימלית של D3D11 הושגה)

חבילה חדשה `crates/directx-graphics-vulkan` מוסיפה את `ash` כתלות
**ישירה** של מרחב עבודה זה (לא בשכבה על גבי `opencuda-vulkan`, שאושר
בביקורת קוד מקור כמשמש לריצת compute בלבד). היא מממשת render pass,
framebuffer, ו-`VkGraphicsPipelineCreateInfo` אמיתיים, תוך שימוש חוזר
ב-SPIR-V שכבר נוצר ועובר את `spirv-val` מ-`translate_vertex_shader`/
`translate_pixel_shader` למעלה (לא מיושם מחדש תרגום שיידר).
`render_uniform_triangle_and_read_back` מצייר משולש "גדול" אחד המכסה
את כל ה-viewport עם צבע קודקוד אחיד יחיד, קורא את התמונה שצוירה חזרה
דרך host-visible staging buffer, ובדיקת החומרה האמיתית
(`crates/directx-graphics-vulkan/tests/triangle_real_vulkan.rs`) מוודאת
שכל פיקסל שנקרא חזרה תואם את צבע הקודקוד המועבר על ה-NVIDIA GT 730
האמיתי הקיים במכונה זו (`cargo test -p directx-graphics-vulkan --test
triangle_real_vulkan -- --nocapture`: 1 עבר). ההיקף מכוון להיות צר:
זוג שיידרים קבוע אחד, קריאת ציור אחת, ללא depth buffer/טקסטורות/
swapchain/בדיקת אינטרפולציה מרובת-משולשים. ראו את ה-HANDOFF
ב-`CLAUDE.md` (המשך 2026-07-26) לגילוי הכן המלא.

## מצב נוכחי (2026-07-25, פרוסה אנכית של שלב 1 הוכללה לשלושה שיידרים ידועים)

`crates/directx-shader-translate` מבצע כעת את הפרוסה האנכית המלאה
**עבור שלושה שיידרים ידועים ספציפיים** (`vector_add.hlsl`,
`vector_mul.hlsl`, `vector_sub_bounded.hlsl`): פענוח DXBC -> פענוח
תת-קבוצת אופקודים צרה של SM5.0 -> יצירת SPIR-V (דרך `rspirv`) ->
ריצת Vulkan אמיתית (`opencuda-vulkan` של `open-cuda`) -> התאמה מספרית
לייחוס CPU, אומת על ה-NVIDIA GT 730 האמיתי של מכונה זו. **זהו עדיין
לא מפענח SM5.0-ל-SPIR-V כללי** — ראו "אינו ממומש" למטה.

- `parse_dxbc` (שלב 0): בדיקת מיכל/chunk של DXBC (נוכחות RDEF/ISGN/
  OSGN/SHEX), ללא שינוי מה-front-end המקורי.
- `spirv_gen::translate_shader` (שלב 1, הוכלל ב-2026-07-25): מזהה 3
  צורות אופקוד המופקות בפועל על ידי `fxc.exe`, כולן חולקות שלד משותף
  (`dcl_globalFlags` -> `dcl_constantbuffer` אופציונלי -> 3x
  `dcl_uav_structured` -> `dcl_input` -> `dcl_temps` ->
  `dcl_thread_group` -> `ult`+`if` אופציונלי -> 2x `ld_structured` ->
  `add`/`mul` -> `store_structured` -> `endif` אופציונלי -> `ret`):
  - `vector_add.hlsl`: `add`, ללא בדיקת גבולות.
  - `vector_mul.hlsl`: `mul` במקום `add`.
  - `vector_sub_bounded.hlsl`: `add` עם דגל `negate` על אופרנד המקור
    הראשון שלו (אושר על ידי dump של פלט `fxc.exe` אמיתי — `fxc`
    מבצע אופטימיזציה ל-`a - b` והופך אותו ל-`add dest, -b, a` במקום
    להפיק אופקוד `sub` ייעודי), בתוספת בדיקת גבולות אמיתית
    `if (id.x < N)` (`ult` מול constant buffer + `if`/`endif`), אותה
    ה-SPIR-V שהופק ממש עם `OpSelectionMerge`/`OpBranchConditional`
    אמיתי, תוך שימוש ב-push-constant `n` להשוואה.
  כל אופקוד/צורה אחרים נדחים באמצעות `SpirvGenError::UnsupportedShader`
  ולא מתורגמים בשקט באופן שגוי. נקודות קישור UAV, גודל קבוצת
  ה-threads, האופרטור, ונוכחות בדיקת הגבולות — כולם מחולצים מה-DXBC
  המפוענח האמיתי, לא קבועים בקוד. `translate_vector_add_shader` נשמר
  כ-alias דק לתאימות לאחור ל-`translate_shader`.
- `tests/vector_add_real_vulkan.rs`, `tests/vector_mul_real_vulkan.rs`,
  `tests/vector_sub_bounded_real_vulkan.rs`: כל אחד שולח את ה-SPIR-V
  המתורגם שלו דרך `opencuda-vulkan::VulkanDevice` האמיתי של `open-cuda`
  (`ash`, feature `real-vulkan`) ובודק את פלט ה-GPU כנגד ייחוס CPU
  עבור 256 איברים (סף 1e-3/1e-2). בדיקת בדיקת-הגבולות שולחת בנוסף 320
  threads עם מספר איברים לוגי של 256 ומוודאת שהאיברים 256..320 לעולם
  אינם נכתבים (נשארים בערך sentinel), ומוכיחה שהענף
  `if (id.x < N)` ב-SPIR-V שנוצר אכן שולט בפועל בביצוע ולא רק מתקמפל.
- `examples/dump_shex.rs`: כלי עצמאי קטן
  (`cargo run -p directx-shader-translate --example dump_shex -- <path.dxbc>`)
  ששימש בסשן זה כדי לבדוק זרמי אופקוד SHEX אמיתיים לפני כתיבת תמיכת
  מפענח עבורם; נשמר לעבודת הכללה מקיפה יותר לפי אופקוד בעתיד.

**מאז שכותרת סעיף זה נכתבה**, נוסף שיידר יחיד-פעולה רביעי
(`vector_div.hlsl`, `div` פשוט) ל-`translate_shader` בעקבות אותו דפוס
בדיוק, ו — לאחרונה יותר — מחלקת דפוס שונה בפועל,
`spirv_gen::translate_chain_shader`, נוספה לצידו (לא במקומו): הוא
מפענח עץ ביטוי-רגיסטר אמיתי של פעולות בינאריות רצופות (חיבור/כפל, ללא
זרימת בקרה) במקום פעולה קבועה יחידה, אומת מול שיידר שקומפל מחדש
ש-SHEX האמיתי שלו התברר כמשתמש חוזר ברכיבי רגיסטר זמני אחד דרך CSE של
fxc במקום להצהיר רגיסטרים זמניים נוספים. ראו את רשומת ה-HANDOFF
"המשך 9" מ-2026-07-25 ב-`CLAUDE.md` לתיאור המלא והעדכני (סעיף זה
נותר כפי שנכתב במקור לצורך דיוק היסטורי לגבי מצב אמצע-יום ה-2026-07-25).

## בנייה ובדיקה

```powershell
cargo build --workspace
cargo test --workspace -- --nocapture
```

### לראות את זה מצייר משהו בפועל (נוסף ב-2026-07-27)

מאגר זה הוא אוסף ספריות ללא `fn main` משלו, כך שהדרך המהירה ביותר
*לראות* את צנרת הגרפיקה עובדת על ה-GPU שלך — במקום לקרוא קוד מקור
בדיקות — היא:

```bash
cargo run -p directx-graphics-vulkan --example render_triangle
```

זה עושה שימוש חוזר באותם שיידרים מתורגמים DXBC → SPIR-V שקומפלו עם
fxc.exe אמיתי כמו `tests/triangle_real_vulkan.rs`, מצייר משולש גרדיאנט
(אדום/ירוק/כחול) על חומרת Vulkan אמיתית, קורא את ה-framebuffer חזרה,
וכותב אותו ל-`render_triangle.ppm` (PPM פשוט, ללא תלות נוספת בחבילת
תמונות — ניתן להמיר אותו למשל עם `magick render_triangle.ppm
render_triangle.png` או לפתוח אותו ישירות ברוב מציגי התמונות). אם אין
מכשיר/דרייבר Vulkan שמיש, הוא מדפיס שגיאה כנה ויוצא עם קוד שאינו
אפס במקום לזייף הצלחה.

פלט שנצפה בפועל (2026-07-25, מכונה זו, NVIDIA GeForce GT 730):

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

`cargo clippy --workspace --all-targets`: 0 אזהרות.

לאחר עבודת פענוח טבלת הטיפוסים/ההוראות של DXIL (2026-07-25, המשך,
מסלול D3D12), `cargo test --workspace` מריץ 23 בדיקות בסך הכול (19
בדיקות יחידה + 4 בדיקות אינטגרציה של Vulkan אמיתי), כולן עוברות,
כולל 3 חדשות על גבי ה-20 הקודמות: `dxil::tests::resolves_real_dxil_
type_table_and_finds_float_and_resource_struct`, `dxil::tests::decodes_
real_dxil_function_block_into_matching_vector_add_shape`, ו-`dxil::
tests::shape_matcher_honestly_rejects_unexpected_instruction_orderings`.

כדי לחדש (regenerate) את קבצי הבדיקה (fixtures) של DXBC מ-HLSL (דורש
את `fxc.exe` של ה-Windows SDK — שימו לב ש-`dxc.exe` מכוון רק ל-
DXIL/SM6+ ולא יכול להפיק DXBC):

```powershell
pwsh tools/compile-dxbc-shaders.ps1
```

## אינו ממומש (היקף כן)

- **פענוח הוראות SM5.0 כללי.** רק 3 צורות האופקוד למעלה מטופלות; כל
  compute shader אחר של D3D11 (פריסת משאבים שונה, זרימת בקרה אחרת,
  אינטרינזיקות אחרות, יותר מבדיקת גבולות אחת, `div`/`sub` כאופקוד
  אמיתי במקום `add` שלילי, וכו') נדחה, לא מתורגם באופן שגוי. בניית
  מפענח כללי אמיתי (או אימוץ/portaportation של אחד קיים, למשל לימוד
  מעמיק יותר של הגישה של `dxbc-spirv`/`dxil-spirv`) נשארת אבן הדרך
  האמיתית הבאה.
- **DXIL (Shader Model 6+, D3D12): פרוסת ה-`vector_add.dxil` הושלמה
  כעת מקצה לקצה, על חומרה אמיתית — אבל עדיין רק לצורת שיידר ידועה זו,
  לא ל-SM6.0 כללי.** `resolve_type_table`/`decode_function_instructions`/
  `resolve_vector_add_dxil_calls` ב-`dxil.rs` מפענחים רשומות `TYPE_BLOCK`/
  `FUNCTION_BLOCK`/`VALUE_SYMTAB_BLOCK` אמיתיות כנגד קודים מתועדים של
  LLVM ומפרשים את כל 7 רשומות ה-`Call` למשמעות `dx.op.*` האמיתית שלהן
  (`CreateHandle`/`ThreadId`/`BufferLoad`/`BufferStore`, עם נקודות
  קישור UAV). `translate_dxil_vector_add_to_spirv` (חדש) מזין את
  הפלט המפוענח הזה ל-`emit_spirv_for_kernel` המשותף של `spirv_gen.rs`
  (הופרד מ-`emit_spirv` של נתיב DXBC כך ששני ה-backends מפיקים
  ממסלול קוד אחד) כדי להפיק SPIR-V אמיתי, אותו `tests/vector_add_
  dxil_real_vulkan.rs` שולח למכונה זו על ה-NVIDIA GT 730 האמיתי שלה
  דרך `opencuda-vulkan` ומאמת שהוא תואם לייחוס CPU `a[i]+b[i]` עבור
  כל 256 האיברים — אותה קפדנות כמו בדיקת ה-`vector_add` של DXBC.
  **גודל קבוצת ה-threads מופק כעת בפועל, לא קבוע**:
  `extract_numthreads_from_metadata` (`dxil.rs`) עוקב אחר נתיב
  `METADATA_BLOCK` האמיתי `dx.entryPoints` -> tuple לכל entry point ->
  `ShaderProperties` -> `kDxilNumThreadsTag` (=4, אושר מול קוד המקור
  של `DxilMetadataHelper.h`/`.cpp` של Microsoft `DirectXShaderCompiler`)
  ומפענח את הצומת `{x,y,z}` מול רשימת הערכים האמיתית של המודול, ונותן
  `(64,1,1)` מה-bytes האמיתיים של `vector_add.dxil` — הקוד הקבוע
  הידוע מהרשומה הקודמת נסגר, ובדיקת רגרסיה סינתטית מוכיחה שלוגיקת
  החילוץ מחזירה ערך *שונה* כאשר ניתן לה metadata שונה (לא רק "מחזיר
  64,1,1 בכל מקרה"). כל אופקוד/צורת אופרנד אחרים (פעולה שונה, מספר
  בלוקים בסיסיים, בדיקות גבולות) עדיין נדחים, לא מתורגמים באופן שגוי.
  תמיכת רשימת פקודות D3D12/ערמת descriptor/root signature (השכבה
  מעל תרגום השיידר) לא נגעת בה.
- **מפענח DXBC הוכלל מעבר ל-4 צורות פעולה-יחידה קבועות: מטפל כעת
  בשרשראות של פעולות בינאריות רצופות (ללא זרימת בקרה) דרך עץ
  ביטוי-רגיסטר אמיתי, לא צורה חמישית קשיחה.**
  `spirv_gen::translate_chain_shader`/`decode_chain_shape` עוברים על
  `ld_structured`/`add`/`mul`/`store_structured` ובונים עץ ביטוי אמיתי
  ממופתח על ידי (רגיסטר זמני, רכיב), כך שהוא מטפל ב-1 פעולה, 2
  פעולות, או N פעולות באותה דרך — אומת מול שיידר אמיתי שקומפל מחדש
  (`vector_add_mul_chain.hlsl`, `t = A[i]+B[i]; Out[i] = t*A[i]`) שה-
  SHEX האמיתי שלו התברר כמשתמש חוזר ברכיבי `.x`/`.y` של רגיסטר זמני
  יחיד (fxc ביצע CSE וסילק את הטעינה החוזרת של `A[i]` במקום להנפיק
  שוב `ld_structured`) — ממצא אמיתי, לא צפוי, שהמפענח מבוסס העץ מטפל
  בו ללא מקרים נוספים. נשלח ואומת על ה-NVIDIA GT 730 האמיתי כנגד
  ייחוס ה-CPU `(a[i]+b[i])*a[i]`. `sub`/`div` בתוך שרשרת נדחים
  במכוון עדיין (סמנטיקת סדר האופרנדים שלהם אומתה רק עבור המקרה
  יחיד-הפעולה). 4 צורות יחיד-הפעולה המקוריות לא נגעו בהן וממשיכות
  לעבור ללא שינוי.
- **צנרת גרפיקה של D3D11: פענוח מיכל DXBC אושר כעובד עבור VS/PS, אך
  אין יצירת קוד SPIR-V, אין ראסטריזטור, אין משולש שמצוייר בפועל על
  המסך.** הצנרת המלאה (ראסטריזטור, דגימת טקסטורות, מצב blend,
  output-merger) נשארת מחוץ להיקף; כך גם הרחבת המפענח בעל צורת
  האופקוד הצרה של `spirv_gen` כדי להבין `dcl_output_siv`/
  `dcl_input_ps`/מצבי אינטרפולציה.
- יעדי משפחת PlayStation — מחוץ להיקף במפורש; ראו `CLAUDE.md` לנימוק
  המשפטי/תנאי השירות.

## פרויקטים קשורים

- [open-cuda](https://github.com/aon-co-jp/open-cuda) — ה-backend
  לביצוע compute של Vulkan שפרויקט זה מיועד לשלוח דרכו
  (`opencuda-core::GpuDevice`, `KernelSource::SpirV`). מכיל גם חבילת
  `opencuda-directx` לא-קשורה שכבר עובדת, המריצה D3D12 **באופן ילידי
  על Windows** — הכיוון ההפוך מפרויקט זה (שמריץ שיידרים של DirectX
  **על יעדים שאינם Windows**).
- [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm) — אין תלות
  טכנית ישירה בפרויקט זה (אומת, לא הונח כמובן מאליו).
