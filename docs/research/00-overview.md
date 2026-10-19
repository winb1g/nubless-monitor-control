# 00 · Architecture of LG Switch 7.60

Observed on Windows 11 with a 27GM950B. Install dir: `C:\Program Files (x86)\LG Electronics\LG Switch\bin\` (very large: CUDA/ONNX runtimes, an encrypted AI model, wallpaper videos and help images dominate; LG lists the installer at about 940 MB).

## Processes and services

| Component | Kind | Role (evidence) |
|---|---|---|
| `LG Switch.exe` | managed .NET Framework 4.7.2, WPF, **obfuscated** (identifier renaming, string encryption) | The UI. Screen Settings, Screen Splitter, Update History, wallpapers. |
| `SwitchAppModel.dll` | managed, same obfuscation | App logic: firmware manager, IPC client, monitor manager, constants. Most useful assembly for RE. |
| `LGSwitchApplicationManager.exe` | native C++ (MFC) | Helper that reads EDID/model name/serial, handles FW SDK handles. |
| **`SmartMonitorControlService.exe`** (service *LG Smart Monitor Control Service*) | native | Talks to the monitor. Receives requests from the UI over named pipes. |
| `SmartMonitorControlStack.dll` | native | Service-side stack. |
| **`LGMonitorDDCCISDK.dll`** | native C++ | LG's DDC/CI "ProtocolEngine". Class names in its log strings: `CCalibration` (Brightness, Contrast, Gamma, Sharpness, ColorTemperature, H/V position), `CColorSetting` (ColorPreset, VideoGain, VideoBlackLevel), `CFEngine` (ACE, RCM, Mode, Brightness), `CFineWindow`, `CCinemaMode`, `CFactorySetting::Restore`, `CAsset`/`CEDID` (model, serial, manufacture date, screen size). |
| `LGSwitchControlService.exe` (service *LG Switch Control Service*) | managed | Companion service (startup/hooks/IPC; not fully analysed). |
| `LG Switch TBT5 Firmware Service` + `LGThunderboltFirmwareUpdater.exe` | managed | Thunderbolt 5 hub firmware (not relevant to a 27GM950B). |
| `FirmwareUpdateDLL.dll`, `LGFirmwareUpdater.exe`, `RHubLib.dll` | native | Firmware flashing. USB HID to I2C bridge (`HidD_*`, `I2C_*`), Realtek hub vendor commands, NXP scaler paths. |
| `SwitchCLI.exe` | managed | A command-line client that talks to LG Switch through a named pipe (`CLIPipeData`, run modes 1-4). |
| `SwitchScreenSplitterHook64App.exe` | native | Global keyboard hook for Screen Splitter / Dual-Mode hotkeys. |
| `CaptureAndPredictScene.exe` + `Vit_B16_*.enc` + ONNX Runtime | .NET + ML | "AI Scene Optimization" / "AI Picture Mode". Model is encrypted; **out of scope**. |

## Data flow for a setting change (hypothesis, consistent with strings)

```
Screen Settings (WPF, LG Switch.exe)
   → SwitchAppModel.IPCHandler  ── named pipes ──►  \\.\pipe\SwitchSendPipe / \\.\pipe\SwitchReceivePipe
                                                    SmartMonitorControlService.exe
                                                    → LGMonitorDDCCISDK.dll (ProtocolEngine)
                                                    → DDC/CI over the GPU driver  → monitor scaler
```

Pipe message types seen in the model: `eSetSmartMonitorInfo`, `eSmartMonitorInfoReceived`, `eSmartMonitorDisplayPresetInfo`; request/response structs `T_S_MONITOR_CONTROL`, `T_S_SMART_MONITOR_INFO`.

A WebSocket path also exists (messages `WM_WEBSOCKET_*`: pairing PIN, handshake, current input, HDR status, picture-mode list, volume, auto-pivot). This looks like a phone/companion app protocol; not analysed.

## Where the UI code really lives

| Assembly | What is in it |
|---|---|
| `ModeSwitchModule.dll` (8.5 MB, managed, decoded) | **All WPF views and view-models**: `Views/ScreenSettings.xaml`, `MonitorSoftwareUpdate`, `MonitorSoftwareUpdateHistory`, `SoftwareUpdate`, `SwitchSettings`, `NetworkSettings`, `ScreenSplit`, `PartitionMode`, `VideoCall`, `MonitorIndicatorWindow`, wallpaper windows. Property names are readable, including the per-model capability flags (see `05`). Product-registration URLs are here. |
| `SwitchAppModel.dll` (managed, decoded) | Firmware manager (`FirmwareUpdateManager.*`), IPC client (`IPCHandlerImpl.IPCHandler`), monitor manager, constants, and LG's **opcode/enum tables** (`T_E_OPCODE` with 211 members, picture modes, response-time/Adaptive-Sync values…) |
| `LGCommonModule.dll` (managed, decoded) | Shared named-pipe layer (`NamedPipeLine`, `ServerPipeLine`, `PipeData`) used by the UI, the CLI and the control service; logging (`LGControlServiceLog_…`, `{name}_{yyyy_MM_dd_HH_mm_ss}.txt`); the **CLI error vocabulary** (below). |
| `SwitchCLI.exe` (managed) | Pipe client with run modes `SWITCH_APP`, `SWITCH_CLI`, `SWITCH_SERVICE` and `CLIPipeData` (command, display id, firmware path, received value). Strings not decoded. |

**CLI/firmware error codes** (names visible in `LGCommonModule`): `CommandNotAvailable`, `ControlServiceNotAvailable`, `FWFileInvalid`, `FWUpdateNotAvailable`, `InvalidCommandFormat`, `InvalidOption`, `MultipleLGMonitorsConnected`, `NoSupportedMonitor`, **`NoUSBCableConnected`**, `OSCorSwitchAppNotAvailable`, `OSCorSwitchAppREInitInProgress`, `OSCorSwitchAppTerminated`. So the firmware flow requires the monitor's **USB cable to be connected** and exactly one supported LG monitor, and there is a CLI path that takes a firmware file (`strFWPath`).

## Why a replacement can be much smaller

Windows already exposes standard **DDC/CI** through `dxva2.dll` (`GetVCPFeatureAndVCPFeatureReply`, `SetVCPFeature`, `GetCapabilitiesString…`). A read of the 27GM950B through that API worked without any LG software (see `04-ddc-vcp-27gm950b.md`), and brightness (VCP `0x10`) and contrast (`0x12`) match the values the LG panel shows exactly. LG's private features (picture modes, response time, black stabilizer…) very likely ride on manufacturer-specific VCP codes (`0xE0–0xFF`) that the monitor itself lists in its capabilities string. Mapping them is what `tools/vcp-diff.ps1` is for.

## Not analysed yet

- `LGSwitchControlService.exe` internals, the WebSocket companion protocol.
- The exact encoding of Dual-Mode "Conversion". The monitor-side opcode is `0xB1` with sub-commands (see `05`); the wire format is not confirmed.
- Strings in `LG Switch.exe`, `LGSoftwareUpdaterServiceApp.exe`, `SwitchCLI.exe`, `LGSwitchControlService.exe`, `LGThunderboltFirmwareUpdater.exe`: a stricter decryptor variant returns a placeholder outside its own process. These are mostly UI text; the networking logic is in the decoded assemblies.
- Whether the manufacturer opcodes go through plain VCP get/set (reads of `0xF7`/`0xF8`/`0xF9`/`0x15` worked this way) or an LG-specific DDC/CI command class for writes.
