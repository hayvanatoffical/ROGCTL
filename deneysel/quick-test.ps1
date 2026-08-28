# ROGCtl - Quick Test Script (No Emoji)

$ErrorActionPreference = "Continue"

Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  ROGCtl - Build Dogrulama" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

$rootDir = $PSScriptRoot
$errors = 0
$warnings = 0

# 1. Rust kontrolu
Write-Host "[1/8] Rust kontrolu..." -NoNewline
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    Write-Host " OK" -ForegroundColor Green
} else {
    Write-Host " HATA" -ForegroundColor Red
    $errors++
}

# 2. Inno Setup kontrolu
Write-Host "[2/8] Inno Setup kontrolu..." -NoNewline
$isccPaths = @(
    "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
    "C:\Program Files\Inno Setup 6\ISCC.exe",
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
)
$isccPath = $null
foreach ($p in $isccPaths) {
    if (Test-Path $p) {
        $isccPath = $p
        break
    }
}
if ($isccPath) {
    Write-Host " OK" -ForegroundColor Green
} else {
    Write-Host " HATA" -ForegroundColor Red
    $errors++
}

# 3. CLI exe kontrolu
Write-Host "[3/8] CLI binary kontrolu..." -NoNewline
$cliPath = "$rootDir\target\release\rogctl.exe"
if (Test-Path $cliPath) {
    $size = (Get-Item $cliPath).Length / 1MB
    Write-Host " OK ($(([math]::Round($size, 2))) MB)" -ForegroundColor Green
} else {
    Write-Host " HATA" -ForegroundColor Red
    $errors++
}

# 4. Setup script kontrolu
Write-Host "[4/8] Setup.iss kontrolu..." -NoNewline
if (Test-Path "$rootDir\installer\setup.iss") {
    Write-Host " OK" -ForegroundColor Green
} else {
    Write-Host " HATA" -ForegroundColor Red
    $errors++
}

# 5. Kurulum scriptleri
Write-Host "[5/8] Kurulum scriptleri kontrolu..." -NoNewline
$scripts = @(
    "installer\scripts\check-requirements.ps1",
    "installer\scripts\post-install.ps1",
    "installer\scripts\uninstall-cleanup.ps1"
)
$missing = @()
foreach ($s in $scripts) {
    if (-not (Test-Path "$rootDir\$s")) {
        $missing += $s
    }
}
if ($missing.Count -eq 0) {
    Write-Host " OK" -ForegroundColor Green
} else {
    Write-Host " HATA" -ForegroundColor Red
    $errors++
}

# 6. Dokumantasyon
Write-Host "[6/8] Dokumantasyon kontrolu..." -NoNewline
$docs = @("LICENSE.txt", "INSTALL_INFO.txt")
$missingDocs = @()
foreach ($d in $docs) {
    if (-not (Test-Path "$rootDir\$d")) {
        $missingDocs += $d
    }
}
if ($missingDocs.Count -eq 0) {
    Write-Host " OK" -ForegroundColor Green
} else {
    Write-Host " UYARI" -ForegroundColor Yellow
    $warnings++
}

# 7. Release dizini
Write-Host "[7/8] Release dizini kontrolu..." -NoNewline
$releaseDir = "$rootDir\release"
if (-not (Test-Path $releaseDir)) {
    New-Item -ItemType Directory -Path $releaseDir -Force | Out-Null
    Write-Host " OK (olusturuldu)" -ForegroundColor Green
} else {
    Write-Host " OK" -ForegroundColor Green
}

# 8. Icon (opsiyonel)
Write-Host "[8/8] Icon kontrolu..." -NoNewline
if (Test-Path "$rootDir\installer\assets\rogctl.ico") {
    Write-Host " OK" -ForegroundColor Green
} else {
    Write-Host " UYARI (varsayilan kullanilacak)" -ForegroundColor Yellow
    $warnings++
}

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan

if ($errors -eq 0) {
    Write-Host "BASARILI! Build yapilabilir." -ForegroundColor Green
    Write-Host ""
    Write-Host "Kurulum paketi olusturmak icin:" -ForegroundColor White
    Write-Host '  powershell -ExecutionPolicy Bypass -File "build-installer.ps1" -SkipGUI' -ForegroundColor Cyan
    Write-Host ""
    exit 0
} else {
    Write-Host "HATALAR VAR! ($errors hata, $warnings uyari)" -ForegroundColor Red
    Write-Host ""
    Write-Host "Once hatalari duzeltin." -ForegroundColor Red
    Write-Host ""
    exit 1
}
