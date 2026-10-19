# 02 · Local files and registry

All paths observed on a machine with LG Switch 7.60 and a 27GM950B. User-specific parts are shown as placeholders.

## Files

| Path | Format | Content |
|---|---|---|
| `C:\Users\Public\Documents\LG Switch_FW Update\<MODEL>_<id>_<id>.txt` | text with a small XML body | **Firmware update history.** Contains `updated date ;<local time>`, then `<version> 3.05, 4.23.16, 1.11</version>`, `<release_date>2026.05.27 (YYYY.MM.DD)</release_date>`, `<update_desc>…</update_desc>`, `<note>…</note>`. This is what the *Monitor Software Update History* screen shows. The same text is probably inside LG's firmware zip. |
| `C:\Users\Public\Documents\LG Switch\LG Switch.txt` | `KEY;VALUE` lines | App settings: `AUTOSTART`, `AUTOUPDATE`, `FW_UPDATE_DO_NOT_SHOW`, `LASTSELECTEDMODE`, `VIDEOCALLAPPLICATION`, `VIDEOCALLHOTKEY` (e.g. `Alt + Shift + U`), `WALLPAPERTIMER`, `LIFEMODETARGETEDMONITOR` (`<EDID name>:<product code>:<id>`), `LASTAIMODELMUVALUE`… |
| `C:\Users\Public\Documents\LG Switch\Switch_UserSelectedApplications.txt`, `LGSwitchWallpaperConfig.txt` | text | Screen Splitter app list, wallpaper config |
| `%APPDATA%\LGSwitch\LMUServerConfigFile.txt`, `RemainingConfigFile.txt`, `wallpaper_list_new.txt` | `url;url` lines | Cached wallpaper pack list |
| `%APPDATA%\LGSwitch\Smart_ConnNetwork_Info.xml`, `LocalConfigFile.txt` | XML / text | Network-connection info for the companion feature (empty here) |
| `%APPDATA%\LGSwitchAppUpdate\LGSwitchAppUpdate.txt` | text | App-update state |
| `<install>\bin\decrypt.json`, `Vit_B16_20250808.enc` | **encrypted** | AI model material. Not analysed. |

The history file name `27GM950B_23800_751222.txt` also tells you which model name LG Switch resolved for the monitor (`27GM950B`), even though Windows only reports `LG ULTRAGEAR+` (EDID product code `5CF8`).

## Registry (HKCU, vendor key is `LG Soft India`)

| Key | Values |
|---|---|
| `HKCU\Software\LG Soft India\LG Switch\ScreenSplitter` | `SwitchMode` (`WORK`), `DualModeShortcutKey` (`Alt + Shift + M`), `KVMShortcutKey` (`Alt+Shift+B`), `QuicksplitShortcutKey` (`Alt+Shift+L`), `SSHotkey`, `DualModeChanged` |
| `…\ScreenSplitter\<EDID name>_<n>_<n>` | per-monitor `ResizeChanged`, `OrientationChanged`, `OrientationMode` |
| `…\LG Switch\NetworkSettings` | `NWSHotkey` |

So hotkeys are *configured* in the registry/`LG Switch.txt` and *implemented* by a keyboard hook process (`SwitchScreenSplitterHook64App.exe`). A replacement can register its own global hotkeys with `RegisterHotKey`; no hook needed.

## Named pipes

`\\.\pipe\SwitchSendPipe`, `\\.\pipe\SwitchReceivePipe` (UI ⇄ control service).
