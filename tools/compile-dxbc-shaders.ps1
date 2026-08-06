# HLSLをDXBC(D3D9/10/11シェーダーバイトコード, Shader Model <= 5.1)へ
# コンパイルする(open-cudaの`tools/compile-dx12-shaders.ps1`と同じ命名・
# 構造パターン)。ファイル名は当初Compute Shader専用だった名残りだが、
# 現在はD3D11の頂点/ピクセルシェーダー(DXBC)とDXIL(SM6.0)のコンパイルも
# 兼ねる(リネームは新規判断のため見送り、中身のコメントで補足する)。
#
# 重要: DXBC(SM5.0以前)は`fxc.exe`でコンパイルする。`dxc.exe`はSM6.0+の
# DXILしか出力できないため、本プロジェクト(D3D9/10/11 DXBC対応)には使えない。
# `fxc.exe`はWindows SDK付属(`C:\Program Files (x86)\Windows Kits\10\bin\
# <version>\x64\fxc.exe`)。DXIL(SM6.0)は逆に`dxc.exe`(Vulkan SDK同梱)が
# 必要——下記参照。
#
# 使い方: pwsh tools/compile-dxbc-shaders.ps1
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

$fxc = $env:FXC_BIN
if (-not $fxc) {
    $cmd = Get-Command fxc.exe -ErrorAction SilentlyContinue
    if ($cmd) {
        $fxc = $cmd.Source
    } else {
        $candidates = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\fxc.exe" -ErrorAction SilentlyContinue
        if ($candidates) {
            $fxc = ($candidates | Sort-Object FullName -Descending | Select-Object -First 1).FullName
        } else {
            Write-Error "fxc.exe not found. Install the Windows SDK, or set FXC_BIN."
            exit 1
        }
    }
}
Write-Host "using fxc: $fxc"

$shaderDir = Join-Path $root "crates\directx-shader-translate\shaders"
& $fxc /T cs_5_0 /E main (Join-Path $shaderDir "vector_add.hlsl") /Fo (Join-Path $shaderDir "vector_add.dxbc") /nologo
Write-Host "OK: compiled vector_add.hlsl -> vector_add.dxbc (DXBC, SM5.0)"
& $fxc /T cs_5_0 /E main (Join-Path $shaderDir "vector_mul.hlsl") /Fo (Join-Path $shaderDir "vector_mul.dxbc") /nologo
Write-Host "OK: compiled vector_mul.hlsl -> vector_mul.dxbc (DXBC, SM5.0)"
& $fxc /T cs_5_0 /E main (Join-Path $shaderDir "vector_sub_bounded.hlsl") /Fo (Join-Path $shaderDir "vector_sub_bounded.dxbc") /nologo
Write-Host "OK: compiled vector_sub_bounded.hlsl -> vector_sub_bounded.dxbc (DXBC, SM5.0)"
& $fxc /T cs_5_0 /E main (Join-Path $shaderDir "vector_div.hlsl") /Fo (Join-Path $shaderDir "vector_div.dxbc") /nologo
Write-Host "OK: compiled vector_div.hlsl -> vector_div.dxbc (DXBC, SM5.0)"
& $fxc /T cs_5_0 /E main (Join-Path $shaderDir "vector_add_mul_chain.hlsl") /Fo (Join-Path $shaderDir "vector_add_mul_chain.dxbc") /nologo
Write-Host "OK: compiled vector_add_mul_chain.hlsl -> vector_add_mul_chain.dxbc (DXBC, SM5.0, 2 sequential ops / 3 UAVs, reg-expr chain decoder)"
& $fxc /T cs_5_0 /E main (Join-Path $shaderDir "vector_add_mul_div_chain3.hlsl") /Fo (Join-Path $shaderDir "vector_add_mul_div_chain3.dxbc") /nologo
Write-Host "OK: compiled vector_add_mul_div_chain3.hlsl -> vector_add_mul_div_chain3.dxbc (DXBC, SM5.0, 3 sequential ops / 3 UAVs, reg-expr chain decoder)"
& $fxc /T cs_5_0 /E main (Join-Path $shaderDir "vector_sub_div_add_mul_chain4.hlsl") /Fo (Join-Path $shaderDir "vector_sub_div_add_mul_chain4.dxbc") /nologo
Write-Host "OK: compiled vector_sub_div_add_mul_chain4.hlsl -> vector_sub_div_add_mul_chain4.dxbc (DXBC, SM5.0, 4 sequential ops sub->div->add->mul / 3 UAVs, reg-expr chain decoder)"
& $fxc /T cs_5_0 /E main (Join-Path $shaderDir "vector_add_mul_chain_bounded.hlsl") /Fo (Join-Path $shaderDir "vector_add_mul_chain_bounded.dxbc") /nologo
Write-Host "OK: compiled vector_add_mul_chain_bounded.hlsl -> vector_add_mul_chain_bounded.dxbc (DXBC, SM5.0, cbuffer+ult+if+endif bounded 2-op chain, 2026-08-06)"

