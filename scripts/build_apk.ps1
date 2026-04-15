#!/usr/bin/env pwsh
# 完整构建脚本：Rust + Android APK (Gradle)
#
# 使用方法:
#   .\build_apk.ps1           # 构建 Debug APK
#   .\build_apk.ps1 -Release  # 构建 Release APK
#   .\build_apk.ps1 -SkipRust # 跳过 Rust 构建

param(
    [switch]$Release,
    [switch]$SkipRust
)

$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$ScriptsDir = $PSScriptRoot
$AndroidDir = Join-Path $ProjectRoot "android"
$OutputDir = if ($Release) {
    Join-Path $AndroidDir "app/build/outputs/apk/release"
} else {
    Join-Path $AndroidDir "app/build/outputs/apk/debug"
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "SecureZip APK 构建脚本" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "模式: $(if ($Release) { 'Release' } else { 'Debug' })" -ForegroundColor Yellow

# 步骤 1: 构建 Rust 库
if (-not $SkipRust) {
    Write-Host "`n[1/3] 构建 Rust 库..." -ForegroundColor Yellow
    $rustScript = Join-Path $ScriptsDir "build_android_rust.ps1"

    if ($Release) {
        & $rustScript -Release
    } else {
        & $rustScript
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Host "Rust 构建失败!" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "`n[1/3] 跳过 Rust 构建" -ForegroundColor Gray
}

# 步骤 2: 构建 Android APK (Gradle)
Write-Host "`n[2/3] 构建 Android APK..." -ForegroundColor Yellow
Push-Location $AndroidDir

try {
    if ($Release) {
        ./gradlew assembleRelease
    } else {
        ./gradlew assembleDebug
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Host "Gradle 构建失败!" -ForegroundColor Red
        Pop-Location
        exit 1
    }
} finally {
    Pop-Location
}

# 步骤 3: 显示输出
Write-Host "`n[3/3] 构建完成!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "输出文件:" -ForegroundColor Yellow

if (Test-Path $OutputDir) {
    Get-ChildItem -Path $OutputDir -Filter "*.apk" | ForEach-Object {
        $size = $_.Length / 1MB
        Write-Host "  $($_.Name) ($($size.ToString('F2')) MB)" -ForegroundColor Green
        Write-Host "    路径: $($_.FullName)" -ForegroundColor Gray
    }
} else {
    Write-Host "  未找到输出目录: $OutputDir" -ForegroundColor Yellow
}

Write-Host "`n========================================" -ForegroundColor Cyan
