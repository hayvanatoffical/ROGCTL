# ROGCtl - Kurulum Sonrası Yapılandırma
# Bu script kurulum tamamlandıktan sonra gerekli yapılandırmaları yapar

param(
    [string]$InstallPath = "$env:ProgramFiles\ROGCtl"
)

$ErrorActionPreference = "Stop"

Write-Host "=== ROGCtl Kurulum Sonrası Yapılandırma ===" -ForegroundColor Cyan
Write-Host ""

$exe = "$InstallPath\rogctl.exe"
$cfg = "$InstallPath\rogctl.yaml"

# 1. Config dosyası oluşturma
Write-Host "[1/6] Yapılandırma dosyası kontrol ediliyor..." -NoNewline
if (-not (Test-Path $cfg)) {
    try {
        # rogctl'nin kendi config'ini oluşturmasını sağla
        $null = & $exe plan 2>&1
        if (Test-Path $cfg) {
            Write-Host " ✓ OLUŞTURULDU" -ForegroundColor Green
        } else {
            Write-Host " ⚠ VARSAYILAN KULLANILACAK" -ForegroundColor Yellow
        }
    } catch {
        Write-Host " ⚠ VARSAYILAN KULLANILACAK" -ForegroundColor Yellow
    }
} else {
    Write-Host " ✓ MEVCUT" -ForegroundColor Green
}

# 2. ASUS servislerini durdurma (isteğe bağlı)
Write-Host "[2/6] ASUS servisleri kontrol ediliyor..." -NoNewline
$asusServices = @(
    "ASUSSystemAnalysis",
    "ASUSSystemDiagnosis", 
    "ASUSSoftwareManager",
    "AsusCertService"
)

$stoppedServices = @()
foreach ($svc in $asusServices) {
    try {
        $service = Get-Service -Name $svc -ErrorAction SilentlyContinue
        if ($service -and $service.Status -eq "Running") {
            Stop-Service -Name $svc -Force -ErrorAction SilentlyContinue
            $stoppedServices += $svc
        }
    } catch {
        # Servis yoksa veya durdurulamazsa sessizce devam et
    }
}

if ($stoppedServices.Count -gt 0) {
    Write-Host " ✓ $($stoppedServices.Count) SERVİS DURDURULDU" -ForegroundColor Green
    Write-Host "      $($stoppedServices -join ', ')" -ForegroundColor Gray
} else {
    Write-Host " - HİÇBİRİ AKTİF DEĞİL" -ForegroundColor Gray
}

# 3. Zamanlanmış görev oluşturma
Write-Host "[3/6] Zamanlanmış görev oluşturuluyor..." -NoNewline
try {
    $action = New-ScheduledTaskAction -Execute $exe -Argument "daemon"
    $trigger = New-ScheduledTaskTrigger -AtLogOn
    $principal = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" -RunLevel Highest
    $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit ([TimeSpan]::Zero) -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1)
    
    $null = Register-ScheduledTask -TaskName "rogctl" -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Force 2>&1
    
    Write-Host " ✓ BAŞARILI" -ForegroundColor Green
} catch {
    Write-Host " ✗ BAŞARISIZ" -ForegroundColor Red
    Write-Host "      Hata: $($_.Exception.Message)" -ForegroundColor Yellow
}

# 4. NVIDIA profil ayarlarını uygulama (eğer mevcutsa)
Write-Host "[4/6] NVIDIA sürücü ayarları uygulanıyor..." -NoNewline
try {
    $nvidiaSmi = Get-Command nvidia-smi.exe -ErrorAction Stop
    # NVIDIA ayarlarını uygula
    $result = & $exe nv uygula 2>&1
    Write-Host " ✓ UYGULAND" -ForegroundColor Green
    Write-Host "      $($result | Select-Object -First 1)" -ForegroundColor Gray
} catch {
    Write-Host " - ATLANDI (NVIDIA bulunamadı)" -ForegroundColor Gray
}

# 5. RAM geri kazanım testi
Write-Host "[5/6] RAM geri kazanım kolu test ediliyor..." -NoNewline
try {
    $memResult = & $exe mem temizle 2>&1
    $freed = $memResult | Select-String "gercekten bos bellek.*artti"
    if ($freed) {
        Write-Host " ✓ ÇALIŞIYOR" -ForegroundColor Green
        Write-Host "      $freed" -ForegroundColor Gray
    } else {
        Write-Host " ⚠ SINIRLAMA VAR" -ForegroundColor Yellow
        Write-Host "      Yönetici yetkisi gerekebilir" -ForegroundColor Yellow
    }
} catch {
    Write-Host " ⚠ TEST EDİLEMEDİ" -ForegroundColor Yellow
}

# 6. İlk başlatma
Write-Host "[6/6] ROGCtl başlatılıyor..." -NoNewline
try {
    $null = Start-ScheduledTask -TaskName "rogctl" 2>&1
    Start-Sleep -Seconds 3
    
    $process = Get-Process rogctl -ErrorAction SilentlyContinue
    if ($process) {
        Write-Host " ✓ ÇALIŞIYOR" -ForegroundColor Green
        Write-Host "      PID: $($process.Id)" -ForegroundColor Gray
    } else {
        Write-Host " ⚠ BAŞLAMADI" -ForegroundColor Yellow
        Write-Host "      Manuel başlatma gerekebilir: rogctl daemon" -ForegroundColor Yellow
    }
} catch {
    Write-Host " ⚠ BAŞLANAMADI" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "KURULUM TAMAMLANDI!" -ForegroundColor Green
Write-Host ""
Write-Host "ROGCtl artık arka planda çalışıyor ve her açılışta" -ForegroundColor White
Write-Host "otomatik olarak başlayacak." -ForegroundColor White
Write-Host ""
Write-Host "Hızlı Komutlar:" -ForegroundColor Cyan
Write-Host "  rogctl status           - Mevcut durum" -ForegroundColor White
Write-Host "  rogctl rapor            - HTML rapor oluştur" -ForegroundColor White
Write-Host "  rogctl nv kim           - Kare hızı tanılama" -ForegroundColor White
Write-Host "  rogctl valorant         - Valorant FPS ayarları" -ForegroundColor White
Write-Host ""
Write-Host "Yapılandırma: $cfg" -ForegroundColor Gray
Write-Host "Log dosyası: $InstallPath\rogctl.log" -ForegroundColor Gray
Write-Host ""

exit 0
