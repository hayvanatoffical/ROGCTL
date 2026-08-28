# rogctl - yonetici yetkisi gerektiren tani testi
#
# Uc soruyu tek seferde cevaplar:
#   1. Armoury Crate servisleri durunca CPU guc limiti (PPT) tutuyor mu?
#   2. Performans modu yazilinca kaliyor mu?
#   3. NVML yazma yollari (GPU guc limiti, clock lock) yonetici ile calisiyor mu?
#
# Durdurdugu her servisi sonunda geri baslatir - hata alsa bile.

$ErrorActionPreference = "Continue"
$exe = "C:\Users\Muhammed_ali\rogctl\target\release\rogctl.exe"

if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host "Bu betik YONETICI olarak calistirilmali." -ForegroundColor Red
    Write-Host "PowerShell'i yonetici olarak ac ve tekrar dene."
    exit 1
}

if (-not (Test-Path $exe)) {
    Write-Host "rogctl.exe bulunamadi: $exe" -ForegroundColor Red
    exit 1
}

# Isi/guc yazan servisler. Aura ve klavye servislerine dokunmuyoruz.
$targets = @("ArmouryCrateService", "ASUSOptimization", "ArmouryCrateControlInterface")
$stopped = @()

function Start-Load {
    param([int]$Seconds)
    1..14 | ForEach-Object {
        Start-Job -ScriptBlock {
            param($s)
            $e = (Get-Date).AddSeconds($s); $x = 1.0
            while ((Get-Date) -lt $e) { for ($i = 0; $i -lt 300000; $i++) { $x = [math]::Sqrt($x + $i) } }
        } -ArgumentList $Seconds
    }
}

try {
    Write-Host "`n=== 0. BASLANGIC DURUMU ===" -ForegroundColor Cyan
    & $exe ppt
    & $exe perf

    Write-Host "`n=== 1. ARMOURY CRATE SERVISLERI DURDURULUYOR ===" -ForegroundColor Cyan
    foreach ($name in $targets) {
        $svc = Get-Service -Name $name -ErrorAction SilentlyContinue
        if ($svc -and $svc.Status -eq "Running") {
            try {
                Stop-Service -Name $name -Force -ErrorAction Stop
                $stopped += $name
                Write-Host "  [+] $name durduruldu"
            } catch {
                Write-Host "  [-] $name durdurulamadi: $($_.Exception.Message)" -ForegroundColor Yellow
            }
        } else {
            Write-Host "  [.] $name zaten calismiyor"
        }
    }
    Start-Sleep -Seconds 3

    Write-Host "`n=== 2. PERFORMANS MODU YAZMA TESTI ===" -ForegroundColor Cyan
    Write-Host "AC durmusken PERF_MODE degistirilebiliyor mu?"
    & $exe perf 2

    Write-Host "`n=== 3. PPT TESTI: 25W altinda tam yuk ===" -ForegroundColor Cyan
    Write-Host "PPT gercekten uygulanirsa CPU 95C'de cakili KALMAMALI."
    & $exe ppt 25
    $jobs = Start-Load -Seconds 20
    & $exe mon 16
    $jobs | Stop-Job -ErrorAction SilentlyContinue
    $jobs | Remove-Job -Force -ErrorAction SilentlyContinue

    Write-Host "`n--- karsilastirma: PPT serbest (BIOS) altinda ayni yuk ---" -ForegroundColor Cyan
    & $exe ppt 0
    Start-Sleep -Seconds 5
    $jobs = Start-Load -Seconds 20
    & $exe mon 16
    $jobs | Stop-Job -ErrorAction SilentlyContinue
    $jobs | Remove-Job -Force -ErrorAction SilentlyContinue

    Write-Host "`n=== 4. NVML YAZMA TESTI (yonetici ile) ===" -ForegroundColor Cyan
    & $exe selftest
}
finally {
    Write-Host "`n=== TEMIZLIK ===" -ForegroundColor Cyan
    & $exe ppt 0 | Out-Null
    & $exe perf 3 | Out-Null
    Write-Host "  [+] PPT ve performans modu BIOS'a birakildi"

    foreach ($name in $stopped) {
        try {
            Start-Service -Name $name -ErrorAction Stop
            Write-Host "  [+] $name geri baslatildi"
        } catch {
            Write-Host "  [!] $name BASLATILAMADI: $($_.Exception.Message)" -ForegroundColor Red
            Write-Host "      Elle baslat:  Start-Service $name" -ForegroundColor Yellow
        }
    }
    Write-Host "`nTest bitti." -ForegroundColor Green
}
