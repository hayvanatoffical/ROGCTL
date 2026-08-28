# rogctl - kalici kurulum
#
# Acilista YONETICI yetkisiyle baslayan bir gorev olusturur. Yonetici olmasi
# sart, cunku GPU clock lock (voltaj kolu) NVML uzerinden yalnizca yukseltilmis
# haklarla yazilabiliyor. Yetkisiz calisirken fan kontrolu calisir, clock lock
# calismaz.
#
# Kaldirmak icin:  .\install.ps1 -Uninstall

param([switch]$Uninstall)

$ErrorActionPreference = "Stop"
# Kok, betigin kendi konumu. Sabit bir yol yazmak, proje klasoru
# tasindiginda kayitli gorevin var olmayan bir exe'yi gostermesine ve
# servisin sessizce hic acilmamasina yol aciyordu.
$root  = $PSScriptRoot
$build = "$root\target\release\rogctl.exe"
# Kurulum derleme agacinin disinda durur: 'cargo clean' config ve loglari
# silmesin, yeniden derleme calisan daemon'un exe'sini kilitlemesin.
$dir   = "$root\bin"
$exe   = "$dir\rogctl.exe"
# Arayuz ayni bin klasorunde durur: veri.rs kok dizini exe'nin yanindan
# bulur, yani status.txt ve rogctl.yaml ile ayni yerde olmasi sart.
$guiBuild = "$root\target\release\rogctl-gui.exe"
$guiExe   = "$dir\rogctl-gui.exe"
$kisayol  = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\rogctl.lnk"
$masaustu = "$([Environment]::GetFolderPath('Desktop'))\rogctl.lnk"
$task  = "rogctl"

# Yonetici sart: kayitli gorev en yuksek seviyede kosuyor ve ASUS termal
# servislerini durdurmak icin 'sc stop' gerekiyor. Hata verip cikmak yerine
# kendini yukseltiyor - yetkisiz calisan bir kurulum, fan egrisini Armoury
# Crate'e geri kaptirdigi icin sessizce yarim is yapardi.
if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host "Yonetici yetkisi gerekiyor - UAC penceresi acilacak." -ForegroundColor Yellow
    $argl = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "`"$PSCommandPath`"")
    if ($Uninstall) { $argl += "-Uninstall" }
    try {
        $p = Start-Process powershell.exe -ArgumentList $argl -Verb RunAs -PassThru -Wait
        exit $p.ExitCode
    } catch {
        Write-Host "Yukseltme reddedildi. Betigi yonetici kabuktan calistir." -ForegroundColor Red
        exit 1
    }
}

# Native araclarin stderr'i PowerShell'de NativeCommandError'a donusup
# ErrorActionPreference=Stop ile betigi olduruyor. Var olmayan bir anahtari
# silmek normal bir durum oldugu icin bu islerde PowerShell yerlisi kullaniliyor.
# Daemon'i oldurmek yerine durdurmasini iste.
#
# rogctl cikarken GPU saat kilidini birakir; 'Stop-Process -Force' ona bu
# firsati vermiyordu ve kilit surucu yeniden yuklenene kadar kaliyordu. Boyle
# birakilan bir bosta kilidi (~900MHz) karti ucte birine dusurur ve bunu
# aciklayan hicbir sey ekranda gorunmez - kurulumun kendisi bu hataya sebep
# oluyordu. CloseMainWindow yoksa gorevi durdurmak temiz cikisi tetikler.
function Stop-RogctlNazikce {
    $vardi = $false
    Get-ScheduledTask -TaskName $task -ErrorAction SilentlyContinue | ForEach-Object {
        $vardi = $true
        Stop-ScheduledTask -TaskName $task -ErrorAction SilentlyContinue
    }
    if ($vardi) {
        for ($i = 0; $i -lt 20; $i++) {
            if (-not (Get-Process rogctl -ErrorAction SilentlyContinue)) { break }
            Start-Sleep -Milliseconds 250
        }
    }
    $kalan = Get-Process rogctl -ErrorAction SilentlyContinue
    if ($kalan) { $kalan | Stop-Process -Force }

    # Arayuz exe'yi kilitler: acik dururken uzerine kopyalama basarisiz olur.
    $arayuz = Get-Process rogctl-gui -ErrorAction SilentlyContinue
    if ($arayuz) { $arayuz | Stop-Process -Force }

    # Kilidi her durumda birak. Windows bir gorevi durdururken sureci
    # sonlandiriyor, ona temiz cikis sinyali vermiyor - yani 'nazikce durdu'
    # varsayimi guvenli degil. Kilit zaten yoksa bu komut zararsiz.
    try { & nvidia-smi -rgc 2>&1 | Out-Null } catch { }
}

function Remove-AutoRun {
    Remove-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" `
        -Name rogctl -ErrorAction SilentlyContinue
}

