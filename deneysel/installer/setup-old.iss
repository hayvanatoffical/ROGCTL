; ROGCtl - Professional Setup Wizard
; Inno Setup Script - Armoury Crate inspired professional installer
; Supports: Turkish and English

#define MyAppName "ROGCtl"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "ROGCtl Team"
#define MyAppURL "https://github.com/yourusername/rogctl"
#define MyAppExeName "rogctl.exe"

[Setup]
; Temel Ayarlar / Basic Settings
AppId={{B8F7D9A1-6C4E-4B2A-9F3D-8E5A7C2D1F4B}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE.txt
InfoBeforeFile=..\INSTALL_INFO.txt
OutputDir=..\release
OutputBaseFilename=ROGCtl-Setup-v{#MyAppVersion}
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
DisableWelcomePage=no

; Modern görünüm
WizardResizable=yes
WizardSizePercent=120

; Dil desteği / Language Support
ShowLanguageDialog=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "turkish"; MessagesFile: "compiler:Languages\Turkish.isl"

[CustomMessages]
; İngilizce mesajlar
english.WelcomeLabel2=Bu sihirbaz bilgisayarınıza [name/ver] kuracak.%n%nROGCtl, ASUS ROG laptoplar için gelişmiş termal ve güç yönetim yazılımıdır. Armoury Crate'in manuel modunu akıllı otomasyonla değiştirir.%n%nDevam etmeden önce tüm diğer uygulamaları kapatmanız önerilir.
english.RequirementsTitle=Sistem Gereksinimleri
english.RequirementsSubtitle=Lütfen sisteminizin minimum gereksinimleri karşıladığından emin olun
english.RequirementsLabel=Kurulum devam etmeden önce aşağıdaki gereksinimler kontrol edilecek:%n%n• ASUS ROG Laptop%n• NVIDIA Grafik Kartı%n• NVIDIA Sürücüsü (NVML desteği)%n• Yönetici Yetkisi%n%nNot: GPU saat kontrolü için NVML gereklidir. NVML olmadan sadece fan kontrolü çalışır.
english.PostInstallTitle=Kurulum Tamamlandı
english.LaunchProgram=ROGCtl'yi şimdi başlat (Yönetici olarak)
english.ViewStatus=Durum penceresini göster
english.OpenConfig=Yapılandırma dosyasını aç

; Türkçe mesajlar
turkish.WelcomeLabel2=Bu sihirbaz bilgisayarınıza [name/ver] kuracak.%n%nROGCtl, ASUS ROG dizüstü bilgisayarlar için gelişmiş termal ve güç yönetim yazılımıdır. Armoury Crate'in manuel modunu akıllı otomasyonla değiştirir.%n%nDevam etmeden önce tüm diğer uygulamaları kapatmanız önerilir.
turkish.RequirementsTitle=Sistem Gereksinimleri
turkish.RequirementsSubtitle=Lütfen sisteminizin minimum gereksinimleri karşıladığından emin olun
turkish.RequirementsLabel=Kurulum devam etmeden önce aşağıdaki gereksinimler kontrol edilecek:%n%n• ASUS ROG Dizüstü Bilgisayar%n• NVIDIA Ekran Kartı%n• NVIDIA Sürücüsü (NVML desteği)%n• Yönetici Yetkisi%n%nNot: GPU saat kontrolü için NVML gereklidir. NVML olmadan sadece fan kontrolü çalışır.
turkish.PostInstallTitle=Kurulum Tamamlandı
turkish.LaunchProgram=ROGCtl'yi şimdi başlat (Yönetici olarak)
turkish.ViewStatus=Durum penceresini göster
turkish.OpenConfig=Yapılandırma dosyasını aç

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "startupicon"; Description: "Başlangıçta otomatik başlat (Önerilen)"; GroupDescription: "Ek seçenekler:"; Flags: checkedonce
Name: "pathenv"; Description: "PATH ortam değişkenine ekle"; GroupDescription: "Ek seçenekler:"; Flags: checkedonce

[Files]
; Ana program dosyaları
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion; Check: FileExists(ExpandConstant('..\target\release\{#MyAppExeName}'))
Source: "..\target\release\rogctl.yaml"; DestDir: "{app}"; Flags: ignoreversion onlyifdoesntexist skipifsourcedoesntexist
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist

; Yardımcı scriptler
Source: "..\installer\scripts\check-requirements.ps1"; DestDir: "{tmp}"; Flags: dontcopy
Source: "..\installer\scripts\post-install.ps1"; DestDir: "{tmp}"; Flags: dontcopy
Source: "..\installer\scripts\uninstall-cleanup.ps1"; DestDir: "{tmp}"; Flags: dontcopy

[Icons]
Name: "{group}\{#MyAppName} Status"; Filename: "{app}\{#MyAppExeName}"; Parameters: "status"; IconIndex: 0
Name: "{group}\{#MyAppName} Rapor"; Filename: "{app}\{#MyAppExeName}"; Parameters: "rapor"; IconIndex: 0
Name: "{group}\Yapılandırma"; Filename: "notepad.exe"; Parameters: "{app}\rogctl.yaml"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Parameters: "status"; Tasks: desktopicon

[Registry]
; PATH ortam değişkenine ekle
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: pathenv; Check: NeedsAddPath(ExpandConstant('{app}'))

[Run]
; Sistem gereksinimlerini kontrol et
Filename: "powershell.exe"; Parameters: "-ExecutionPolicy Bypass -File ""{tmp}\check-requirements.ps1"""; StatusMsg: "Sistem gereksinimleri kontrol ediliyor..."; Flags: runhidden waituntilterminated; BeforeInstall: ExtractRequirementsScript

; Kurulum sonrası yapılandırma
Filename: "powershell.exe"; Parameters: "-ExecutionPolicy Bypass -File ""{tmp}\post-install.ps1"" -InstallPath ""{app}"""; StatusMsg: "ROGCtl yapılandırılıyor..."; Flags: runhidden waituntilterminated runascurrentuser; BeforeInstall: ExtractPostInstallScript

; Kullanıcı seçenekleri
Filename: "{app}\{#MyAppExeName}"; Parameters: "status"; Description: "{cm:LaunchProgram}"; Flags: nowait postinstall skipifsilent shellexec
Filename: "notepad.exe"; Parameters: "{app}\rogctl.yaml"; Description: "{cm:OpenConfig}"; Flags: nowait postinstall skipifsilent unchecked

[UninstallRun]
; Servisi durdur
Filename: "schtasks.exe"; Parameters: "/End /TN rogctl"; Flags: runhidden; RunOnceId: "StopRogctl"
Filename: "schtasks.exe"; Parameters: "/Delete /TN rogctl /F"; Flags: runhidden; RunOnceId: "DeleteRogctlTask"

; GPU kilidini serbest bırak
Filename: "{sys}\nvidia-smi.exe"; Parameters: "-rgc"; Flags: runhidden; RunOnceId: "ResetGPUClock"

; Temizlik scripti
Filename: "powershell.exe"; Parameters: "-ExecutionPolicy Bypass -File ""{tmp}\uninstall-cleanup.ps1"" -InstallPath ""{app}"""; Flags: runhidden waituntilterminated; BeforeInstall: ExtractUninstallScript

[Code]
var
  RequirementsPage: TOutputMsgWizardPage;
  RequirementsMet: Boolean;

procedure ExtractRequirementsScript();
begin
  ExtractTemporaryFile('check-requirements.ps1');
end;

procedure ExtractPostInstallScript();
begin
  ExtractTemporaryFile('post-install.ps1');
end;

procedure ExtractUninstallScript();
begin
  ExtractTemporaryFile('uninstall-cleanup.ps1');
end;

procedure CreateLicenseIfMissing();
var
  LicensePath: String;
  DefaultLicense: String;
begin
  LicensePath := ExpandConstant('{app}\LICENSE.txt');
  if not FileExists(LicensePath) then
  begin
    DefaultLicense := 'MIT License' + #13#10 + #13#10 +
                     'Copyright (c) 2024 ROGCtl Team' + #13#10 + #13#10 +
                     'Permission is hereby granted, free of charge, to any person obtaining a copy...' + #13#10;
    SaveStringToFile(LicensePath, DefaultLicense, False);
  end;
end;

function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath) then
  begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Param + ';', ';' + OrigPath + ';') = 0;
