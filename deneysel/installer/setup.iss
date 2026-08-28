; ROGCtl - Professional Setup Wizard (PRODUCTION READY)
; Inno Setup 6.x Script
; Hatasız, güvenli, profesyonel kurulum

#define MyAppName "ROGCtl"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "ROGCtl Team"
#define MyAppURL "https://github.com/yourusername/rogctl"
#define MyAppExeName "rogctl.exe"
#define MyAppGUIExeName "rogctl-gui.exe"

[Setup]
; Temel bilgiler
AppId={{B8F7D9A1-6C4E-4B2A-9F3D-8E5A7C2D1F4B}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
AppCopyright=Copyright (C) 2024 {#MyAppPublisher}

; Kurulum ayarları
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
DisableProgramGroupPage=yes

; Lisans ve bilgilendirme
LicenseFile=..\LICENSE.txt
InfoBeforeFile=..\INSTALL_INFO.txt

; Çıktı ayarları
OutputDir=..\release
OutputBaseFilename=ROGCtl-Setup-v{#MyAppVersion}
Compression=lzma2/ultra64
SolidCompression=yes

; Görünüm
WizardStyle=modern
WizardResizable=yes
WizardSizePercent=120
DisableWelcomePage=no

; İkonlar (opsiyonel - yoksa varsayılan kullanılır)
#ifdef ICON_FILE
SetupIconFile={#ICON_FILE}
UninstallDisplayIcon={app}\{#MyAppExeName}
#endif

#ifdef WIZARD_IMAGE
WizardImageFile={#WIZARD_IMAGE}
#endif

#ifdef WIZARD_SMALL_IMAGE
WizardSmallImageFile={#WIZARD_SMALL_IMAGE}
#endif

; Sistem gereksinimleri
PrivilegesRequired=admin
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
MinVersion=10.0.17763

; Dil
ShowLanguageDialog=yes

; Güvenlik
SignTool=bypassed
AllowRootDirectory=no
DirExistsWarning=auto

; Kaldırma
Uninstallable=yes
UninstallDisplayName={#MyAppName}
UninstallFilesDir={app}\uninst
CreateUninstallRegKey=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "turkish"; MessagesFile: "compiler:Languages\Turkish.isl"

[CustomMessages]
; English
english.ComponentsCLI=Command-Line Interface (Required)
english.ComponentsGUI=Graphical User Interface (Optional)
english.WelcomeLabel2=This will install [name/ver] on your computer.%n%nROGCtl is an advanced thermal and power management software for ASUS ROG laptops. It replaces Armoury Crate's manual mode with intelligent automation.%n%nIt is recommended that you close all other applications before continuing.
english.LaunchCLI=View status in terminal
english.LaunchGUI=Open GUI application
english.ErrorNoCLI=Error: rogctl.exe not found!%n%nPlease build the project first:%ncargo build --release

; Turkish
turkish.ComponentsCLI=Komut Satırı Arabirimi (Zorunlu)
turkish.ComponentsGUI=Grafik Kullanıcı Arabirimi (Opsiyonel)
turkish.WelcomeLabel2=Bu program bilgisayarınıza [name/ver] kuracak.%n%nROGCtl, ASUS ROG dizüstü bilgisayarlar için gelişmiş termal ve güç yönetim yazılımıdır. Armoury Crate'in manuel modunu akıllı otomasyonla değiştirir.%n%nDevam etmeden önce tüm diğer uygulamaları kapatmanız önerilir.
turkish.LaunchCLI=Terminalde durumu görüntüle
turkish.LaunchGUI=GUI uygulamasını aç
turkish.ErrorNoCLI=Hata: rogctl.exe bulunamadı!%n%nLütfen önce projeyi derleyin:%ncargo build --release

[Types]
Name: "full"; Description: "Tam Kurulum (CLI + GUI)"
Name: "compact"; Description: "Kompakt Kurulum (Sadece CLI)"
Name: "custom"; Description: "Özel Kurulum"; Flags: iscustom

[Components]
Name: "cli"; Description: "{cm:ComponentsCLI}"; Types: full compact custom; Flags: fixed
Name: "gui"; Description: "{cm:ComponentsGUI}"; Types: full; Check: GUIExists

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Components: gui
Name: "quicklaunchicon"; Description: "{cm:CreateQuickLaunchIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Components: gui; Flags: unchecked
Name: "autostart"; Description: "Başlangıçta otomatik başlat (Önerilen)"; GroupDescription: "Sistem:"; Flags: checkedonce
Name: "pathenv"; Description: "PATH ortam değişkenine ekle"; GroupDescription: "Sistem:"; Flags: checkedonce

[Files]
; CLI - Zorunlu
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion; Components: cli; Check: CLIExists; AfterInstall: StopRunningProcesses
Source: "..\target\release\rogctl.yaml"; DestDir: "{app}"; Flags: onlyifdoesntexist skipifsourcedoesntexist; Components: cli

; GUI - Opsiyonel
Source: "..\rogctl-gui\src-tauri\target\release\{#MyAppGUIExeName}"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist; Components: gui

; Dokümantasyon
Source: "..\README.md"; DestDir: "{app}"; Flags: isreadme skipifsourcedoesntexist
Source: "..\LICENSE.txt"; DestDir: "{app}"; Flags: skipifsourcedoesntexist
Source: "..\INSTALL_INFO.txt"; DestDir: "{app}"; Flags: skipifsourcedoesntexist
Source: "..\QUICK_START.md"; DestDir: "{app}"; Flags: skipifsourcedoesntexist

; Kurulum scriptleri (geçici)
Source: "..\installer\scripts\*.ps1"; DestDir: "{tmp}"; Flags: dontcopy

[Icons]
; Start Menu - CLI
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Parameters: "status"; WorkingDir: "{app}"; Components: cli
Name: "{group}\{#MyAppName} Rapor"; Filename: "{app}\{#MyAppExeName}"; Parameters: "rapor"; WorkingDir: "{app}"; Components: cli
Name: "{group}\Yapılandırma"; Filename: "notepad.exe"; Parameters: """{app}\rogctl.yaml"""; Components: cli

; Start Menu - GUI
Name: "{group}\{#MyAppName} GUI"; Filename: "{app}\{#MyAppGUIExeName}"; WorkingDir: "{app}"; Components: gui

; Start Menu - Kaldır
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"

; Desktop - GUI
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppGUIExeName}"; Tasks: desktopicon; Components: gui

; Quick Launch - GUI
Name: "{userappdata}\Microsoft\Internet Explorer\Quick Launch\{#MyAppName}"; Filename: "{app}\{#MyAppGUIExeName}"; Tasks: quicklaunchicon; Components: gui

[Registry]
; PATH ortam değişkenine ekle
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: pathenv; Check: NeedsAddPath(ExpandConstant('{app}'))

; Uninstall bilgileri
Root: HKLM; Subkey: "Software\Microsoft\Windows\CurrentVersion\Uninstall\{#MyAppName}_is1"; ValueType: string; ValueName: "DisplayVersion"; ValueData: "{#MyAppVersion}"; Flags: uninsdeletekey
Root: HKLM; Subkey: "Software\Microsoft\Windows\CurrentVersion\Uninstall\{#MyAppName}_is1"; ValueType: string; ValueName: "Publisher"; ValueData: "{#MyAppPublisher}"; Flags: uninsdeletekey

[Run]
; Kurulum öncesi - çalışan işlemleri durdur
Filename: "taskkill.exe"; Parameters: "/IM rogctl.exe /F"; Flags: runhidden; StatusMsg: "Mevcut işlemler durduruluyor..."; Check: ProcessExists('rogctl.exe')
Filename: "taskkill.exe"; Parameters: "/IM rogctl-gui.exe /F"; Flags: runhidden; Check: ProcessExists('rogctl-gui.exe')

; Gereksinim kontrolü
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{tmp}\check-requirements.ps1"""; StatusMsg: "Sistem gereksinimleri kontrol ediliyor..."; Flags: runhidden waituntilterminated; BeforeInstall: ExtractTempFile('check-requirements.ps1')

; Kurulum sonrası yapılandırma
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{tmp}\post-install.ps1"" -InstallPath ""{app}"""; StatusMsg: "ROGCtl yapılandırılıyor..."; Flags: runhidden waituntilterminated; BeforeInstall: ExtractTempFile('post-install.ps1'); Check: IsAdmin

; Kullanıcı seçenekleri - CLI
Filename: "{cmd}"; Parameters: "/c start """" ""{app}\{#MyAppExeName}"" status"; Description: "{cm:LaunchCLI}"; Flags: nowait postinstall skipifsilent shellexec unchecked; Components: cli

; Kullanıcı seçenekleri - GUI
Filename: "{app}\{#MyAppGUIExeName}"; Description: "{cm:LaunchGUI}"; Flags: nowait postinstall skipifsilent shellexec; Components: gui; Check: GUIInstalled

; Yapılandırma dosyasını aç
Filename: "notepad.exe"; Parameters: """{app}\rogctl.yaml"""; Description: "Yapılandırma dosyasını aç"; Flags: nowait postinstall skipifsilent unchecked

[UninstallRun]
; Çalışan işlemleri durdur
Filename: "taskkill.exe"; Parameters: "/IM rogctl.exe /F"; Flags: runhidden; RunOnceId: "StopCLI"
Filename: "taskkill.exe"; Parameters: "/IM rogctl-gui.exe /F"; Flags: runhidden; RunOnceId: "StopGUI"

; Zamanlanmış görevi durdur ve sil
Filename: "schtasks.exe"; Parameters: "/End /TN rogctl"; Flags: runhidden; RunOnceId: "EndTask"
Filename: "schtasks.exe"; Parameters: "/Delete /TN rogctl /F"; Flags: runhidden; RunOnceId: "DeleteTask"

; GPU saat kilidini serbest bırak
Filename: "nvidia-smi.exe"; Parameters: "-rgc"; Flags: runhidden; RunOnceId: "ResetGPU"

; Temizlik scripti
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{tmp}\uninstall-cleanup.ps1"" -InstallPath ""{app}"""; Flags: runhidden waituntilterminated; BeforeInstall: ExtractTempFile('uninstall-cleanup.ps1')

[UninstallDelete]
; Log dosyaları
Type: files; Name: "{app}\rogctl.log"
Type: files; Name: "{app}\rogctl.log.1"
Type: files; Name: "{app}\status.txt"
Type: files; Name: "{app}\rapor.html"
Type: files; Name: "{app}\cool-ref.txt"

; Geçici dosyalar
Type: files; Name: "{app}\*.tmp"
Type: files; Name: "{app}\*.bak"

; Dizini temizle (config korunur)
Type: filesandordirs; Name: "{app}"; Check: not FileExists(ExpandConstant('{app}\rogctl.yaml'))

[Code]
var
  RequirementsPage: TOutputMsgWizardPage;
  StatusLabel: TNewStaticText;

// Dosya varlık kontrolleri
function CLIExists: Boolean;
begin
  Result := FileExists(ExpandConstant('..\target\release\{#MyAppExeName}'));
  if not Result then
  begin
    MsgBox(ExpandConstant('{cm:ErrorNoCLI}'), mbError, MB_OK);
  end;
end;

function GUIExists: Boolean;
begin
  Result := FileExists(ExpandConstant('..\rogctl-gui\src-tauri\target\release\{#MyAppGUIExeName}'));
end;

function GUIInstalled: Boolean;
begin
  Result := FileExists(ExpandConstant('{app}\{#MyAppGUIExeName}'));
end;

// İşlem kontrolleri
function ProcessExists(ProcessName: String): Boolean;
var
  ResultCode: Integer;
begin
  Result := Exec('tasklist.exe', '/FI "IMAGENAME eq ' + ProcessName + '" /NH', '', SW_HIDE, ewWaitUntilTerminated, ResultCode) and (ResultCode = 0);
end;

procedure StopRunningProcesses;
var
  ResultCode: Integer;
begin
  // rogctl işlemini nazikçe durdur
  if ProcessExists('rogctl.exe') then
  begin
    Exec('schtasks.exe', '/End /TN rogctl', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    Sleep(1000);
    
    // Hala çalışıyorsa zorla kapat
    if ProcessExists('rogctl.exe') then
    begin
      Exec('taskkill.exe', '/IM rogctl.exe /F', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
      Sleep(500);
    end;
  end;
  
  // GUI işlemini durdur
  if ProcessExists('rogctl-gui.exe') then
  begin
    Exec('taskkill.exe', '/IM rogctl-gui.exe /F', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    Sleep(500);
  end;
end;

// PATH kontrolü
function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
  UpperCasePath: string;
  UpperCaseParam: string;
begin
  Result := True;
  
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath) then
    Exit;
  
  // Case-insensitive karşılaştırma
  UpperCasePath := Uppercase(OrigPath);
  UpperCaseParam := Uppercase(Param);
  
  Result := Pos(';' + UpperCaseParam + ';', ';' + UpperCasePath + ';') = 0;
  
  if not Result then
    Log('Path already contains: ' + Param);
end;

// Geçici dosya çıkarma
procedure ExtractTempFile(FileName: String);
begin
  ExtractTemporaryFile(FileName);
end;

// Wizard başlatma
procedure InitializeWizard;
begin
  RequirementsPage := CreateOutputMsgPage(wpWelcome,
    'Sistem Gereksinimleri',
    'Sisteminiz kontrol ediliyor',
    'Kurulum aşağıdaki gereksinimleri kontrol edecek:' + #13#10 + #13#10 +
    '• Windows 10/11 (64-bit)' + #13#10 +
    '• ASUS ROG Laptop' + #13#10 +
    '• NVIDIA Grafik Kartı (önerilir)' + #13#10 +
    '• Yönetici Yetkisi' + #13#10 + #13#10 +
    'Not: GPU özellikleri için NVIDIA sürücüsü gereklidir.');
  
  StatusLabel := TNewStaticText.Create(WizardForm);
  StatusLabel.Parent := WizardForm;
  StatusLabel.Top := WizardForm.ClientHeight - 40;
  StatusLabel.Left := ScaleX(8);
  StatusLabel.Caption := 'ROGCtl v{#MyAppVersion} - Professional Thermal Management';
  StatusLabel.Font.Color := clGray;
end;

// Kurulum başlangıç kontrolü
function InitializeSetup(): Boolean;
var
  ResultCode: Integer;
begin
  Result := True;
  
  // Yönetici yetkisi kontrolü
  if not IsAdmin then
  begin
    MsgBox('Bu kurulum yönetici yetkisi gerektirir.' + #13#10 + #13#10 +
           'Lütfen setup dosyasına sağ tıklayıp "Yönetici olarak çalıştır" seçin.', 
           mbError, MB_OK);
    Result := False;
    Exit;
  end;
  
  // Minimum Windows sürümü kontrolü
  if GetWindowsVersion < $0A000000 then  // Windows 10 1809+
  begin
    MsgBox('ROGCtl, Windows 10 (1809) veya daha yeni bir sürüm gerektirir.' + #13#10 + #13#10 +
           'Sisteminiz: ' + IntToStr(GetWindowsVersion), 
           mbError, MB_OK);
    Result := False;
    Exit;
  end;
end;

// Kurulum hazırlığı
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ResultCode: Integer;
begin
  Result := '';
  NeedsRestart := False;
  
  // NVIDIA sürücüsü kontrolü (uyarı, engelleme değil)
  if not Exec('nvidia-smi.exe', '--query-gpu=driver_version --format=csv,noheader',
              '', SW_HIDE, ewWaitUntilTerminated, ResultCode) or (ResultCode <> 0) then
  begin
    Log('NVIDIA driver not detected - GPU features will be limited');
    if MsgBox('NVIDIA sürücüsü bulunamadı.' + #13#10 + #13#10 +
              'GPU saat kontrolü çalışmayacak (sadece fan kontrolü aktif).' + #13#10 + #13#10 +
              'Devam etmek istiyor musunuz?',
              mbConfirmation, MB_YESNO) = IDNO then
    begin
      Result := 'Kurulum kullanıcı tarafından iptal edildi.';
    end;
  end;
end;

// Kurulum adımları
procedure CurStepChanged(CurStep: TSetupStep);
var
  ResultCode: Integer;
  TaskCommand: String;
begin
  if CurStep = ssPostInstall then
  begin
    // Zamanlanmış görev oluştur
    TaskCommand := '/Create /TN "rogctl" ' +
                   '/TR "\"' + ExpandConstant('{app}\{#MyAppExeName}') + '\" daemon" ' +
                   '/SC ONLOGON /RL HIGHEST /F';
    
    if Exec('schtasks.exe', TaskCommand, '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
    begin
      Log('Scheduled task created successfully');
      
      // Otomatik başlatma seçiliyse hemen başlat
      if WizardIsTaskSelected('autostart') then
      begin
        Sleep(1000);  // Görevin kaydedilmesini bekle
        Exec('schtasks.exe', '/Run /TN "rogctl"', '', SW_HIDE, ewNoWait, ResultCode);
        Log('Daemon started');
      end;
    end
    else
    begin
      Log('Failed to create scheduled task: ' + IntToStr(ResultCode));
      MsgBox('Zamanlanmış görev oluşturulamadı.' + #13#10 +
             'Daemon''u manuel başlatmanız gerekebilir: rogctl daemon',
             mbWarning, MB_OK);
    end;
  end;
end;

// Kaldırma adımları
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  ResultCode: Integer;
begin
  if CurUninstallStep = usUninstall then
  begin
    // İşlemleri durdur
    if ProcessExists('rogctl.exe') then
      Exec('taskkill.exe', '/IM rogctl.exe /F', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    
    if ProcessExists('rogctl-gui.exe') then
      Exec('taskkill.exe', '/IM rogctl-gui.exe /F', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    
    Sleep(1000);
    
    // Zamanlanmış görevi sil
    Exec('schtasks.exe', '/Delete /TN rogctl /F', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    
    // GPU kilidini serbest bırak
    Exec('nvidia-smi.exe', '-rgc', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  end;
end;
