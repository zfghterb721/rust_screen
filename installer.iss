#define MyAppName "Display RTSP Streamer"
#define MyAppVersion "0.1.0"
#define MyAppPublisher "Your Organization"
#define MyAppURL "https://github.com/yourusername/display-rtsp-streamer"
#define MyAppExeName "display_rtsp_streamer.exe"

[Setup]
AppId={{A0C2F65A-8B2D-4E49-9F73-B91F1B2C92D1}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile=LICENSE
OutputDir=installer
OutputBaseFilename=display-rtsp-streamer-setup
Compression=lzma
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "sample_config.toml"; DestDir: "{app}"; Flags: ignoreversion
Source: "gstreamer-requirements.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Parameters: "run"
Name: "{group}\Configure {#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Parameters: "edit-config"
Name: "{group}\Readme"; Filename: "{app}\README.md"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"

[Run]
Filename: "{app}\{#MyAppExeName}"; Parameters: "install"; Description: "Install and start the service"; Flags: runascurrentuser postinstall nowait

[UninstallRun]
Filename: "{app}\{#MyAppExeName}"; Parameters: "uninstall"; RunOnceId: "StopService"; Flags: runascurrentuser

[Messages]
WelcomeLabel2=This will install [name/ver] on your computer.%n%nThis application captures your desktop screens and makes them available as RTSP streams for integration with security camera systems like BlueIris.%n%nIMPORTANT: This application requires GStreamer Runtime 1.20 or newer (MinGW 64-bit) to be installed. If you haven't installed it yet, please cancel this installer and install GStreamer first.

[Code]
function InitializeSetup(): Boolean;
var
  ResultCode: Integer;
  GStreamerPath: String;
begin
  // Check if GStreamer is installed
  GStreamerPath := ExpandConstant('{commonpf64}\GStreamer\1.0\bin');
  if not DirExists(GStreamerPath) then
  begin
    if MsgBox('GStreamer Runtime does not appear to be installed, which is required for this application to work. Do you want to continue anyway?', mbConfirmation, MB_YESNO) = IDNO then
    begin
      MsgBox('Please download and install GStreamer Runtime 1.20 or newer (MinGW 64-bit) from https://gstreamer.freedesktop.org/download/', mbInformation, MB_OK);
      Result := False;
      Exit;
    end;
  end;
  
  Result := True;
end;