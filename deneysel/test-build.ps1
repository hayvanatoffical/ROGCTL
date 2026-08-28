# ROGCtl - Build Test & Validation Script
# Tüm gereksinimleri ve dosyaları kontrol eder

$ErrorActionPreference = "Continue"

Write-Host "══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  ROGCtl - Build Doğrulama Testi" -ForegroundColor Cyan
Write-Host "══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

$rootDir = $PSScriptRoot
$errors = @()
$warnings = @()

# 1. Rust/Cargo kontrolü
Write-Host "[1/10] Rust kontrolü..." -NoNewline
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    Write-Host " ✓" -ForegroundColor Green
} else {
    Write-Host " ✗" -ForegroundColor Red
    $errors += "Rust kurulu değil (https://rustup.rs/)"
}

# 2. Node.js kontrolü (GUI için)
Write-Host "[2/10] Node.js kontrolü..." -NoNewline
if (Get-Command node -ErrorAction SilentlyContinue) {
    Write-Host " ✓" -ForegroundColor Green
} else {
    Write-Host " ⚠" -ForegroundColor Yellow
    $warnings += "Node.js kurulu değil (GUI için gerekli)"
}

# 3. Inno Setup kontrolü
Write-Host "[3/10] Inno Setup kontrolü..." -NoNewline
$isccPath = "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
if (Test-Path $isccPath) {
    Write-Host " ✓" -ForegroundColor Green
} else {
    Write-Host " ✗" -ForegroundColor Red
    $errors += "Inno Setup 6 kurulu değil (https://jrsoftware.org/isdl.php)"
}

# 4. CLI exe kontrolü
Write-Host "[4/10] CLI binary kontrolü..." -NoNewline
$cliPath = "$rootDir\target\release\rogctl.exe"
if (Test-Path $cliPath) {
    $size = (Get-Item $cliPath).Length / 1MB
    Write-Host " ✓ ($([math]::Round($size, 2)) MB)" -ForegroundColor Green
} else {
    Write-Host " ✗" -ForegroundColor Red
    $errors += "rogctl.exe bulunamadı (cargo build --release çalıştırın)"
}

# 5. GUI exe kontrolü
Write-Host "[5/10] GUI binary kontrolü..." -NoNewline
$guiPath = "$rootDir\rogctl-gui\src-tauri\target\release\rogctl-gui.exe"
if (Test-Path $guiPath) {
    $size = (Get-Item $guiPath).Length / 1MB
    Write-Host " ✓ ($([math]::Round($size, 2)) MB)" -ForegroundColor Green
} else {
    Write-Host " ⚠" -ForegroundColor Yellow
    $warnings += "rogctl-gui.exe bulunamadı (opsiyonel)"
}

# 6. Setup script kontrolü
Write-Host "[6/10] Inno Setup script kontrolü..." -NoNewline
$setupScript = "$rootDir\installer\setup.iss"
if (Test-Path $setupScript) {
    Write-Host " ✓" -ForegroundColor Green
} else {
    Write-Host " ✗" -ForegroundColor Red
    $errors += "setup.iss bulunamadı"
}

# 7. Kurulum scriptleri kontrolü
Write-Host "[7/10] Kurulum scriptleri kontrolü..." -NoNewline
$scriptsDir = "$rootDir\installer\scripts"
$requiredScripts = @(
    "check-requirements.ps1",
    "post-install.ps1",
    "uninstall-cleanup.ps1"
)
$missingScripts = @()
foreach ($script in $requiredScripts) {
    if (-not (Test-Path "$scriptsDir\$script")) {
        $missingScripts += $script
    }
}
if ($missingScripts.Count -eq 0) {
    Write-Host " ✓" -ForegroundColor Green
} else {
    Write-Host " ✗" -ForegroundColor Red
    $errors += "Eksik scriptler: $($missingScripts -join ', ')"
}

# 8. Dokümantasyon kontrolü
Write-Host "[8/10] Dokümantasyon kontrolü..." -NoNewline
$docs = @("LICENSE.txt", "INSTALL_INFO.txt", "README.md")
$missingDocs = @()
foreach ($doc in $docs) {
    if (-not (Test-Path "$rootDir\$doc")) {
        $missingDocs += $doc
    }
}
if ($missingDocs.Count -eq 0) {
    Write-Host " ✓" -ForegroundColor Green
} else {
    Write-Host " ⚠" -ForegroundColor Yellow
    $warnings += "Eksik dokümantasyon: $($missingDocs -join ', ')"
}

# 9. İkon kontrolü
Write-Host "[9/10] İkon dosyası kontrolü..." -NoNewline
$iconPath = "$rootDir\installer\assets\rogctl.ico"
if (Test-Path $iconPath) {
    Write-Host " ✓" -ForegroundColor Green
} else {
    Write-Host " ⚠" -ForegroundColor Yellow
    $warnings += "rogctl.ico bulunamadı (varsayılan ikon kullanılacak)"
}

# 10. Release dizini kontrolü
Write-Host "[10/10] Release dizini kontrolü..." -NoNewline
$releaseDir = "$rootDir\release"
if (-not (Test-Path $releaseDir)) {
    New-Item -ItemType Directory -Path $releaseDir -Force | Out-Null
    Write-Host " ✓ (oluşturuldu)" -ForegroundColor Green
} else {
    Write-Host " ✓" -ForegroundColor Green
}

Write-Host ""
Write-Host "══════════════════════════════════════════════════" -ForegroundColor Cyan

# Sonuçlar
if ($errors.Count -eq 0 -and $warnings.Count -eq 0) {
    Write-Host "✓ TÜM KONTROLLER BAŞARILI!" -ForegroundColor Green
    Write-Host ""
    Write-Host "Kurulum paketi oluşturmaya hazırsınız:" -ForegroundColor White
    Write-Host "  .\build-installer.ps1" -ForegroundColor Cyan
    Write-Host ""
    exit 0
} elseif ($errors.Count -eq 0) {
    Write-Host "⚠ UYARILAR VAR (devam edilebilir)" -ForegroundColor Yellow
    Write-Host ""
    foreach ($warn in $warnings) {
        Write-Host "  • $warn" -ForegroundColor Yellow
    }
    Write-Host ""
    Write-Host "Temel kurulum paketi oluşturulabilir:" -ForegroundColor White
    Write-Host "  .\build-installer.ps1" -ForegroundColor Cyan
    Write-Host ""
    exit 0
} else {
    Write-Host "✗ KRİTİK HATALAR VAR!" -ForegroundColor Red
    Write-Host ""
    Write-Host "Hatalar:" -ForegroundColor Red
    foreach ($err in $errors) {
        Write-Host "  ✗ $err" -ForegroundColor Red
    }
    Write-Host ""
    if ($warnings.Count -gt 0) {
        Write-Host "Uyarılar:" -ForegroundColor Yellow
        foreach ($warn in $warnings) {
            Write-Host "  • $warn" -ForegroundColor Yellow
        }
        Write-Host ""
    }
    Write-Host "Önce hataları düzeltin, sonra tekrar deneyin." -ForegroundColor Red
    Write-Host ""
    exit 1
}