if ($Uninstall) {
    Unregister-ScheduledTask -TaskName $task -Confirm:$false -ErrorAction SilentlyContinue
    Remove-AutoRun
    Stop-RogctlNazikce

    # Kurulumda eklenen PATH girdisini geri al; digerlerine dokunma.
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath) {
        $kept = @($userPath -split ';' | Where-Object { $_ -ne '' -and $_ -ne $dir })
        [Environment]::SetEnvironmentVariable("Path", ($kept -join ';'), "User")
    }

    Remove-Item $kisayol, $masaustu -Force -ErrorAction SilentlyContinue

    Write-Host "rogctl kaldirildi. Fan egrileri Armoury Crate'e birakildi." -ForegroundColor Green
    exit 0
}

if (-not (Test-Path $build)) {
    Write-Host "Derlenmis rogctl.exe bulunamadi: $build" -ForegroundColor Red
    Write-Host "Once derle:  cargo build --release --workspace --manifest-path $root\Cargo.toml"
    exit 1
}

# Ayni anda iki baslatma yolu olmasin.
Remove-AutoRun
Unregister-ScheduledTask -TaskName $task -Confirm:$false -ErrorAction SilentlyContinue
Stop-RogctlNazikce
Start-Sleep -Milliseconds 800

# Derlenen ikiliyi kalici konuma kur, mevcut ayarlari tasi.
New-Item -ItemType Directory -Path $dir -Force | Out-Null
Copy-Item $build $exe -Force
foreach ($f in @("rogctl.yaml")) {
    $old = "$root\target\release\$f"
    if ((Test-Path $old) -and -not (Test-Path "$dir\$f")) {
        Copy-Item $old "$dir\$f" -Force
        Write-Host "  [+] mevcut $f tasindi"
    }
}

# Arayuz zorunlu degil: daemon onsuz da calisir. Derlenmemisse kurulum
# yarim kalmaz, sadece kisayol olusturulmaz.
if (Test-Path $guiBuild) {
    Copy-Item $guiBuild $guiExe -Force

    # Tek tiklama ile acilsin - komut yazmak gerekmesin. Arayuz kendini
    # UAC ile yukselttigi icin kisayolun yonetici bayragina ihtiyaci yok.
    $sh = New-Object -ComObject WScript.Shell
    foreach ($yol in @($kisayol, $masaustu)) {
        $lnk = $sh.CreateShortcut($yol)
        $lnk.TargetPath       = $guiExe
        $lnk.WorkingDirectory = $dir
        $lnk.Description      = 'rogctl - termal ve guc denetimi'
        $lnk.Save()
    }
    Write-Host "  [+] arayuz kuruldu, Baslat menusune ve masaustune kisayol eklendi"
} else {
    Write-Host "  [!] rogctl-gui.exe derlenmemis - arayuz kurulmadi" -ForegroundColor Yellow
    Write-Host "      derlemek icin: cargo build --release --workspace"
}

$cfg = "$dir\rogctl.yaml"

# Var olan bir config dosyasi asla ustune yazilmaz - bu, elle yapilan
# duzenlemeleri korur ama olculerek bulunmus varsayilan duzeltmelerinin hicbir
# zaman ulasmamasi demektir. Surum damgasi tutmuyorsa eskisini YEDEKLEYIP
# yenisini urettiriyoruz.
$surum = 6
if ((Test-Path $cfg) -and -not ((Get-Content $cfg -Raw) -match "(?m)^surum:\s*$surum\b")) {
    $bak = "$dir\rogctl.yaml.eski"
    Move-Item $cfg $bak -Force
    & $exe plan | Out-Null
    if (Test-Path $cfg) {
        Write-Host "  [+] config surum $surum'e yenilendi (eskisi: rogctl.yaml.eski)"
    } else {
        Move-Item $bak $cfg -Force
        Write-Host "  [!] yeni config uretilemedi, eskisi geri konuldu" -ForegroundColor Yellow
    }
}

# Eski bir config dosyasinda 'memory' bolumu yoktur. Kod varsayilanlarla
# calisir ama dosyada gorunmedigi surece ayarlanamaz. Bloku sonuna ekliyoruz:
# mevcut icerige ve yorumlara dokunmadan, sadece eksigi tamamlayarak.
if ((Test-Path $cfg) -and -not ((Get-Content $cfg -Raw) -match '(?m)^memory:')) {
    $block = @'

# --- RAM geri kazanimi -------------------------------------------------------
# Windows standby listesinin dusuk oncelikli (0-4) kismini bosaltir; ise
# yarayan dosya onbellegi (oncelik 5) ve sicak onbellek (6-7) korunur.
# Sadece agir moda girerken/cikarken ve gercek bellek baskisinda calisir.
#   aggressive: true  -> tum standby + dosya onbellegi (cok daha fazla, bedelli)
memory:
  enabled: true
  on_mode_change: true
  min_free_mb: 2048
  min_standby_mb: 2048
  cooldown_s: 180
  aggressive: false
'@
    Add-Content -Path $cfg -Value $block -Encoding utf8
    Write-Host "  [+] rogctl.yaml'a memory bolumu eklendi"
}

