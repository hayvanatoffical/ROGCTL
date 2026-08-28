# ROGCtl - Kaldırma Temizliği
# Bu script kaldırma işlemi sırasında gerekli temizlikleri yapar

param(
    [string]$InstallPath = "$env:ProgramFiles\ROGCtl"
)

$ErrorActionPreference = "Continue"

Write-Host "=== ROGCtl Kaldırma İşlemi ===" -ForegroundColor Cyan
Write-Host ""

# 1. Çalışan işlemi durdur
Write-Host "[1/5] ROGCtl işlemi durduruluyor..." -NoNewline
$processes = Get-Process rogctl -ErrorAction SilentlyContinue
if ($processes) {
    try {
        # Önce nazikçe kapat
        $processes | ForEach-Object { $_.CloseMainWindow() | Out-Null }
        Start-Sleep -Seconds 2
        
        # Hala çalışıyorsa zorla kapat
        $stillRunning = Get-Process rogctl -ErrorAction SilentlyContinue
        if ($stillRunning) {
            Stop-Process -Name rogctl -Force -ErrorAction Stop
        }
        Write-Host " ✓ DURDURULDU" -ForegroundColor Green
    } catch {
        Write-Host " ⚠ SORUN OLUŞTU" -ForegroundColor Yellow
    }
} else {
    Write-Host " - ZATEN KAPALI" -ForegroundColor Gray
}

# 2. Zamanlanmış görevi sil
Write-Host "[2/5] Zamanlanmış görev siliniyor..." -NoNewline
try {
    $task = Get-ScheduledTask -TaskName "rogctl" -ErrorAction SilentlyContinue
    if ($task) {
        Unregister-ScheduledTask -TaskName "rogctl" -Confirm:$false -ErrorAction Stop
        Write-Host " ✓ SİLİNDİ" -ForegroundColor Green
    } else {
        Write-Host " - BULUNAMADI" -ForegroundColor Gray
    }
} catch {
    Write-Host " ⚠ SİLİNEMEDİ" -ForegroundColor Yellow
}

# 3. GPU saat kilidini serbest bırak
Write-Host "[3/5] GPU saat kilidi serbest bırakılıyor..." -NoNewline
try {
    $nvidiaSmi = Get-Command nvidia-smi.exe -ErrorAction Stop
    $result = & nvidia-smi -rgc 2>&1
    Write-Host " ✓ SERBEST BIRAKILDI" -ForegroundColor Green
} catch {
    Write-Host " - NVIDIA BULUNAMADI" -ForegroundColor Gray
}

# 4. NVIDIA sürücü ayarlarını geri al
Write-Host "[4/5] NVIDIA sürücü ayarları geri alınıyor..." -NoNewline
$exe = "$InstallPath\rogctl.exe"
if (Test-Path $exe) {
    try {
        $result = & $exe nv sifirla 2>&1
        Write-Host " ✓ GERİ ALINDI" -ForegroundColor Green
    } catch {
        Write-Host " ⚠ GERİ ALINAMADI" -ForegroundColor Yellow
    }
} else {
    Write-Host " - ATLANDI" -ForegroundColor Gray
}

# 5. ASUS servislerini yeniden başlat (eğer durdurulmuşlarsa)
Write-Host "[5/5] ASUS servisleri yeniden başlatılıyor..." -NoNewline
$asusServices = @(
    "ASUSSystemAnalysis",
    "ASUSSystemDiagnosis",
    "ASUSSoftwareManager", 
    "AsusCertService"
)

$restarted = @()
foreach ($svc in $asusServices) {
    try {
        $service = Get-Service -Name $svc -ErrorAction SilentlyContinue
        if ($service -and $service.Status -eq "Stopped") {
            Start-Service -Name $svc -ErrorAction SilentlyContinue
            $restarted += $svc
        }
    } catch {
        # Sessizce devam et
    }
}

if ($restarted.Count -gt 0) {
    Write-Host " ✓ $($restarted.Count) SERVİS BAŞLATILDI" -ForegroundColor Green
    Write-Host "      $($restarted -join ', ')" -ForegroundColor Gray
} else {
    Write-Host " - GEREKMEDİ" -ForegroundColor Gray
}

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "KALDIRMA TAMAMLANDI!" -ForegroundColor Green
Write-Host ""
Write-Host "ROGCtl sistemden kaldırıldı." -ForegroundColor White
Write-Host "Fan eğrileri Armoury Crate'e geri bırakıldı." -ForegroundColor White
Write-Host ""

# Kullanıcıya config dosyalarını silme seçeneği sun
Write-Host "Yapılandırma dosyalarını da silmek ister misiniz?" -ForegroundColor Yellow
Write-Host "(rogctl.yaml, log dosyaları vb.)" -ForegroundColor Yellow
Write-Host ""
Write-Host "E - Evet, tümünü sil" -ForegroundColor White
Write-Host "H - Hayır, yapılandırmayı koru (varsayılan)" -ForegroundColor White
Write-Host ""
Write-Host "Seçiminiz (E/H): " -NoNewline -ForegroundColor Cyan

# Kullanıcıdan giriş bekleyelim (5 saniye timeout)
$choice = $null
$timeout = 5
$timer = [Diagnostics.Stopwatch]::StartNew()

while ($timer.Elapsed.TotalSeconds -lt $timeout -and -not $choice) {
    if ([Console]::KeyAvailable) {
        $key = [Console]::ReadKey($true)
        if ($key.Key -eq 'E' -or $key.Key -eq 'Y') {
            $choice = 'E'
        } elseif ($key.Key -eq 'H' -or $key.Key -eq 'N' -or $key.Key -eq 'Enter') {
            $choice = 'H'
        }
    }
    Start-Sleep -Milliseconds 100
}

if ($choice -eq 'E') {
    Write-Host "E" -ForegroundColor Green
    Write-Host ""
    Write-Host "Yapılandırma dosyaları siliniyor..." -ForegroundColor Yellow
    
    # Yapılandırma dosyalarını sil
    $filesToDelete = @(
        "$InstallPath\rogctl.yaml",
        "$InstallPath\rogctl.yaml.eski",
        "$InstallPath\rogctl.log",
        "$InstallPath\rogctl.log.1",
        "$InstallPath\status.txt",
        "$InstallPath\rapor.html",
        "$InstallPath\cool-ref.txt"
    )
    
    foreach ($file in $filesToDelete) {
        if (Test-Path $file) {
            Remove-Item $file -Force -ErrorAction SilentlyContinue
            Write-Host "  Silindi: $(Split-Path $file -Leaf)" -ForegroundColor Gray
        }
    }
    
    Write-Host ""
    Write-Host "Tüm yapılandırma dosyaları silindi." -ForegroundColor Green
} else {
    Write-Host "H" -ForegroundColor Green
    Write-Host ""
    Write-Host "Yapılandırma dosyaları korundu:" -ForegroundColor White
    Write-Host "  $InstallPath\rogctl.yaml" -ForegroundColor Gray
    Write-Host "  $InstallPath\rogctl.log" -ForegroundColor Gray
}

Write-Host ""
Write-Host "ROGCtl'yi kullandığınız için teşekkürler!" -ForegroundColor Cyan
Write-Host ""

exit 0
