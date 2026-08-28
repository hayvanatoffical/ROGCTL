# ROGCtl - GUI Template Dosyalarını Kopyalama Scripti
# GUI projesi oluşturulduktan sonra bu scripti çalıştırın

$ErrorActionPreference = "Stop"

Write-Host "══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  ROGCtl - GUI Template Kopyalayıcı" -ForegroundColor Cyan
Write-Host "══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

$rootDir = $PSScriptRoot
$guiDir = "$rootDir\rogctl-gui"
$templateDir = "$rootDir\gui-templates"

# GUI projesi var mı kontrol et
if (-not (Test-Path $guiDir)) {
    Write-Host "✗ rogctl-gui klasörü bulunamadı!" -ForegroundColor Red
    Write-Host ""
    Write-Host "Önce GUI projesini oluşturun:" -ForegroundColor Yellow
    Write-Host "  .\setup-gui.ps1" -ForegroundColor White
    Write-Host "  cd rogctl-gui" -ForegroundColor White
    Write-Host "  npm create tauri-app@latest ." -ForegroundColor White
    Write-Host ""
    exit 1
}

# Template dizini var mı kontrol et
if (-not (Test-Path $templateDir)) {
    Write-Host "✗ gui-templates klasörü bulunamadı!" -ForegroundColor Red
    exit 1
}

Write-Host "[1/3] Hedef dizinler hazırlanıyor..." -ForegroundColor Yellow

# Hedef dizinler
$srcDir = "$guiDir\src"
$tauriSrcDir = "$guiDir\src-tauri\src"

if (-not (Test-Path $srcDir)) {
    New-Item -ItemType Directory -Path $srcDir -Force | Out-Null
}

if (-not (Test-Path $tauriSrcDir)) {
    Write-Host "  ⚠ src-tauri/src klasörü bulunamadı. Tauri projesi oluşturuldu mu?" -ForegroundColor Yellow
}

Write-Host "  ✓ Dizinler hazır" -ForegroundColor Green
Write-Host ""

Write-Host "[2/3] Frontend dosyaları kopyalanıyor..." -ForegroundColor Yellow

$frontendFiles = @(
    @{ Source = "index.html"; Dest = "$srcDir\index.html" },
    @{ Source = "styles.css"; Dest = "$srcDir\styles.css" },
    @{ Source = "app.js"; Dest = "$srcDir\app.js" }
)

foreach ($file in $frontendFiles) {
    $source = "$templateDir\$($file.Source)"
    $dest = $file.Dest
    
    if (Test-Path $source) {
        Copy-Item -Path $source -Destination $dest -Force
        Write-Host "  ✓ $($file.Source) → $(Split-Path $dest -Leaf)" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $($file.Source) bulunamadı!" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "[3/3] Backend dosyaları kopyalanıyor..." -ForegroundColor Yellow

if (Test-Path $tauriSrcDir) {
    $backendFile = "$templateDir\tauri-commands.rs"
    
    if (Test-Path $backendFile) {
        Copy-Item -Path $backendFile -Destination "$tauriSrcDir\tauri_commands.rs" -Force
        Write-Host "  ✓ tauri-commands.rs → tauri_commands.rs" -ForegroundColor Green
        
        Write-Host ""
        Write-Host "  📝 ÖNEMLİ: main.rs dosyasını düzenlemeyi unutmayın!" -ForegroundColor Yellow
        Write-Host "  Dosya: $tauriSrcDir\main.rs" -ForegroundColor Gray
        Write-Host ""
        Write-Host "  Eklenecek satırlar:" -ForegroundColor White
        Write-Host "    mod tauri_commands;" -ForegroundColor Cyan
        Write-Host "    use tauri_commands::*;" -ForegroundColor Cyan
    } else {
        Write-Host "  ✗ tauri-commands.rs bulunamadı!" -ForegroundColor Red
    }
} else {
    Write-Host "  - Backend dosyaları atlandı (Tauri projesi hazır değil)" -ForegroundColor Gray
}

Write-Host ""
Write-Host "══════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "✓ Kopyalama tamamlandı!" -ForegroundColor Green
Write-Host ""
Write-Host "Sonraki adımlar:" -ForegroundColor Cyan
Write-Host ""
Write-Host "1. main.rs dosyasını düzenleyin:" -ForegroundColor White
Write-Host "   $tauriSrcDir\main.rs" -ForegroundColor Gray
Write-Host ""
Write-Host "2. Geliştirme modunda test edin:" -ForegroundColor White
Write-Host "   cd rogctl-gui" -ForegroundColor Gray
Write-Host "   npm run tauri dev" -ForegroundColor Gray
Write-Host ""
Write-Host "3. Production build:" -ForegroundColor White
Write-Host "   npm run tauri build" -ForegroundColor Gray
Write-Host ""
Write-Host "4. Kurulum paketi oluşturun:" -ForegroundColor White
Write-Host "   cd .." -ForegroundColor Gray
Write-Host "   .\build-installer.ps1" -ForegroundColor Gray
Write-Host ""

# main.rs örneğini göster
$mainRsExample = @"

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 main.rs İÇİN ÖRNEK KOD:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod tauri_commands;
use tauri_commands::*;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_status,
            generate_report,
            nvidia_check,
            valorant_fix,
            mem_cleanup,
            open_logs,
            open_config,
            open_config_folder,
            restart_daemon,
            set_mode
        ])
        .run(tauri::generate_context!())
        .expect("ROGCtl GUI başlatılamadı");
}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
"@

Write-Host $mainRsExample -ForegroundColor DarkGray
Write-Host ""

exit 0