if ((Test-Path $cfg) -and -not ((Get-Content $cfg -Raw) -match '(?m)^nvidia:')) {
    $block = @'

# --- NVIDIA surucu ayarlari --------------------------------------------------
# Surucu profiline yazilir, KALICIDIR. Sadece adi birimini soyleyen tamsayi
# ayarlar surulur; enum degerli olanlara (doku filtreleme, guc yonetimi)
# kasitli olarak dokunulmaz.
#   max_fps: 0      -> ekranin tazeleme hizini kullan
#   fps_headroom    -> YALNIZCA VRR (G-SYNC) KAPALIYKEN kullanilir: sinir,
#                      tazeleme hizinin bu kadar ustune kurulur (144 -> 151).
#                      Surucunun sinirlayicisi hedefin birkac kare altinda
#                      kaldigi icin, VRR yokken tam tazeleme hizina kurmak
#                      gercek kare hizini tazelemenin ALTINA dusuruyor.
#                      VRR ACIKKEN bu deger yok sayilir; asagiya bak.
#   respect_vrr     -> true: VRR aciksa sinir tazelemenin vrr_margin kadar
#                      ALTINA kurulur (144 -> 141). VRR penceresinin tavani
#                      tazeleme hizidir; ustune cikan kare pencereden duser ve
#                      ekran yirtilmasi (tearing) baslar.
#                      false: VRR acik olsa bile +fps_headroom kurali uygulanir
#                      (144 -> 151). Tercih senin: tearing'e karsi fazladan kare.
#   vrr_margin      -> VRR acikken tazelemenin kac kare altina kurulacagi.
#   refresh_hz: 0   -> paneli canli oku. Bir sayi yazarsan (ornegin 144) sinir
#                      HER ZAMAN o hizdan hesaplanir. Canli okuma dogru olani
#                      yapar ama oturum acilisinda panel 144'e gecmeden 60
#                      okunabiliyor; o anda hesaplanan sinir surucu profiline
#                      KALICI yaziliyor. Paneli degistirmeyeceksen sabitle.
#   idle_max_fps: 0 -> kapali. Surucunun "bosta" karari oyun oynarken de
#                      tetiklenip goruntuyu 30 fps'e kilitleyebiliyor.
# Geri almak icin:  rogctl nv sifirla
# Hangi profilin sinirladigini gormek icin:  rogctl nv kim
nvidia:
  enabled: true
  max_fps: 0
  fps_headroom: 7
  respect_vrr: true
  vrr_margin: 3
  refresh_hz: 0
  idle_max_fps: 0
  idle_timeout_s: 10
'@
    Add-Content -Path $cfg -Value $block -Encoding utf8
    Write-Host "  [+] rogctl.yaml'a nvidia bolumu eklendi"
}
# Var olan bir nvidia blokuna SONRADAN eklenen anahtarlar.
#
# Surum damgasi yalnizca varsayilan DEGERLER olcumle degistiginde artirilir;
# geriye donuk uyumlu yeni bir anahtar icin dosyayi bastan uretmek, kullanicinin
# elle yaptigi her duzenlemeyi de silmek demek olurdu. Bunun yerine eksik satir
# blogun icine cerrahi olarak ekleniyor. Anahtar kodda `#[serde(default)]`
# oldugu icin dosyada gorunmese de calisir - ama gorunmedigi surece
# AYARLANAMAZ, ki bu da onu var olmayan bir ozellik yapar.
if (Test-Path $cfg) {
    $raw = Get-Content $cfg -Raw
    if (($raw -match '(?m)^nvidia:') -and -not ($raw -match '(?m)^\s+refresh_hz:')) {
        $yorum = @(
            "#   refresh_hz: 0   -> paneli canli oku. Bir sayi yazarsan (ornegin 144) sinir",
            "#                      HER ZAMAN o hizdan hesaplanir. Canli okuma dogru olani",
            "#                      yapar ama oturum acilisinda panel 144'e gecmeden 60",
            "#                      okunabiliyor; o anda hesaplanan sinir surucu profiline",
            "#                      KALICI yaziliyor. Paneli degistirmeyeceksen sabitle."
        ) -join "`r`n"
        # Yorum nvidia blogunun basligindan ONCE, deger ise vrr_margin satirindan
        # SONRA gider. Ikisi de bulunamazsa dosyaya dokunulmaz.
        if ($raw -match '(?m)^\s+vrr_margin:.*$') {
            $raw = $raw -replace '(?m)^(\s+)(vrr_margin:.*)$', "`$1`$2`r`n`$1refresh_hz: 0"
            $raw = $raw -replace '(?m)^nvidia:', "$yorum`r`nnvidia:"
            Set-Content -Path $cfg -Value $raw -Encoding utf8 -NoNewline
            Write-Host "  [+] rogctl.yaml'a refresh_hz ayari eklendi"
        }
    }
}

