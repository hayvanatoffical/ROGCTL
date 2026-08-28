# ROGCtl - Simple Build Script (No Emoji, ASCII only)
param(
    [switch]$SkipBuild,
    [string]$Version = "1.0.0"
)

$ErrorActionPreference = "Stop"

Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  ROGCtl - Kurulum Paketi Olusturucu" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

$rootDir = $PSScriptRoot
$targetDir = "$rootDir\target\release"
$releaseDir = "$rootDir\release"

# Release klasorunu olustur
if (-not (Test-Path $releaseDir)) {
    New-Item -ItemType Directory -Path $releaseDir -Force | Out-Null
}

# 1. Inno Setup yolunu bul
Write-Host "[1/4] Inno Setup bulunuyor..." -ForegroundColor Yellow
$isccPaths = @(
    "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
    "C:\Program Files\Inno Setup 6\ISCC.exe",
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
)
$isccPath = $null
foreach ($p in $isccPaths) {
    if (Test-Path $p) {
        $isccPath = $p
        Write-Host "  Bulundu: $p" -ForegroundColor Green
        break
    }
}

if (-not $isccPath) {
    Write-Host "  HATA: Inno Setup bulunamadi!" -ForegroundColor Red
    exit 1
}

# 2. CLI binary kontrolu
Write-Host ""
Write-Host "[2/4] CLI binary kontrolu..." -ForegroundColor Yellow
$cliPath = "$targetDir\rogctl.exe"
if (-not (Test-Path $cliPath)) {
    Write-Host "  HATA: rogctl.exe bulunamadi!" -ForegroundColor Red
    Write-Host "  Once derleyin: cargo build --release" -ForegroundColor Yellow
    exit 1
}

$size = (Get-Item $cliPath).Length / 1MB
Write-Host "  OK: rogctl.exe ($([math]::Round($size, 2)) MB)" -ForegroundColor Green

# 3. Setup script kontrolu
Write-Host ""
Write-Host "[3/4] Setup script kontrolu..." -ForegroundColor Yellow
$setupScript = "$rootDir\installer\setup.iss"
if (-not (Test-Path $setupScript)) {
    Write-Host "  HATA: setup.iss bulunamadi!" -ForegroundColor Red
    exit 1
}
Write-Host "  OK: setup.iss" -ForegroundColor Green

# 4. Kurulum paketi olustur
Write-Host ""
Write-Host "[4/4] Kurulum EXE'si olusturuluyor..." -ForegroundColor Yellow
Write-Host "  Inno Setup calistiriliyor..." -ForegroundColor Gray

try {
    $process = Start-Process -FilePath $isccPath -ArgumentList "`"$setupScript`"" -Wait -PassThru -NoNewWindow
    
    if ($process.ExitCode -eq 0) {
        Write-Host "  BASARILI!" -ForegroundColor Green
        Write-Host ""
        
        # Ciktiyi kontrol et
        $setupExe = Get-ChildItem "$releaseDir\ROGCtl-Setup-*.exe" -ErrorAction SilentlyContinue | 
                    Sort-Object LastWriteTime -Descending | 
                    Select-Object -First 1
        
        if ($setupExe) {
            $setupSize = $setupExe.Length / 1MB
            Write-Host "================================================" -ForegroundColor Cyan
            Write-Host "  KURULUM PAKETI HAZIR!" -ForegroundColor Green
            Write-Host "================================================" -ForegroundColor Cyan
            Write-Host ""
            Write-Host "  Dosya: $($setupExe.Name)" -ForegroundColor White
            Write-Host "  Boyut: $([math]::Round($setupSize, 2)) MB" -ForegroundColor White
            Write-Host "  Konum: $($setupExe.FullName)" -ForegroundColor Gray
            Write-Host ""
            Write-Host "Test etmek icin:" -ForegroundColor Cyan
            Write-Host "  $($setupExe.FullName)" -ForegroundColor White
            Write-Host ""
        } else {
            Write-Host "  UYARI: Setup EXE bulunamadi!" -ForegroundColor Yellow
        }
    } else {
        Write-Host "  HATA: Inno Setup basarisiz! (Exit code: $($process.ExitCode))" -ForegroundColor Red
        exit 1
    }
} catch {
    Write-Host "  HATA: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

exit 0
