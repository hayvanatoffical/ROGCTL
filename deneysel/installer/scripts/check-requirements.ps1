# ROGCtl - Sistem Gereksinimlerini Kontrol Et
# Bu script kurulum öncesi sistem uygunluğunu kontrol eder

$ErrorActionPreference = "Continue"

Write-Host "=== ROGCtl Sistem Gereksinimleri Kontrolü ===" -ForegroundColor Cyan
Write-Host ""

$allChecks = @()

# 1. İşletim Sistemi Kontrolü
Write-Host "[1/5] İşletim sistemi kontrol ediliyor..." -NoNewline
$os = Get-CimInstance Win32_OperatingSystem
if ($os.Caption -match "Windows 10|Windows 11") {
    Write-Host " ✓ BAŞARILI" -ForegroundColor Green
    Write-Host "      $($os.Caption)" -ForegroundColor Gray
    $allChecks += $true
} else {
    Write-Host " ✗ BAŞARISIZ" -ForegroundColor Red
    Write-Host "      Desteklenen: Windows 10/11" -ForegroundColor Yellow
    $allChecks += $false
}

# 2. ASUS Sistem Kontrolü
Write-Host "[2/5] ASUS donanım kontrol ediliyor..." -NoNewline
$manufacturer = (Get-CimInstance Win32_ComputerSystem).Manufacturer
if ($manufacturer -match "ASUS|ASUSTeK") {
    Write-Host " ✓ BAŞARILI" -ForegroundColor Green
    $model = (Get-CimInstance Win32_ComputerSystem).Model
    Write-Host "      Model: $model" -ForegroundColor Gray
    $allChecks += $true
} else {
    Write-Host " ⚠ UYARI" -ForegroundColor Yellow
    Write-Host "      ASUS sistemi değil, bazı özellikler çalışmayabilir" -ForegroundColor Yellow
    $allChecks += $true  # Engellemesin, sadece uyarsın
}

# 3. NVIDIA Ekran Kartı Kontrolü
Write-Host "[3/5] NVIDIA ekran kartı kontrol ediliyor..." -NoNewline
$hasNvidia = $false
try {
    $gpu = Get-CimInstance Win32_VideoController | Where-Object { $_.Name -match "NVIDIA" }
    if ($gpu) {
        Write-Host " ✓ BAŞARILI" -ForegroundColor Green
        Write-Host "      $($gpu.Name)" -ForegroundColor Gray
        $hasNvidia = $true
        $allChecks += $true
    } else {
        throw "NVIDIA GPU bulunamadı"
    }
} catch {
    Write-Host " ✗ BAŞARISIZ" -ForegroundColor Red
    Write-Host "      NVIDIA ekran kartı bulunamadı" -ForegroundColor Yellow
    $allChecks += $false
}

# 4. NVIDIA Sürücü Kontrolü (NVML)
Write-Host "[4/5] NVIDIA sürücüsü kontrol ediliyor..." -NoNewline
if ($hasNvidia) {
    try {
        $nvidiaSmi = Get-Command nvidia-smi.exe -ErrorAction Stop
        $result = & nvidia-smi --query-gpu=driver_version --format=csv,noheader 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Host " ✓ BAŞARILI" -ForegroundColor Green
            Write-Host "      Sürücü Versiyonu: $result" -ForegroundColor Gray
            $allChecks += $true
        } else {
            throw "nvidia-smi çalışmadı"
        }
    } catch {
        Write-Host " ⚠ UYARI" -ForegroundColor Yellow
        Write-Host "      NVIDIA sürücüsü bulunamadı (GPU özellikleri sınırlı olacak)" -ForegroundColor Yellow
        $allChecks += $true  # GPU olmadan da çalışabilir
    }
} else {
    Write-Host " - ATLANDI" -ForegroundColor Gray
    $allChecks += $true
}

# 5. Yönetici Yetkisi Kontrolü
Write-Host "[5/5] Yönetici yetkisi kontrol ediliyor..." -NoNewline
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if ($isAdmin) {
    Write-Host " ✓ BAŞARILI" -ForegroundColor Green
    Write-Host "      Yükseltilmiş yetkilerle çalışıyor" -ForegroundColor Gray
    $allChecks += $true
} else {
    Write-Host " ✗ BAŞARISIZ" -ForegroundColor Red
    Write-Host "      Kurulum yönetici yetkisi gerektirir" -ForegroundColor Yellow
    $allChecks += $false
}

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan

# Sonuç değerlendirmesi
$criticalFailed = $false
if (-not $allChecks[0]) { $criticalFailed = $true }  # OS
if (-not $allChecks[4]) { $criticalFailed = $true }  # Admin

if ($criticalFailed) {
    Write-Host "KRİTİK HATALAR BULUNDU!" -ForegroundColor Red
    Write-Host "Kuruluma devam edilemiyor." -ForegroundColor Red
    exit 1
} elseif (-not $hasNvidia) {
    Write-Host "UYARI: NVIDIA özellikleri kullanılamayacak" -ForegroundColor Yellow
    Write-Host "Sadece fan kontrolü çalışacak, GPU saat kontrolü devre dışı" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Yine de devam edilsin mi? (E/H): " -NoNewline -ForegroundColor Yellow
    $response = Read-Host
    if ($response -notmatch "^[EeYy]") {
        exit 1
    }
    exit 0
} else {
    Write-Host "TÜM KONTROLLER BAŞARILI!" -ForegroundColor Green
    Write-Host "Kuruluma devam edilebilir." -ForegroundColor Green
    exit 0
}