end;

procedure InitializeWizard;
begin
  RequirementsMet := True;
  
  // İkonları dinamik yükle
  if FileExists(ExpandConstant('..\installer\assets\rogctl.ico')) then
  begin
    WizardForm.Icon := LoadIconFromFile(ExpandConstant('..\installer\assets\rogctl.ico'));
  end;
  
  // Gereksinimler sayfası
  RequirementsPage := CreateOutputMsgPage(wpWelcome,
    ExpandConstant('{cm:RequirementsTitle}'),
    ExpandConstant('{cm:RequirementsSubtitle}'),
    ExpandConstant('{cm:RequirementsLabel}'));
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ResultCode: Integer;
begin
  Result := '';
  
  // NVIDIA sürücü kontrolü (opsiyonel - hata vermez)
  if Exec('nvidia-smi.exe', '--query-gpu=driver_version --format=csv,noheader',
         '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
  begin
    Log('NVIDIA driver detected');
  end
  else
  begin
    Log('NVIDIA driver not detected - GPU features may be limited');
    // Uyarı ver ama kurulumu engelleme
    MsgBox('NVIDIA sürücüsü bulunamadı. GPU saat kontrolü çalışmayacak, sadece fan kontrolü aktif olacak.' + #13#10 + #13#10 +
           'Devam etmek istiyor musunuz?', mbInformation, MB_YESNO);
  end;
end;

function InitializeSetup(): Boolean;
begin
  Result := True;
  
  // Yönetici yetkisi kontrolü (otomatik yükseltilmiş durumda)
  if not IsAdmin then
  begin
    MsgBox('Bu kurulum yönetici yetkisi gerektirir.' + #13#10 + 
           'Lütfen kurulumu yönetici olarak çalıştırın.', mbError, MB_OK);
    Result := False;
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  ResultCode: Integer;
begin
  if CurStep = ssPostInstall then
  begin
    // Zamanlanmış görev oluştur
    Exec('schtasks.exe', '/Create /TN "rogctl" /TR "' + ExpandConstant('{app}\{#MyAppExeName}') + ' daemon" /SC ONLOGON /RL HIGHEST /F',
         '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    
    // İsteğe bağlı olarak başlat
    if WizardIsTaskSelected('startupicon') then
    begin
      Exec('schtasks.exe', '/Run /TN "rogctl"', '', SW_HIDE, ewNoWait, ResultCode);
    end;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  ResultCode: Integer;
begin
  if CurUninstallStep = usUninstall then
  begin
    // Çalışan işlemi durdur
    Exec('taskkill.exe', '/IM rogctl.exe /F', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    
    // Zamanlanmış görevi sil
    Exec('schtasks.exe', '/Delete /TN rogctl /F', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
    
    // GPU saat kilidini serbest bırak
    Exec('nvidia-smi.exe', '-rgc', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  end;
end;

[UninstallDelete]
Type: files; Name: "{app}\rogctl.log"
Type: files; Name: "{app}\rogctl.log.1"
Type: files; Name: "{app}\status.txt"
Type: files; Name: "{app}\rapor.html"
Type: filesandordirs; Name: "{app}"
