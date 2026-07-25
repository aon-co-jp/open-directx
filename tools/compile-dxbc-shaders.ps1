# HLSLをDXBC(D3D9/10/11シェーダーバイトコード, Shader Model <= 5.1)へ
# コンパイルする(open-cudaの`tools/compile-dx12-shaders.ps1`と同じ命名・
# 構造パターン)。
#
# 重要: DXBC(SM5.0以前)は`fxc.exe`でコンパイルする。`dxc.exe`はSM6.0+の
# DXILしか出力できないため、本プロジェクト(D3D9/10/11 DXBC対応)には使えない。
# `fxc.exe`はWindows SDK付属(`C:\Program Files (x86)\Windows Kits\10\bin\
# <version>\x64\fxc.exe`)。
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