# D3D11グラフィックスパイプライン(タスク2): 頂点/ピクセルシェーダーも
# fxc.exe(SM<=5.1、DXBC)でコンパイルする。dxc.exeではない(dxc.exeは
# SM6.0+/DXIL専用でDXBCを出力できない、上記コメント参照)。
& $fxc /T vs_5_0 /E main (Join-Path $shaderDir "triangle_vs.hlsl") /Fo (Join-Path $shaderDir "triangle_vs.dxbc") /nologo
Write-Host "OK: compiled triangle_vs.hlsl -> triangle_vs.dxbc (DXBC, SM5.0, vertex shader)"
& $fxc /T ps_5_0 /E main (Join-Path $shaderDir "triangle_ps.hlsl") /Fo (Join-Path $shaderDir "triangle_ps.dxbc") /nologo
Write-Host "OK: compiled triangle_ps.hlsl -> triangle_ps.dxbc (DXBC, SM5.0, pixel shader)"

# タスク1(DXIL): dxc.exe(Vulkan SDK同梱、DirectX Shader Compiler本体)で
# SM6.0のDXIL(D3D12用、LLVM bitcodeベース)を出力する。fxc.exeではDXILは
# 出力できない(fxc.exeはSM<=5.1/DXBC専用、上記と対称)。
$dxc = $env:DXC_BIN
if (-not $dxc) {
    $cmd = Get-Command dxc.exe -ErrorAction SilentlyContinue
    if ($cmd) {
        $dxc = $cmd.Source
    } elseif ($env:VULKAN_SDK) {
        $dxc = Join-Path $env:VULKAN_SDK "Bin\dxc.exe"
    }
}
if (-not $dxc -or -not (Test-Path $dxc)) {
    Write-Error "dxc.exe not found. Install the Vulkan SDK (or Windows SDK's dxc.exe), or set DXC_BIN."
    exit 1
}
Write-Host "using dxc: $dxc"
& $dxc -T cs_6_0 -E main (Join-Path $shaderDir "vector_add_dxil.hlsl") -Fo (Join-Path $shaderDir "vector_add.dxil")
Write-Host "OK: compiled vector_add_dxil.hlsl -> vector_add.dxil (DXIL, SM6.0)"
# 2026-07-26: DXILデコーダをadd専用からmul/sub/div一般化するための追加3本
# (vector_add_dxil.hlslと同一形状・同一契約、演算のみ異なる)。
& $dxc -T cs_6_0 -E main (Join-Path $shaderDir "vector_mul_dxil.hlsl") -Fo (Join-Path $shaderDir "vector_mul.dxil")
Write-Host "OK: compiled vector_mul_dxil.hlsl -> vector_mul.dxil (DXIL, SM6.0)"
& $dxc -T cs_6_0 -E main (Join-Path $shaderDir "vector_sub_dxil.hlsl") -Fo (Join-Path $shaderDir "vector_sub.dxil")
Write-Host "OK: compiled vector_sub_dxil.hlsl -> vector_sub.dxil (DXIL, SM6.0)"
& $dxc -T cs_6_0 -E main (Join-Path $shaderDir "vector_div_dxil.hlsl") -Fo (Join-Path $shaderDir "vector_div.dxil")
Write-Host "OK: compiled vector_div_dxil.hlsl -> vector_div.dxil (DXIL, SM6.0)"
# 2026-08-05: DXIL側チェーン(N個の逐次2項演算)対応のための2本
# (vector_add_mul_chain.hlsl/vector_sub_div_chain.hlslのDXBC/SM5.0版と同一
# 契約・演算内容、dxc.exe/SM6.0向けに別ファイルへ分離した_dxil.hlsl版)。
& $dxc -T cs_6_0 -E main (Join-Path $shaderDir "vector_add_mul_chain_dxil.hlsl") -Fo (Join-Path $shaderDir "vector_add_mul_chain.dxil")
Write-Host "OK: compiled vector_add_mul_chain_dxil.hlsl -> vector_add_mul_chain.dxil (DXIL, SM6.0, 2 sequential ops add+mul)"
& $dxc -T cs_6_0 -E main (Join-Path $shaderDir "vector_sub_div_chain_dxil.hlsl") -Fo (Join-Path $shaderDir "vector_sub_div_chain.dxil")
Write-Host "OK: compiled vector_sub_div_chain_dxil.hlsl -> vector_sub_div_chain.dxil (DXIL, SM6.0, 2 sequential ops sub+div)"
# 2026-08-05: 3個の逐次2項演算チェーン(add+mul+div)のDXBC/DXIL版
# (decode_chain_shape/resolve_dxil_calls_and_chainを3項へ拡張する検証用)。
& $dxc -T cs_6_0 -E main (Join-Path $shaderDir "vector_add_mul_div_chain3_dxil.hlsl") -Fo (Join-Path $shaderDir "vector_add_mul_div_chain3.dxil")
Write-Host "OK: compiled vector_add_mul_div_chain3_dxil.hlsl -> vector_add_mul_div_chain3.dxil (DXIL, SM6.0, 3 sequential ops add+mul+div)"
# 2026-08-06: 4個の逐次2項演算チェーン(sub->div->add->mul、新しい順序組み
# 合わせ)のDXBC/DXIL版(decode_chain_shape/resolve_dxil_calls_and_chainが
# 4項・かつsub先頭の順序でも無改修で動くかの検証用)。
& $dxc -T cs_6_0 -E main (Join-Path $shaderDir "vector_sub_div_add_mul_chain4_dxil.hlsl") -Fo (Join-Path $shaderDir "vector_sub_div_add_mul_chain4.dxil")
Write-Host "OK: compiled vector_sub_div_add_mul_chain4_dxil.hlsl -> vector_sub_div_add_mul_chain4.dxil (DXIL, SM6.0, 4 sequential ops sub->div->add->mul)"
# 2026-08-06: 境界チェック付き2項演算チェーンのDXIL版(DXBC側
# vector_add_mul_chain_bounded.hlslと同一契約、resolve_dxil_calls_and_chainへ
# 境界チェック検出ロジックを追加するための検証用)。
& $dxc -T cs_6_0 -E main (Join-Path $shaderDir "vector_add_mul_chain_bounded_dxil.hlsl") -Fo (Join-Path $shaderDir "vector_add_mul_chain_bounded.dxil")
Write-Host "OK: compiled vector_add_mul_chain_bounded_dxil.hlsl -> vector_add_mul_chain_bounded.dxil (DXIL, SM6.0, cbuffer+bounds-checked 2-op chain)"
