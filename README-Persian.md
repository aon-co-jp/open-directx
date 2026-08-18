# open-directx

> 📌 **به‌روزرسانی اخیر (2026-08-08، نمونه اولیه رندر اسپرایت دو‌بعدی)**:
> در پاسخ به پیشنهاد کاربر «توسعه نمونه‌های اولیه بازی/ماینینگ/LLM روی
> dream-os/Linux از طریق open-directx برای رایانه‌ی GT730»، کار ابتدا با
> تمرکز بر رندر اسپرایت دو‌بعدی آغاز شد. پیاده‌سازی اولیه‌ی نمونه‌برداری
> بافت (`Texture2D.Sample`) → پشتیبانی از چند اسپرایت/شیت اسپرایت →
> حلقه‌ی بازی (به‌روزرسانی موقعیت + فیزیک برخورد) → **پنجره‌ی واقعی +
> زنجیره‌ی تعویض (swapchain) واقعی Vulkan + ورودی صفحه‌کلید واقعی**
> (کریت جدید `directx-graphics-window`، کاربر خودش آن را اجرا کرد و با
> چشمان خود «پارو حرکت کرد و توپ را برگرداند» را تأیید کرد) → ترکیب آلفا
> (اسپرایت‌های نیمه‌شفاف) → بارگذاری بافت از فایل‌های واقعی PNG، که همه‌ی
> این افزایش‌های تدریجی روی سخت‌افزار واقعی Windows (NVIDIA GT 730) و
> برخی نیز روی سخت‌افزار واقعی Linux (WSL2 Ubuntu) بررسی شدند. گزینه‌های
> بعدی: چند اسپرایت متحرک همراه با تشخیص برخورد، پشتیبانی از تغییر اندازه‌ی
> پنجره. برای جزئیات به [CLAUDE.md](CLAUDE.md) مراجعه کنید.
>
> *English*: Following the user's proposal to build game/mining/LLM
> prototypes for the GT 730 via open-directx on dream-os/Linux, started
> narrowly with a 2D sprite-rendering prototype: first texture sampling
> support, then multi-sprite/sprite-sheet support, a game loop
> (position update + bounce physics), a **real window + real Vulkan
> swapchain + real keyboard input** (new `directx-graphics-window`
> crate — the user ran it themselves and confirmed "the paddle moved
> and hit the ball back"), alpha blending, and loading textures from
> real PNG files. All verified on real Windows hardware (NVIDIA GT 730),
> some also on real Linux hardware (WSL2 Ubuntu). Next candidates:
> multiple moving sprites with collision detection, window resize
> support. See [CLAUDE.md](CLAUDE.md) for details.

> 📌 وظیفه‌ی معلق (2026-08-06): طرحی برای گنجاندن فناوری SBM توشیبا و
> فناوری DeepSeek وجود دارد (هدف: 8 مخزن از جمله dream-os). برای جزئیات
> به [CLAUDE.md](CLAUDE.md) مراجعه کنید.

> 📌 **به‌روزرسانی اخیر (2026-08-08)**: عدم‌تقارن مربوط به زنجیره‌ی
> هفت‌جمله‌ای دارای بررسی مرز، که فقط در سمت DXBC وجود داشت و در سمت DXIL
> وجود نداشت، برطرف شد — فایل جدید
> `vector_add_mul_div_sub_add_mul_div_chain7_bounded_dxil.hlsl` واقعاً با
> `dxc.exe` کامپایل شد و روی سخت‌افزار واقعی NVIDIA GT730 تطابق عددی با
> پیاده‌سازی مرجع CPU و عملکرد صحیح بررسی مرز تأیید شد (کل تست‌های واحد
> فضای کاری: 50 مورد + 22 تست سخت‌افزار واقعی، همگی سبز، صفر هشدار).
> برای جزئیات به [CLAUDE.md](CLAUDE.md) مراجعه کنید.
>
> *English*: Closed the DXBC-only vs. DXIL-missing asymmetry for the
> boundary-checked 7-term chain — added and real-`dxc.exe`-compiled
> `vector_add_mul_div_sub_add_mul_div_chain7_bounded_dxil.hlsl`, verified
> on real NVIDIA GT730 hardware (matches the CPU reference, boundary
> check confirmed; full workspace: 50 unit tests + 22 real-hardware
> tests all green, zero warnings). See [CLAUDE.md](CLAUDE.md) for
> details.

> 📌 **به‌روزرسانی اخیر (2026-08-07)**: زنجیره‌ی DXBC/DXIL دارای بررسی
> مرز به 6 جمله گسترش یافت و روی سخت‌افزار واقعی NVIDIA GT730 عملکرد آن
> تأیید شد. همکاری تقویت‌شده با dream-os/open-cuda/aruaru-llm (انتقال
> SBM/DeepSeek و غیره) بررسی شد، اما تصمیم گرفته شد که گسترش منطق تولید
> زنجیره‌ی DXBC/DXIL موجود بدون درک عمیق آن، ریسک نادیده‌گرفتن خطاهای
> عددی را در پی دارد، بنابراین هیچ تغییری در کد اعمال نشد و نتایج بررسی
> با صداقت در [CLAUDE.md](CLAUDE.md) ثبت شد.
>
> *English*: Extended the boundary-checked DXBC/DXIL chain to 6 terms,
> verified on real NVIDIA GT730 hardware. Investigated deeper
> integration with dream-os/open-cuda/aruaru-llm (SBM/DeepSeek
> transplant) but decided against guessing extensions to the DXBC/DXIL
> chain logic without deep domain understanding — no code changed there,
> findings honestly recorded in [CLAUDE.md](CLAUDE.md).

> **به‌روزرسانی 2026-07-25**: عنوان فایل خط‌مشی توسعه (`CLAUDE.md`) از
> «خط‌مشی توسعه و قواعد محیط توسعه» به «فلسفه‌ی طراحی و خط‌مشی توسعه و
> قواعد محیط توسعه» تغییر نام یافت تا فلسفه‌ی طراحی پروژه (آنچه ارزش
> می‌گذاریم)، خط‌مشی توسعه (چگونگی کار ما) و قواعد محیط توسعه (قراردادهای
> عملیاتی مشخص) را واضح‌تر از هم جدا کند. برای جزئیات به `CLAUDE.md`
> مراجعه کنید.


یک لایه‌ی سازگاری DirectX (D3D9/10/11/12) چندسکویی — با روحیه‌ای مشابه
DXVK / vkd3d-proton — که هدفش اجرای برنامه‌های DirectX ویندوز بدون تغییر
روی لینوکس (و در آینده اندروید/macOS) با ترجمه‌ی بایت‌کد شیدر DXBC/DXIL
به SPIR-V و ارسال آن از طریق بک‌اند موجود Vulkan compute
([open-cuda](https://github.com/aon-co-jp/open-cuda)، یعنی
`opencuda-vulkan`) است.

برای منطق کامل طراحی، دامنه/نقشه‌ی راه با صداقت کامل، و گزارش نشست‌های
HANDOFF به [`CLAUDE.md`](CLAUDE.md) مراجعه کنید — این README تنها خلاصه‌ای
از وضعیت فعلی و تأییدشده است.

### جدول پشتیبانی پلتفرم و سازنده (افزوده‌شده در 2026-07-27، افشای صادقانه)

خودِ DirectX یک API مخصوص ویندوز/ایکس‌باکس است — «چندسکویی بودن» در اینجا
یعنی بایت‌کد DXBC/DXIL به SPIR-V ترجمه شده و از طریق Vulkan ارسال می‌شود،
که همان چیزی است که واقعاً به پلتفرم‌های غیر ویندوز می‌رسد. امروز هیچ
`cfg(windows)` یا محدودیت پلتفرمی دیگری در کد خود این مخزن وجود ندارد
(پارسر DXBC، تولید کد SPIR-V، و `directx-graphics-vulkan` همگی Rust +
`ash` ساده و بدون وابستگی به پلتفرم هستند)، پس قابلیت ساخت/تست بر
اساس دسترسی خود Vulkan تعیین می‌شود:

| پلتفرم | مسیر | وضعیت |
|---|---|---|
| ویندوز | Vulkan بومی | **روی سخت‌افزار واقعی تأیید شده** (ماشین توسعه‌ی این مخزن، NVIDIA GeForce GT 730) |
| لینوکس | Vulkan بومی | باید بدون تغییر ساخته/اجرا شود (هیچ کد مخصوص ویندوز برای مسدود کردنش وجود ندارد) — **هنوز روی یک ماشین واقعی لینوکس در این محیط تست نشده** |
| اندروید | Vulkan بومی | `open-cuda` تأیید کرده که کراس‌کامپایل `aarch64-linux-android` موفق است (طبق CLAUDE.md آن)؛ اجرا روی دستگاه واقعی (`vkCreateInstance` روی یک گوشی واقعی) همچنان معلق است |
| macOS | Vulkan از طریق [MoltenVK](https://github.com/KhronosGroup/MoltenVK) (ترجمه به Metal) | هنوز امتحان نشده — MoltenVK یک لایه‌ی ترجمه است، نه Vulkan بومی، پس این ضمانتی ضعیف‌تر از لینوکس/اندروید است |
| iOS / iPadOS (افزوده‌شده در 2026-08-17) | Vulkan از طریق MoltenVK (ترجمه به Metal) | هنوز امتحان نشده. **همان محدودیت MoltenVK مربوط به macOS اینجا هم صدق می‌کند** — Vulkan به‌طور بومی روی iOS/iPadOS اجرا نمی‌شود، فقط از طریق این لایه‌ی ترجمه، بنابراین برابری با مسیر بومی ویندوز/Vulkan تا زمان امتحان واقعی روی دستگاه تضمین نیست. همچنین برای توزیع رسمی نیازمند عضویت در Apple Developer Program است. |
| انواع یونیکس/BSD (افزوده‌شده در 2026-08-17) | احتمالاً Vulkan بومی | بررسی‌نشده — پشتیبانی Vulkan بسته به توزیع/درایور متفاوت است؛ انتظار می‌رود پس از بررسی، بیشتر مسیر لینوکس قابل استفاده مجدد باشد |
| سونی PlayStation 4/5/6/7 | ندارد | فعلاً به‌طور صریح خارج از دامنه — به یادداشت «اهداف خانواده‌ی PlayStation» در پایین و `CLAUDE.md` مراجعه کنید |
| Nintendo Switch 2 / 3 (افزوده‌شده در 2026-08-17) | ندارد | همان وضعیت «آرزوی آینده، به تعویق افتاده تا SDK/NDA رسمی» مانند PlayStation. **Switch 3 تا تاریخ 2026-08-17 به‌طور رسمی از سوی نینتندو اعلام نشده — گنجاندن آن اینجا فقط یک جانگهدار برای زمان احتمالی اعلام است، نه بر اساس اطلاعات واقعی محصول.** |

پوشش سازنده‌ی GPU (تطابق شناسه‌ی سازنده‌ی PCI، هماهنگ در این مخزن و
`open-cuda`: NVIDIA `0x10DE`، AMD `0x1002`/`0x1022`، Intel `0x8086`):

| سازنده | وضعیت |
|---|---|
| NVIDIA | **روی سخت‌افزار واقعی تأیید شده** (GeForce GT 730) |
| AMD | کد تطابق شناسه‌ی سازنده وجود دارد و از نظر نوع بررسی می‌شود، اما **هرگز روی سخت‌افزار واقعی AMD در این محیط اجرا نشده** — تأییدنشده تلقی کنید |
| Intel | مشابه AMD: کد وجود دارد، **هرگز روی سخت‌افزار واقعی GPU اینتل تأیید نشده** |

برای قابل تشخیص‌بودن این سه شناسه‌ی سازنده نیازی به هیچ رفعِ اشکالی
نیست — کد از قبل درست است و در `open-directx`/`opencuda-vulkan`/
`opencuda-directx` یکسان است. آنچه کم است، سخت‌افزار واقعی AMD/Intel
برای واقعاً اجرا کردن این مسیر کد است، که این محیط توسعه فاقد آن است.

## وضعیت فعلی (2026-07-27، آخرین: میان‌یابی گرادیان، تشخیص سازنده‌ی GPU، زنجیره‌ی تفریق/تقسیم)

سه پیشرفت بر پایه‌ی خط لوله‌ی گرافیکی حداقلی D3D11 و کار رده‌ی زنجیره‌ی
DXBC زیر افزوده شد، همگی روی NVIDIA GT 730 واقعی این ماشین تأیید شد:
(1) `render_gradient_triangle_and_read_back` — خط لوله‌ی گرافیکی اکنون
می‌تواند رنگ مجزا برای هر رأس اختصاص دهد (نه فقط حالت تباهیده‌ی رنگ
یکنواخت)، از طریق بررسی ناوردای «افراز واحد» روی پیکسل‌های خوانده‌شده
از سخت‌افزار واقعی تأیید شده است. (2) `enumerate_graphics_devices()` —
شکاف تشخیصی را می‌بندد که در آن مسیر Compute در `open-cuda` تشخیص شناسه‌ی
سازنده داشت اما مسیر Graphics اینجا نداشت؛ مستقل است، بدون وابستگی جدید
به `opencuda-vulkan`. (3) `decode_chain_shape` اکنون از `sub`/`div`
پشتیبانی می‌کند (پیش‌تر به‌صراحت به دلیل قابل‌تأیید‌نبودن رد می‌شد) — یک
شیدر جدید (`vector_sub_div_chain.hlsl`) واقعاً با `fxc.exe` کامپایل شد و
خروجی SHEX آن برای تأیید ترتیب دقیق عملوندها استفاده شد، سپس به‌صورت
سرتاسری در برابر مرجع CPU روی سخت‌افزار واقعی تأیید شد. برای شرح کامل به
ورودی‌های HANDOFF مربوط به 2026-07-27 در `CLAUDE.md` مراجعه کنید.

## وضعیت فعلی (2026-07-25، آخرین: برش عمودی DXIL کامل روی سخت‌افزار واقعی)

برش عمودی شیدر compute برای D3D12/DXIL اکنون به برابری کامل با نمونه‌ی
D3D11/DXBC رسیده است: `vector_add.dxil` (خروجی واقعی `dxc.exe -T cs_6_0`)
به‌طور سرتاسری رمزگشایی می‌شود (کانتینر -> جریان بیت LLVM -> جدول نوع ->
دستورالعمل‌ها -> هر 7 رکورد `Call` که به معنای واقعی `dx.op.*` تفکیک
شده‌اند) و به SPIR-V واقعی ترجمه می‌شود
(`directx_shader_translate::translate_dxil_vector_add_to_spirv`)، که
`tests/vector_add_dxil_real_vulkan.rs` آن را روی NVIDIA GT 730 واقعی این
ماشین ارسال می‌کند و تطابق عددی با مرجع CPU (`a[i]+b[i]`) را تأیید
می‌کند. این هنوز فقط یک شکل شناخته‌شده از شیدر است، نه یک رمزگشای عمومی
SM6.0 — برای مرز دقیق به بخش «پیاده‌سازی‌نشده (دامنه‌ی صادقانه)» در
پایین مراجعه کنید. اندازه‌ی گروه کاری SPIR-V اکنون واقعاً از
`METADATA_BLOCK` (`dx.entryPoints` -> `ShaderProperties` -> `NumThreads`)
استخراج می‌شود، نه ثابت کدشده — برای شرح کامل به ورودی HANDOFF «ادامه 9»
مورخ 2026-07-25 در `CLAUDE.md` و «ادامه 7» برای دستاورد اصلی برش عمودی
که این شکاف شناخته‌شده را بست مراجعه کنید.

## وضعیت فعلی (2026-07-25، ادامه: تجزیه‌ی سطح جریان بیت DXIL + تجزیه‌ی DXBC ورتکس/پیکسل D3D11)

دو کار جدید بر پایه‌ی برش عمودی شیدر compute فاز 1 زیر افزوده شد:

- **DXIL (D3D12/SM6+) — بایت‌های واقعی تجزیه شده، تنها در سطح
  کانتینر/جریان بیت.** `crates/directx-shader-translate/src/dxil.rs`
  (`parse_dxil_container`) یک کانتینر واقعی DXBC کامپایل‌شده با
  `dxc.exe -T cs_6_0` را تجزیه می‌کند (`shaders/vector_add_dxil.hlsl` ->
  `shaders/vector_add.dxil`، تولیدشده توسط
  `tools/compile-dxbc-shaders.ps1`): سرآیند
  `DxilProgramHeader`/`DxilBitcodeHeader` تکه‌ی `DXIL` (نوع شیدر، SM6.0،
  نسخه‌ی DXIL) را از طریق کریت موجود `dxbc` استخراج می‌کند، سپس بار خام
  bitcode LLVM را به کریت `llvm-bitcode` (وابستگی جدید افزوده‌شده، خواننده‌ی
  عمومی جریان بیت LLVM بدون دانش خاص DXIL) می‌سپارد تا درخت
  بلوک/رکورد را واقعاً رمزگشایی کند. در برابر بایت‌های واقعی تأیید شده:
  علامت جادویی رَپر LLVM `BC\xC0\xDE`، یک `MODULE_BLOCK` سطح بالای منفرد
  (شناسه 8)، و زیربلوک‌های استاندارد LLVM داخل آن —
  `TYPE_BLOCK_ID_NEW`(17)، `PARAMATTR_GROUP_BLOCK`(10)،
  `PARAMATTR_BLOCK`(9)، `CONSTANTS_BLOCK`(11)، `FUNCTION_BLOCK`(12، ×5 —
  یکی برای هر بلوک پایه‌ی `main`)، `VALUE_SYMTAB_BLOCK`(14)،
  `METADATA_BLOCK`(15، ×2). **به‌روزرسانی (2026-07-25، ادامه، مسیر
  D3D12)**: از آن پس تفکیک جدول نوع و رمزگشایی درشت‌دانه‌ی دستورالعمل
  افزوده شده است (`resolve_type_table`/`decode_function_instructions` در
  همان فایل)، با اعمال جداول رکورد مستند LLVM `TYPE_BLOCK`/`FUNC_CODE` بر
  بایت‌های واقعی `vector_add.dxil` — یک جدول نوع ۲۲مدخلی شامل `Float` و
  `StructNamed{"class.RWStructuredBuffer<float>"}` تأیید شد، و یک دنباله‌ی
  واقعی از دستورالعمل‌ها (`DeclareBlocks -> Call*5 -> ExtractValue -> Call
  -> ExtractValue -> BinOp -> Call -> Ret`). **به‌روزرسانی (2026-07-25،
  ادامه 6)**: اکنون همه‌ی 7 رکورد `Call` تفکیک شده‌اند.
  `resolve_vector_add_dxil_calls` نام‌های تابع `VALUE_SYMTAB_BLOCK` را
  حل می‌کند (با `Record::take_payload()` پیدا شده، نه `fields()` — شکاف
  واقعی در درک ورودی قبلی از کریت) و رمزگذاری عملوند مقدار نسبی LLVM را
  دستی رمزگشایی می‌کند (با بررسی دستی در برابر بایت‌های واقعی تأیید شده)،
  که به `[CreateHandle{range_id:2}, CreateHandle{range_id:1},
  CreateHandle{range_id:0}, ThreadId, BufferLoad{handle_range_id:0},
  BufferLoad{handle_range_id:1}, BufferStore{handle_range_id:2}]` منتهی
  می‌شود. شماره‌های اپکد DXIL (`CreateHandle`=57، `BufferLoad`=68،
  `BufferStore`=69، `ThreadId`=93) از طریق جست‌وجوی وب در برابر
  `DirectXShaderCompiler/docs/DXIL.rst` مایکروسافت تأیید شدند، نه فرض بر
  اساس حافظه، و دقیقاً با ثابت‌های رمزگشایی‌شده‌ی واقعی مطابقت داشتند.
  **همچنان هیچ ترجمه‌ی DXIL به SPIR-V وجود ندارد** — که افزایش بعدی
  خواهد بود. به «پیاده‌سازی‌نشده» در پایین مراجعه کنید.
- **خط لوله‌ی گرافیکی D3D11 — تولید واقعی SPIR-V برای VS/PS رسیده و
  تأیید شده، هنوز بدون رسترایزر/رسم.** `shaders/triangle_vs.hlsl`/
  `shaders/triangle_ps.hlsl` (جفت شیدر ورتکس+پیکسل عبوری حداقلی، ورودی
  `POSITION`/`COLOR`، خروجی `SV_POSITION`/`SV_TARGET`) با
  `fxc.exe /T vs_5_0`/`/T ps_5_0` واقعی کامپایل شد. `parse_dxbc` هر دو را
  بدون تغییر تجزیه می‌کند. `spirv_gen::translate_vertex_shader`/
  `translate_pixel_shader` (جدید) دنباله‌ی ثابت اپکد SHEX واقعی را
  رمزگشایی می‌کنند (`dcl_input`×2/`dcl_output_siv`/`dcl_output`/`mov`×3/`ret`
  برای VS؛ `dcl_input_ps`(خطی)/`dcl_output`/`mov`/`ret` برای PS) و SPIR-V
  گرافیکی واقعی تولید می‌کنند: `OpEntryPoint Vertex`/`Fragment` (نه
  `GLCompute`)، متغیرهای کلاس ذخیره‌سازی `Input`/`Output` با تزئین
  `Location`، تزئین `BuiltIn Position` روی خروجی `SV_POSITION` شیدر
  ورتکس، و `OpExecutionMode ... OriginUpperLeft` روی شیدر فرگمنت. به دو
  روش تأیید شد: (1) بارگذار خودِ `rspirv` بایت‌های ساطع‌شده را بدون خطا
  دوباره تجزیه می‌کند، (2) `spirv-val.exe` واقعی از Vulkan SDK
  (`C:\VulkanSDK\1.4.350.0\Bin\spirv-val.exe`) در برابر هر دو ماژول
  ساطع‌شده اجرا شد و برای هر دو کد خروج 0 بدون هیچ تشخیصی برگرداند.
  `translate_shader`/`translate_chain_shader` (فقط compute) همچنان به‌درستی
  هر دو شیدر را رد می‌کنند. **هیچ رسترایزر، فریم‌بافر، یا فراخوانی واقعی
  رسم Vulkan وجود ندارد** — تأیید شد (با خواندن سورس واقعی آن) که
  `opencuda-vulkan` هیچ کد
  `VkGraphicsPipelineCreateInfo`/render-pass/framebuffer ندارد، فقط
  compute-dispatch، بنابراین یک پیکسل واقعاً رندرشده در این گذر خارج از
  دامنه است. برای مرز صادقانه‌ی نقطه‌ی عطف به HANDOFF در `CLAUDE.md`
  مراجعه کنید.

## وضعیت فعلی (2026-07-26، نقطه‌ی عطف خط لوله‌ی گرافیکی حداقلی D3D11 محقق شد)

کریت جدید `crates/directx-graphics-vulkan` وابستگی **مستقیم** `ash` را به
این فضای کاری اضافه می‌کند (نه لایه‌شده روی `opencuda-vulkan`، که با
بازرسی سورس تأیید شد فقط compute-dispatch است). این کریت رندر پس واقعی،
فریم‌بافر، و `VkGraphicsPipelineCreateInfo` را پیاده‌سازی می‌کند، و
SPIR-V از پیش تولیدشده و از پیش تأییدشده با `spirv-val` را از
`translate_vertex_shader`/`translate_pixel_shader` بالا مجدداً استفاده
می‌کند (هیچ ترجمه‌ی شیدری دوباره پیاده‌سازی نشده است).
`render_uniform_triangle_and_read_back` یک مثلث «بزرگ» تمام‌دیدگاه با یک
رنگ رأس یکنواخت واحد رسم می‌کند، تصویر رندرشده را از طریق یک بافر واسط
میزبان‌قابل‌رؤیت می‌خواند، و تست سخت‌افزار واقعی
(`crates/directx-graphics-vulkan/tests/triangle_real_vulkan.rs`) تأیید
می‌کند که هر پیکسل خوانده‌شده با رنگ رأس عبوری روی NVIDIA GT 730 واقعی
موجود در این ماشین مطابقت دارد (`cargo test -p
directx-graphics-vulkan --test triangle_real_vulkan -- --nocapture`: 1
عبور). دامنه به‌عمد باریک است: یک جفت شیدر ثابت، یک فراخوانی رسم، بدون
بافر عمق/بافت/زنجیره‌ی تعویض/بررسی میان‌یابی چند‌مثلثی. برای افشای کامل
صادقانه به HANDOFF در `CLAUDE.md` (ادامه‌ی 2026-07-26) مراجعه کنید.

## وضعیت فعلی (2026-07-25، برش عمودی فاز 1 به 3 شیدر شناخته‌شده تعمیم یافت)

`crates/directx-shader-translate` اکنون برش عمودی کامل را برای **سه
شیدر شناخته‌شده‌ی مشخص** انجام می‌دهد (`vector_add.hlsl`،
`vector_mul.hlsl`، `vector_sub_bounded.hlsl`): تجزیه‌ی DXBC -> رمزگشایی
زیرمجموعه‌ی محدود اپکد SM5.0 -> تولید کد SPIR-V (از طریق `rspirv`) ->
ارسال واقعی Vulkan (`opencuda-vulkan` از `open-cuda`) -> تطابق عددی با
مرجع CPU، که روی NVIDIA GT 730 واقعی این ماشین تأیید شده است. **این
همچنان یک رمزگشای عمومی SM5.0 به SPIR-V نیست** — به «پیاده‌سازی‌نشده» در
پایین مراجعه کنید.

- `parse_dxbc` (فاز 0): بازرسی کانتینر/تکه‌ی DXBC (حضور RDEF/ISGN/
  OSGN/SHEX)، بدون تغییر نسبت به فرانت‌اند اصلی.
- `spirv_gen::translate_shader` (فاز 1، تعمیم‌یافته در 2026-07-25):
  3 شکل اپکد که واقعاً توسط `fxc.exe` ساطع می‌شوند را تشخیص می‌دهد، همگی
  با یک اسکلت مشترک (`dcl_globalFlags` -> `dcl_constantbuffer` اختیاری
  -> 3× `dcl_uav_structured` -> `dcl_input` -> `dcl_temps` ->
  `dcl_thread_group` -> `ult`+`if` اختیاری -> 2× `ld_structured` ->
  `add`/`mul` -> `store_structured` -> `endif` اختیاری -> `ret`):
  - `vector_add.hlsl`: `add`، بدون بررسی مرز.
  - `vector_mul.hlsl`: `mul` به‌جای `add`.
  - `vector_sub_bounded.hlsl`: `add` با پرچم `negate` روی اولین عملوند
    منبع آن (با بررسی خروجی واقعی `fxc.exe` تأیید شده — `fxc`، `a - b`
    را به جای صدور اپکد اختصاصی `sub`، به `add dest, -b, a` بهینه
    می‌کند)، به‌علاوه‌ی یک بررسی مرز واقعی `if (id.x < N)` (`ult` در
    برابر یک بافر ثابت + `if`/`endif`)، که SPIR-V ساطع‌شده آن را با
    `OpSelectionMerge`/`OpBranchConditional` واقعی پیاده‌سازی می‌کند، با
    استفاده از پوش‌کانستنت `n` برای مقایسه.
  هر اپکد/شکل دیگری به جای ترجمه‌ی نادرست خاموش، از طریق
  `SpirvGenError::UnsupportedShader` رد می‌شود. نقاط اتصال UAV، اندازه‌ی
  گروه کاری، عملگر، و حضور بررسی مرز همگی از DXBC تجزیه‌شده‌ی واقعی
  استخراج می‌شوند، نه ثابت‌کدشده. `translate_vector_add_shader` به‌عنوان
  یک نام مستعار نازک سازگار به عقب برای `translate_shader` نگه داشته
  شده است.
- `tests/vector_add_real_vulkan.rs`، `tests/vector_mul_real_vulkan.rs`،
  `tests/vector_sub_bounded_real_vulkan.rs`: هر کدام SPIR-V ترجمه‌شده‌ی
  خود را از طریق `opencuda-vulkan::VulkanDevice` واقعی `open-cuda`
  (`ash`، ویژگی `real-vulkan`) ارسال می‌کنند و خروجی GPU را در برابر
  مرجع CPU برای 256 عنصر (تلورانس 1e-3/1e-2) بررسی می‌کنند. تست بررسی
  مرز به‌علاوه 320 نخ را با تعداد عنصر منطقی 256 ارسال می‌کند و تأیید
  می‌کند که عناصر 256..320 هرگز نوشته نمی‌شوند (روی مقدار سنتینل باقی
  می‌مانند)، که ثابت می‌کند شاخه‌ی `if (id.x < N)` در SPIR-V تولیدشده
  واقعاً اجرا را دروازه‌بانی می‌کند، نه فقط کامپایل می‌شود.
- `examples/dump_shex.rs`: یک ابزار مستقل کوچک
  (`cargo run -p directx-shader-translate --example dump_shex -- <path.dxbc>`)
  که در طول این نشست برای بازرسی جریان‌های واقعی اپکد SHEX پیش از نوشتن
  پشتیبانی رمزگشا برای آن‌ها استفاده شد؛ برای کار تعمیم آینده اپکد به
  اپکد نگه داشته شده است.

**از زمان نوشتن عنوان این بخش**، چهارمین شیدر تک‌عملگری
(`vector_div.hlsl`، `div` ساده) با پیروی از همان الگو به `translate_shader`
افزوده شد، و — جدیدتر — یک کلاس الگوی واقعاً متفاوت،
`spirv_gen::translate_chain_shader`، در کنار آن (نه به‌جای آن) اضافه شد:
این تابع درخت واقعی عبارت رجیستر عملیات دودویی متوالی (جمع/ضرب، بدون
جریان کنترل) را رمزگشایی می‌کند به‌جای یک عملگر ثابت منفرد، که در برابر
شیدری تازه کامپایل‌شده تأیید شده که SHEX واقعی آن معلوم شد یک رجیستر
موقت را از طریق CSE فxc به‌جای اعلام رجیسترهای موقت اضافی، اجزای آن را
مجدداً استفاده می‌کند. برای گزارش کامل و فعلی به ورودی HANDOFF «ادامه 9»
مورخ 2026-07-25 در `CLAUDE.md` مراجعه کنید (این بخش برای دقت تاریخی
درباره‌ی وضعیت میانروزی 2026-07-25 به همان شکل اصلی باقی گذاشته شده است).

## ساخت و تست

```powershell
cargo build --workspace
cargo test --workspace -- --nocapture
```

### واقعاً چیزی رسم‌شده را ببینید (افزوده‌شده در 2026-07-27)

این مخزن مجموعه‌ای از کتابخانه‌هاست و `fn main` خودش را ندارد، پس
سریع‌ترین راه برای *دیدن* کار خط لوله‌ی گرافیکی روی GPU خودتان — به‌جای
خواندن سورس تست — این است:

```bash
cargo run -p directx-graphics-vulkan --example render_triangle
```

این دستور همان شیدرهای ترجمه‌شده‌ی DXBC → SPIR-V کامپایل‌شده با
fxc.exe واقعی را که `tests/triangle_real_vulkan.rs` استفاده می‌کند
مجدداً به‌کار می‌برد، یک مثلث گرادیان (قرمز/سبز/آبی) روی سخت‌افزار واقعی
Vulkan رسم می‌کند، فریم‌بافر را می‌خواند، و آن را در `render_triangle.ppm`
می‌نویسد (PPM ساده، بدون نیاز به وابستگی اضافی کریت تصویر — آن را با
مثلاً `magick render_triangle.ppm render_triangle.png` تبدیل کنید یا
مستقیماً در بیشتر نمایشگرهای تصویر باز کنید). اگر هیچ درایور/دستگاه
قابل‌استفاده‌ی Vulkan وجود نداشته باشد، به‌جای جعل موفقیت، خطای صادقانه
چاپ کرده و با کد غیرصفر خارج می‌شود.

خروجی واقعاً مشاهده‌شده (2026-07-25، این ماشین، NVIDIA GeForce GT 730):

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

`cargo clippy --workspace --all-targets`: 0 هشدار.

پس از کار جدول نوع/رمزگشایی دستورالعمل DXIL (2026-07-25، ادامه، مسیر
D3D12)، `cargo test --workspace` در مجموع 23 تست را اجرا می‌کند (19 تست
واحد + 4 تست یکپارچگی Vulkan واقعی)، همگی موفق، شامل 3 تست جدید علاوه
بر 20 تست پیشین: `dxil::tests::resolves_real_dxil_
type_table_and_finds_float_and_resource_struct`، `dxil::tests::decodes_
real_dxil_function_block_into_matching_vector_add_shape`، و `dxil::
tests::shape_matcher_honestly_rejects_unexpected_instruction_orderings`.

برای بازتولید فیکسچرهای DXBC از HLSL (نیازمند `fxc.exe` از Windows SDK —
توجه کنید که `dxc.exe` فقط DXIL/SM6+ را هدف می‌گیرد و نمی‌تواند DXBC
تولید کند):

```powershell
pwsh tools/compile-dxbc-shaders.ps1
```

## پیاده‌سازی‌نشده (دامنه‌ی صادقانه)

- **رمزگشایی عمومی دستورالعمل SM5.0.** فقط 3 شکل اپکد بالا مدیریت
  می‌شوند؛ هر شیدر compute دیگر D3D11 (چیدمان منبع متفاوت، جریان کنترل
  دیگر، شهود دیگر، بیش از یک بررسی مرز، `div`/`sub` به‌عنوان یک اپکد
  واقعی به‌جای `add` منفی‌شده و غیره) به‌جای ترجمه‌ی نادرست، رد می‌شود.
  ساخت یک رمزگشای عمومی واقعی (یا اقتباس/انتقال یک رمزگشای موجود، مثلاً
  مطالعه‌ی دقیق‌تر رویکرد `dxbc-spirv`/`dxil-spirv`) نقطه‌ی عطف واقعی
  بعدی باقی می‌ماند.
- **DXIL (شیدر مدل 6+، D3D12): برش عمودی `vector_add.dxil` اکنون
  به‌طور سرتاسری، روی سخت‌افزار واقعی کامل است — اما همچنان فقط برای این
  یک شکل شناخته‌شده‌ی شیدر، نه SM6.0 عمومی.**
  `resolve_type_table`/`decode_function_instructions`/
  `resolve_vector_add_dxil_calls` در `dxil.rs` رکوردهای واقعی
  `TYPE_BLOCK`/`FUNCTION_BLOCK`/`VALUE_SYMTAB_BLOCK` را در برابر کدهای
  مستند LLVM رمزگشایی می‌کنند و همه‌ی 7 رکورد `Call` را به معنای واقعی
  `dx.op.*` آن‌ها تفکیک می‌کنند (`CreateHandle`/`ThreadId`/`BufferLoad`/
  `BufferStore`، همراه با نقاط اتصال UAV).
  `translate_dxil_vector_add_to_spirv` (جدید) خروجی حل‌شده را به
  `emit_spirv_for_kernel` مشترک `spirv_gen.rs` (که از `emit_spirv` مسیر
  DXBC استخراج شده تا هر دو بک‌اند از یک مسیر کد ساطع کنند) می‌سپارد تا
  SPIR-V واقعی تولید شود، که `tests/vector_add_dxil_real_vulkan.rs` آن را
  روی NVIDIA GT 730 واقعی این ماشین از طریق `opencuda-vulkan` ارسال
  می‌کند و تأیید می‌کند با مرجع CPU `a[i]+b[i]` برای همه‌ی 256 عنصر
  مطابقت دارد — همان دقتِ تست `vector_add` در DXBC. **اندازه‌ی گروه کاری
  اکنون واقعاً استخراج می‌شود، نه ثابت‌کدشده**:
  `extract_numthreads_from_metadata` (`dxil.rs`) مسیر واقعی
  `METADATA_BLOCK` را طی می‌کند: `dx.entryPoints` -> تاپل هر
  نقطه‌ی ورودی -> `ShaderProperties` -> `kDxilNumThreadsTag` (=4، در
  برابر سورس‌های `DxilMetadataHelper.h`/`.cpp` مایکروسافت
  `DirectXShaderCompiler` تأیید شده) و گره `{x,y,z}` را در برابر لیست
  مقدار واقعی ماژول حل می‌کند، که `(64,1,1)` را از بایت‌های واقعی
  `vector_add.dxil` می‌دهد — کدسازی ثابت شناخته‌شده‌ی ورودی پیشین بسته
  شده است، و یک تست رگرسیون مصنوعی ثابت می‌کند که منطق استخراج مقدار
  *متفاوتی* را وقتی متادیتای متفاوتی داده می‌شود برمی‌گرداند (نه فقط
  «همیشه 64,1,1 برمی‌گرداند بدون توجه به ورودی»). هر شکل دیگر
  اپکد/عملوند (عملیات متفاوت، چند بلوک پایه، بررسی‌های مرز) همچنان رد
  می‌شود، نه ترجمه‌ی نادرست. پشتیبانی از فهرست فرمان/انبوه توصیفگر/امضای
  ریشه‌ی D3D12 (لایه‌ی بالای ترجمه‌ی شیدر) دست‌نخورده باقی مانده است.
- **رمزگشای DXBC فراتر از 4 شکل تک‌عملگری ثابت تعمیم یافته: اکنون
  زنجیره‌های عملیات دودویی متوالی (بدون جریان کنترل) را از طریق یک درخت
  واقعی عبارت رجیستر مدیریت می‌کند، نه یک پنجمین شکل ثابت‌کدشده.**
  `spirv_gen::translate_chain_shader`/`decode_chain_shape` روی
  `ld_structured`/`add`/`mul`/`store_structured` پیمایش می‌کنند و یک
  درخت عبارت واقعی به‌کلید (رجیستر موقت، جزء) می‌سازند، پس 1 عملیات، 2
  عملیات، یا N عملیات را به یک شکل مدیریت می‌کند — در برابر یک شیدر
  واقعی تازه کامپایل‌شده (`vector_add_mul_chain.hlsl`، `t = A[i]+B[i];
  Out[i] = t*A[i]`) تأیید شده که SHEX واقعی آن معلوم شد اجزای `.x`/`.y`
  یک رجیستر موقت واحد را دوباره استفاده می‌کند (fxc با CSE بارگذاری
  تکراری `A[i]` را حذف کرده به‌جای صدور مجدد `ld_structured`) — یک
  یافته‌ی واقعی و غیرمنتظر که رمزگشای درخت‌محور بدون نیاز به حالت
  اضافی مدیریت می‌کند. روی NVIDIA GT 730 واقعی در برابر مرجع CPU
  `(a[i]+b[i])*a[i]` ارسال و تأیید شده است. `sub`/`div` داخل زنجیره
  به‌عمد همچنان رد می‌شوند (معنای ترتیب عملوند آن‌ها فقط برای حالت
  تک‌عملگری تأیید شده است). 4 شکل تک‌عملگری اصلی دست‌نخورده و همچنان
  بدون تغییر عبور می‌کنند.
- **خط لوله‌ی گرافیکی D3D11: تجزیه‌ی کانتینر DXBC برای VS/PS تأیید
  شده که کار می‌کند، اما هیچ تولید کد SPIR-V، رسترایزر، یا مثلث واقعی
  رسم‌شده روی صفحه وجود ندارد.** خط لوله‌ی کامل (رسترایزر، نمونه‌برداری
  بافت، حالت ترکیب، خروجی-ادغام‌گر) خارج از دامنه باقی می‌ماند؛ همچنین
  تعمیم رمزگشای شکل اپکد باریک `spirv_gen` برای درک
  `dcl_output_siv`/`dcl_input_ps`/حالت‌های میان‌یابی.
- اهداف خانواده‌ی PlayStation — به‌صراحت خارج از دامنه؛ برای منطق
  حقوقی/شرایط خدمات به `CLAUDE.md` مراجعه کنید.

## پروژه‌های مرتبط

- [open-cuda](https://github.com/aon-co-jp/open-cuda) — بک‌اند اجرای
  compute Vulkan که این پروژه طراحی شده تا از طریق آن ارسال کند
  (`opencuda-core::GpuDevice`، `KernelSource::SpirV`). همچنین شامل یک
  کریت `opencuda-directx` بدون‌ربط و از پیش کارآمد است که D3D12 را
  **به‌طور بومی روی ویندوز** اجرا می‌کند — جهت مخالف این پروژه (که
  شیدرهای DirectX را **روی اهداف غیر ویندوز** اجرا می‌کند).
- [aruaru-llm](https://github.com/aon-co-jp/aruaru-llm) — بدون وابستگی
  فنی مستقیم به این پروژه (تأیید شده، نه فرض‌شده).
