; Inno Setup Script for YouTube Client Suite
; Compiles a professional single-file Windows installer: youtube-client-setup-<version>.exe

#ifndef MyAppVersion
#define MyAppVersion "0.1.3"
#endif

#define MyAppName "YouTube Client"
#define MyAppPublisher "Rob Tizzard"
#define MyAppURL "https://github.com/clockworkengineer/youtube-client"
#define MyAppExeName "youtube-gui.exe"
#define MyCliExeName "youtube-client.exe"

[Setup]
AppId={{9FA72A3C-1F54-4C78-8DF4-39908F87C9E1}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={localappdata}\Programs\YouTubeClient
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
LicenseFile=..\..\LICENSE
OutputDir=..\..\dist
OutputBaseFilename=youtube-client-setup-{#MyAppVersion}-x86_64
SetupIconFile=..\..\assets\icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "addtopath"; Description: "Add installation directory to user PATH for command-line access"; GroupDescription: "System Integration:"

[Files]
Source: "..\..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\target\release\{#MyCliExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\target\release\youtube-installer.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\assets\icon.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\assets\icon.png"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\icon.ico"
Name: "{group}\YouTube Client CLI"; Filename: "{app}\{#MyCliExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\icon.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Registry]
; Register PATH addition if selected
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: addtopath; Check: NeedsAddPath(ExpandConstant('{app}'))

[Code]
function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath)
  then begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + UpperCase(Param) + ';', ';' + UpperCase(OrigPath) + ';') = 0;
  if Result then
    Result := Pos(';' + UpperCase(Param), ';' + UpperCase(OrigPath)) = 0;
end;
