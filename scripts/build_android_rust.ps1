#!/usr/bin/env pwsh
# Android Rust 库构建脚本
# 使用 cargo-ndk 交叉编译 Rust 库到 Android 目标平台

param(
    [switch]$Release,
    [switch]$SkipInstall
)

$ErrorActionPreference = "Stop"

# 项目路径
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$RustDir = Join-Path $ProjectRoot "rust"
$AndroidDir = Join-Path $ProjectRoot "android"
$JniLibsDir = Join-Path $AndroidDir "app/src/main/jniLibs"

# Android NDK 目标
$Targets = @(
    @{ Triple = "aarch64-linux-android"; JniDir = "arm64-v8a" },
    @{ Triple = "armv7-linux-androideabi"; JniDir = "armeabi-v7a" },
    @{ Triple = "x86_64-linux-android"; JniDir = "x86_64" }
)

# 库名称
$LibName = "libsz_ffi.so"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Android Rust 库构建脚本" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# 检查 cargo-ndk 是否安装
if (-not $SkipInstall) {
    Write-Host "`n[1/5] 检查 cargo-ndk..." -ForegroundColor Yellow
    $cargoNdkInstalled = cargo ndk --version 2>$null
    if (-not $cargoNdkInstalled) {
        Write-Host "安装 cargo-ndk..." -ForegroundColor Green
        cargo install cargo-ndk
    } else {
        Write-Host "cargo-ndk 已安装: $cargoNdkInstalled" -ForegroundColor Green
    }
    
    # 添加 Rust 目标
    Write-Host "`n[2/5] 添加 Rust 目标..." -ForegroundColor Yellow
    foreach ($target in $Targets) {
        Write-Host "  添加 $($target.Triple)..." -ForegroundColor Gray
        rustup target add $target.Triple
    }
}

# 确保 jniLibs 目录存在
Write-Host "`n[3/5] 创建 jniLibs 目录..." -ForegroundColor Yellow
foreach ($target in $Targets) {
    $dir = Join-Path $JniLibsDir $target.JniDir
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
    Write-Host "  $($target.JniDir): $dir" -ForegroundColor Gray
}

# 构建 Rust 库
Write-Host "`n[4/5] 构建 Rust 库..." -ForegroundColor Yellow
Push-Location $RustDir

$BuildMode = if ($Release) { "--release" } else { "" }
$TargetDir = if ($Release) { "release" } else { "debug" }

foreach ($target in $Targets) {
    Write-Host "`n  构建 $($target.Triple)..." -ForegroundColor Cyan
    
    if ($Release) {
        cargo ndk -t $target.Triple build --release -p sz-ffi
    } else {
        cargo ndk -t $target.Triple build -p sz-ffi
    }
    
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  构建失败: $($target.Triple)" -ForegroundColor Red
        Pop-Location
        exit 1
    }
    
    Write-Host "  $($target.Triple) 构建成功" -ForegroundColor Green
}

Pop-Location

# 复制库文件到 jniLibs
Write-Host "`n[5/5] 复制库文件到 jniLibs..." -ForegroundColor Yellow
foreach ($target in $Targets) {
    $sourcePath = Join-Path $RustDir "target/$($target.Triple)/$TargetDir/$LibName"
    $destPath = Join-Path $JniLibsDir "$($target.JniDir)/$LibName"
    
    if (Test-Path $sourcePath) {
        Copy-Item -Path $sourcePath -Destination $destPath -Force
        $size = (Get-Item $destPath).Length / 1MB
        Write-Host "  $($target.JniDir): $($size.ToString('F2')) MB" -ForegroundColor Green
    } else {
        Write-Host "  警告: 未找到 $sourcePath" -ForegroundColor Yellow
    }
}

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "构建完成!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan

# 显示生成的文件
Write-Host "`n生成的库文件:" -ForegroundColor Yellow
Get-ChildItem -Path $JniLibsDir -Recurse -Filter "*.so" | ForEach-Object {
    $relativePath = $_.FullName.Replace($JniLibsDir, "jniLibs")
    $sizeMB = [math]::Round($_.Length / 1048576, 2)
    $sizeDisplay = "$sizeMB MB"
    Write-Host "  $relativePath ($sizeDisplay)" -ForegroundColor Gray
}
