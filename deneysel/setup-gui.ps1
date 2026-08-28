# ROGCtl GUI Kurulum Script
# Bu script Tauri tabanlı GUI projesini oluşturur

$ErrorActionPreference = "Stop"

Write-Host "══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  ROGCtl GUI Projesi Oluşturucu" -ForegroundColor Cyan
Write-Host "══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

# Gereksinimleri kontrol et
Write-Host "[1/4] Gereksinimler kontrol ediliyor..." -ForegroundColor Yellow

# Node.js kontrolü
Write-Host "  • Node.js..." -NoNewline
if (Get-Command node -ErrorAction SilentlyContinue) {
    $nodeVersion = node --version
    Write-Host " ✓ $nodeVersion" -ForegroundColor Green
} else {
    Write-Host " ✗" -ForegroundColor Red
    Write-Host ""
    Write-Host "Node.js kurulu değil!" -ForegroundColor Red
    Write-Host "İndir: https://nodejs.org/" -ForegroundColor Yellow
    exit 1
}

# Rust kontrolü
Write-Host "  • Rust..." -NoNewline
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    $rustVersion = cargo --version | Select-Object -First 1
    Write-Host " ✓ $rustVersion" -ForegroundColor Green
} else {
    Write-Host " ✗" -ForegroundColor Red
    Write-Host ""
    Write-Host "Rust kurulu değil!" -ForegroundColor Red
    Write-Host "İndir: https://rustup.rs/" -ForegroundColor Yellow
    exit 1
}

Write-Host ""

# Proje dizini
$rootDir = $PSScriptRoot
$guiDir = "$rootDir\rogctl-gui"

Write-Host "[2/4] Tauri CLI yükleniyor..." -ForegroundColor Yellow
try {
    cargo install tauri-cli --locked
    Write-Host "  ✓ Tauri CLI hazır" -ForegroundColor Green
} catch {
    Write-Host "  ⚠ Tauri CLI zaten kurulu veya hata oluştu" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "[3/4] GUI proje yapısı oluşturuluyor..." -ForegroundColor Yellow

if (Test-Path $guiDir) {
    Write-Host "  ⚠ rogctl-gui klasörü zaten mevcut" -ForegroundColor Yellow
    Write-Host "  Üzerine yazmak için 'E', atlamak için 'H': " -NoNewline
    $response = Read-Host
    if ($response -ne 'E' -and $response -ne 'e') {
        Write-Host "  İşlem iptal edildi" -ForegroundColor Gray
        exit 0
    }
    Remove-Item -Path $guiDir -Recurse -Force
}

New-Item -ItemType Directory -Path $guiDir -Force | Out-Null
Write-Host "  ✓ Klasör oluşturuldu: $guiDir" -ForegroundColor Green

Write-Host ""
Write-Host "[4/4] Dosyalar oluşturuluyor..." -ForegroundColor Yellow
Write-Host "  Bu işlem birkaç dakika sürebilir..." -ForegroundColor Gray
Write-Host ""

Write-Host "NOT: Manuel adımlar gerekiyor!" -ForegroundColor Yellow
Write-Host ""
Write-Host "Şimdi şunları yapın:" -ForegroundColor Cyan
Write-Host "1. rogctl-gui klasörüne gidin:" -ForegroundColor White
Write-Host "   cd $guiDir" -ForegroundColor Gray
Write-Host ""
Write-Host "2. Tauri projesini başlatın:" -ForegroundColor White
Write-Host "   npm create tauri-app@latest" -ForegroundColor Gray
Write-Host ""
Write-Host "   Ayarlar:" -ForegroundColor White
Write-Host "   • App name: ROGCtl" -ForegroundColor Gray
Write-Host "   • Window title: ROGCtl - Termal Yönetim" -ForegroundColor Gray
Write-Host "   • UI template: Vanilla (veya React/Vue tercihinize göre)" -ForegroundColor Gray
Write-Host "   • TypeScript: Evet" -ForegroundColor Gray
Write-Host ""
Write-Host "3. Dependencies yükleyin:" -ForegroundColor White
Write-Host "   npm install" -ForegroundColor Gray
Write-Host ""
Write-Host "4. Geliştirme modunda çalıştırın:" -ForegroundColor White
Write-Host "   npm run tauri dev" -ForegroundColor Gray
Write-Host ""
Write-Host "Ben template dosyalarını hazırlıyorum..." -ForegroundColor Yellow
Write-Host "Script tamamlandıktan sonra yukarıdaki adımları uygulayın." -ForegroundColor Yellow
Write-Host ""

Write-Host "Devam etmek için Enter'a basın..." -ForegroundColor Cyan
Read-Host

Write-Host "✓ GUI kurulum script'i tamamlandı!" -ForegroundColor Green
Write-Host ""
Write-Host "Sonraki adımlar için GUI template dosyalarına bakın:" -ForegroundColor White
Write-Host "  • $rootDir\gui-templates\" -ForegroundColor Gray
Write-Host ""

exit 0