Write-Host "  [+] kuruldu: $exe"

# 'rogctl' komutu her dizinden calissin. Makine geneli yerine KULLANICI
# kapsamina yaziliyor: kaldirmasi temiz ve baska hesaplari etkilemiyor.
# Degisiklik yalnizca bundan SONRA acilan kabuklarda gorunur - bu betigin
# icinde bulundugu oturum eski PATH'i tasimaya devam eder.
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not $userPath) { $userPath = "" }
# Tasinmis bir kurulumun eski bin klasoru PATH'te kalirsa 'rogctl' yazmak
# artik var olmayan bir exe'yi cagirir. Bize ait olup bu kuruluma ait olmayan
# girisler temizlenir; baskasinin yollarina dokunulmaz.
$parts = @($userPath -split ';' | Where-Object {
    $_ -ne '' -and -not ((Split-Path $_ -Leaf) -eq 'bin' -and (Split-Path (Split-Path $_ -Parent) -Leaf) -eq 'rogctl' -and $_ -ne $dir)
})
$hedef = @($parts)
if ($hedef -notcontains $dir) { $hedef += $dir }
# Tek yazma noktasi: temizlik tek basina kaldiginda da diske isliyor.
if (($hedef -join ';') -ne $userPath) {
    [Environment]::SetEnvironmentVariable("Path", ($hedef -join ';'), "User")
    Write-Host "  [+] PATH guncellendi: $dir"
    Write-Host "      (YENI bir PowerShell penceresinde 'rogctl rapor' yazabilirsin)"
}

# Standby listesini bosaltmak SeProfileSingleProcessPrivilege ister. Bu betik
# zaten yukseltilmis calistigi icin, kolun gercekten calisip calismadigini
# burada bir kez olcup gostermek en durust yer.
Write-Host ""
Write-Host "  RAM geri kazanim kolu dogrulaniyor..."
& $exe mem temizle | Select-String "gercekten bos|=> " | ForEach-Object { Write-Host "    $_" }

# Acilista, en yuksek yetkiyle, oturum acan kullanici icin.
$action  = New-ScheduledTaskAction -Execute $exe -Argument "daemon"
$trigger = New-ScheduledTaskTrigger -AtLogOn
$princ   = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" -RunLevel Highest
$set     = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit ([TimeSpan]::Zero) -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1)

Register-ScheduledTask -TaskName $task -Action $action -Trigger $trigger -Principal $princ -Settings $set -Force | Out-Null
Start-ScheduledTask -TaskName $task

Start-Sleep -Seconds 4
if (Get-Process rogctl -ErrorAction SilentlyContinue) {
    Write-Host "`nrogctl kuruldu ve YONETICI olarak calisiyor." -ForegroundColor Green
    Write-Host "Her acilista kendiliginden baslar; hicbir sey yapman gerekmez."
    Write-Host ""
    if (Test-Path $guiExe) {
        Write-Host "  arayuz  : Baslat menusunden 'rogctl' - her sey orada, komut gerekmez"
        Write-Host ""
    }
    Write-Host "  durum   : $exe status"
    Write-Host "  rapor   : $exe rapor      (HTML rapor uretir ve tarayicida acar)"
    Write-Host "  fps tani: $exe nv kim     (kare hizini hangi profil siniriyor)"
    Write-Host "            $exe nv izle 60 (oyun sirasinda tazeleme hizini olcer)"
    Write-Host "  valorant: $exe valorant   (oyunun KENDI fps sinirlarini gosterir)"
    Write-Host "            $exe valorant hepsi    (TUM hesaplarin sinirlarini gosterir)"
    Write-Host "            $exe valorant hepsi duzelt   <- sinirli, Valorant KAPALIYKEN"
    Write-Host "            $exe valorant hepsi serbest  <- sinir yok, fps serbest"
    Write-Host "  ayarlar : $dir\rogctl.yaml"
    Write-Host "  log     : $dir\rogctl.log"
} else {
    Write-Host "Gorev olusturuldu ama surec ayakta degil - log'a bak." -ForegroundColor Yellow
}
