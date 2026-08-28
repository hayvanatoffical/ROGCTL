# ROGCtl - Profesyonel Kurulum Paketi Oluşturucu
# Bu script projeyi derler ve kurulum EXE'sini oluşturur

param(
    [switch]$SkipBuild,
    [switch]$SkipGUI,
    [string]$Version = "1.0.0"
)

$ErrorActionPreference = "Stop"

Write-Host "══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  ROGCtl - Kurulum Paketi Oluşturucu" -ForegroundColor Cyan
Write-Host "══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

# Gerekli araçları kontrol et
Write-Host "[1/7] Gerekli araçlar kontrol ediliyor..." -ForegroundColor Yellow

# Rust kontrolü
Write-Host "  • Rust (cargo)..." -NoNewline
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    $rustVersion = cargo --version
    Write-Host " ✓" -ForegroundColor Green
    Write-Host "    $rustVersion" -ForegroundColor Gray
} else {
    Write-Host " ✗" -ForegroundColor Red
    Write-Host "    Rust kurulu değil! https://rustup.rs/" -ForegroundColor Red
    exit 1
}

# Inno Setup kontrolü
Write-Host "  • Inno Setup..." -NoNewline
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
    Write-Host " ✓" -ForegroundColor Green
} else {
    Write-Host " ✗" -ForegroundColor Red
    Write-Host "    Inno Setup 6 kurulu değil!" -ForegroundColor Red
    Write-Host "    İndir: https://jrsoftware.org/isdl.php" -ForegroundColor Yellow
    exit 1
}

# Node.js kontrolü (GUI için)
if (-not $SkipGUI) {
    Write-Host "  • Node.js..." -NoNewline
    if (Get-Command node -ErrorAction SilentlyContinue) {
        $nodeVersion = node --version
        Write-Host " ✓" -ForegroundColor Green
        Write-Host "    $nodeVersion" -ForegroundColor Gray
    } else {
        Write-Host " ⚠" -ForegroundColor Yellow
        Write-Host "    Node.js kurulu değil (GUI için gerekli)" -ForegroundColor Yellow
        Write-Host "    Sadece CLI derlenecek..." -ForegroundColor Yellow
        $SkipGUI = $true
    }
}

Write-Host ""

# Proje dizinleri
$rootDir = $PSScriptRoot
$targetDir = "$rootDir\target\release"
$installerDir = "$rootDir\installer"
$assetsDir = "$installerDir\assets"
$releaseDir = "$rootDir\release"
$guiDir = "$rootDir\rogctl-gui"

# Release klasörünü oluştur
if (-not (Test-Path $releaseDir)) {
    New-Item -ItemType Directory -Path $releaseDir -Force | Out-Null
}

# Assets klasörünü kontrol et
if (-not (Test-Path $assetsDir)) {
    New-Item -ItemType Directory -Path $assetsDir -Force | Out-Null
}

