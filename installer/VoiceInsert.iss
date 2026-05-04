#define AppName "VoiceInsert"
#define AppVersion "1.0.0"
#define Publisher "strokinkv"
#define PublishDir "..\artifacts\publish"

[Setup]
AppId=strokinkv.VoiceInsert
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName}
AppPublisher={#Publisher}
DefaultDirName={userappdata}\VoiceInsert
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
OutputDir=..\artifacts\installer
OutputBaseFilename=VoiceInsertSetup
Compression=lzma2
SolidCompression=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
UninstallDisplayIcon={app}\VoiceInsert.exe
SetupIconFile=..\src\VoiceInsert.App\Assets\VoiceInsert.ico
AppMutex=VoiceInsertAppMutex
CloseApplications=yes
RestartApplications=no
CloseApplicationsFilter=VoiceInsert.exe,VoiceInsert.dll,*.dll

[Files]
Source: "{#PublishDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs restartreplace

[InstallDelete]
Type: files; Name: "{app}\VoiceInsert.App.exe"
Type: files; Name: "{app}\VoiceInsert.App.dll"
Type: files; Name: "{app}\VoiceInsert.App.deps.json"
Type: files; Name: "{app}\VoiceInsert.App.runtimeconfig.json"
Type: files; Name: "{app}\is-*.tmp"

[Icons]
Name: "{group}\VoiceInsert"; Filename: "{app}\VoiceInsert.exe"

[Run]
Filename: "{app}\VoiceInsert.exe"; Description: "Launch VoiceInsert"; Flags: nowait postinstall skipifsilent unchecked

[Code]
procedure CurStepChanged(CurStep: TSetupStep);
var
  UninstallKey: string;
  SilentCommand: string;
begin
  if CurStep = ssPostInstall then
  begin
    UninstallKey := 'Software\Microsoft\Windows\CurrentVersion\Uninstall\strokinkv.VoiceInsert_is1';
    SilentCommand := '"' + ExpandConstant('{uninstallexe}') + '" /VERYSILENT /SUPPRESSMSGBOXES /NORESTART';
    RegWriteStringValue(HKCU, UninstallKey, 'UninstallString', SilentCommand);
    RegWriteStringValue(HKCU, UninstallKey, 'QuietUninstallString', SilentCommand);
  end;
end;