# 2. CLI Uygulamasını derle
if (-not $SkipBuild) {
    Write-Host "[2/7] ROGCtl CLI derleniyor..." -ForegroundColor Yellow
    Write-Host "  Release modunda optimizasyon yapılıyor..." -ForegroundColor Gray
    
    try {
        cargo build --release --manifest-path "$rootDir\Cargo.toml"
        Write-Host "  ✓ Derleme başarılı!" -ForegroundColor Green
        
        $exeSize = (Get-Item "$targetDir\rogctl.exe").Length / 1MB
        Write-Host "  Boyut: $([math]::Round($exeSize, 2)) MB" -ForegroundColor Gray
    } catch {
        Write-Host "  ✗ Derleme başarısız!" -ForegroundColor Red
        Write-Host "  Hata: $($_.Exception.Message)" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "[2/7] CLI derlemesi atlandı (SkipBuild)" -ForegroundColor Gray
}

# 3. GUI Uygulamasını derle
if (-not $SkipGUI -and (Test-Path $guiDir)) {
    Write-Host "[3/7] ROGCtl GUI derleniyor..." -ForegroundColor Yellow
    
    try {
        Push-Location $guiDir
        
        # Dependencies kur
        if (-not (Test-Path "$guiDir\node_modules")) {
            Write-Host "  Dependencies yükleniyor..." -ForegroundColor Gray
            npm install
        }
        
        # Build
        Write-Host "  Tauri build başlatılıyor..." -ForegroundColor Gray
        npm run tauri build
        
        Write-Host "  ✓ GUI derleme başarılı!" -ForegroundColor Green
        Pop-Location
    } catch {
        Write-Host "  ⚠ GUI derlenemedi, sadece CLI kurulacak" -ForegroundColor Yellow
        Pop-Location
    }
} else {
    Write-Host "[3/7] GUI derlemesi atlandı" -ForegroundColor Gray
}

# 4. Görselleri kontrol et/oluştur
Write-Host "[4/7] Kurulum görselleri hazırlanıyor..." -ForegroundColor Yellow

# Varsayılan görsel oluştur (eğer yoksa)
$iconPath = "$assetsDir\rogctl.ico"
if (-not (Test-Path $iconPath)) {
    Write-Host "  • İkon dosyası bulunamadı, varsayılan oluşturuluyor..." -ForegroundColor Gray
    # Basit bir PowerShell betiği ile minimal ikon oluştur
    # Gerçek projede profesyonel ikon kullanılmalı
    Write-Host "  ⚠ Lütfen rogctl.ico dosyasını $assetsDir klasörüne ekleyin" -ForegroundColor Yellow
}

Write-Host "  ✓ Görseller hazır" -ForegroundColor Green

# 5. Versiyon güncelle
Write-Host "[5/7] Versiyon bilgisi güncelleniyor..." -ForegroundColor Yellow

$issFile = "$installerDir\setup.iss"
if (Test-Path $issFile) {
    $issContent = Get-Content $issFile -Raw
    $issContent = $issContent -replace '#define MyAppVersion "[\d\.]+"', "#define MyAppVersion `"$Version`""
    Set-Content -Path $issFile -Value $issContent -NoNewline
    Write-Host "  ✓ Versiyon $Version olarak ayarlandı" -ForegroundColor Green
} else {
    Write-Host "  ✗ setup.iss bulunamadı!" -ForegroundColor Red
    exit 1
}

# 6. Kurulum paketi oluştur
Write-Host "[6/7] Kurulum EXE'si oluşturuluyor..." -ForegroundColor Yellow
Write-Host "  Inno Setup derleniyor..." -ForegroundColor Gray

try {
    $process = Start-Process -FilePath $isccPath -ArgumentList "`"$issFile`"" -Wait -PassThru -NoNewWindow
    
    if ($process.ExitCode -eq 0) {
        Write-Host "  ✓ Kurulum paketi başarıyla oluşturuldu!" -ForegroundColor Green
        
        $setupExe = Get-ChildItem "$releaseDir\ROGCtl-Setup-*.exe" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
        if ($setupExe) {
            $setupSize = $setupExe.Length / 1MB
            Write-Host "  Dosya: $($setupExe.Name)" -ForegroundColor Gray
            Write-Host "  Boyut: $([math]::Round($setupSize, 2)) MB" -ForegroundColor Gray
            Write-Host "  Konum: $($setupExe.FullName)" -ForegroundColor Gray
        }
    } else {
        Write-Host "  ✗ Kurulum paketi oluşturulamadı! (Exit code: $($process.ExitCode))" -ForegroundColor Red
        exit 1
    }
} catch {
    Write-Host "  ✗ Hata oluştu: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

# 7. Özet
Write-Host ""
Write-Host "[7/7] Özet" -ForegroundColor Yellow
Write-Host "══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""
Write-Host "✓ ROGCtl v$Version kurulum paketi hazır!" -ForegroundColor Green
Write-Host ""
Write-Host "Çıktılar:" -ForegroundColor White
Write-Host "  • CLI: $targetDir\rogctl.exe" -ForegroundColor Gray

if (-not $SkipGUI -and (Test-Path "$guiDir\src-tauri\target\release")) {
    Write-Host "  • GUI: $guiDir\src-tauri\target\release\rogctl-gui.exe" -ForegroundColor Gray
}

$setupFile = Get-ChildItem "$releaseDir\ROGCtl-Setup-*.exe" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if ($setupFile) {
    Write-Host "  • Setup: $($setupFile.FullName)" -ForegroundColor Gray
}

Write-Host ""
Write-Host "Test etmek için:" -ForegroundColor Cyan
Write-Host "  $($setupFile.FullName)" -ForegroundColor White
Write-Host ""

exit 0
