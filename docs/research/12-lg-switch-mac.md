# 12 · LG Switch for Mac (7.49): how it reaches the monitor

The macOS build of LG Switch, read without installing or running it. Findings are in our own words. Symbol names and constants are given where they help someone check the result; no LG text, strings or code is reproduced. The Windows build (7.60) is covered in notes 00 to 09.

## What was looked at, and how

- `Switch_V7.49_signed.pkg`, signed by "Developer ID Installer: LG Electronics (5SKT5H4CPQ)" and notarized by Apple. Expanded with `pkgutil --expand-full` into an empty folder. The installer, its scripts and its apps were never run.
- Universal (x86_64 and arm64) binaries: the main app, a second app called *LG Switch Multi Monitor*, a login-item tool `SwitchCLI`, a small HDR helper shipped with a `.txt` extension, and frameworks.
- Methods: file listings, `strings`, Objective-C metadata (`otool -ov`) for class and method names, the imported-symbol list (`nm -u`), and the arm64 disassembly (`objdump -d`) of a handful of short functions, only to read the constants they pass. 2026-11-19.

## The two apps

| App | Job |
|---|---|
| **LG Switch** | UI, settings, firmware updates, webOS monitors. It doesn't import the video-cable DDC/CI functions; its `DDCCIEngine` / `DDCCIEngineHelper` pick the code and value, and it launches and checks for a separate monitor process (`launchMonitorProcess`, `checkIfMultiMonitorProcessExists`). How the two apps exchange requests wasn't traced. |
| **LG Switch Multi Monitor** | Talks to the monitors (DDC/CI over the video cable, USB HID), and does the window tiling (Screen Split). It also ships six C headers from LG's 2006 Windows I2C driver and protocol engine; they're leftovers, but they name the USB serial route below. |

## DDC/CI over the video cable

The Multi Monitor app uses the same Apple calls as nubless (`IOAVServiceCreateWithService`, `IOAVServiceWriteI2C`, `IOAVServiceReadI2C`). There are only two write sites, and both use chip `0x37` with the **standard sender `0x51`**:

| Request | What LG's code does |
|---|---|
| **Set VCP Feature** (6 bytes) | Writes it **three times**, sleeping 10 ms before each, and doesn't stop early if a write succeeds. The checksum starts from `0xB8` (`0x6E ^ 0x51 ^ 0x84 ^ 0x03`), so it's an ordinary Set VCP Feature. |
| **Get VCP Feature** (4 bytes) | Writes the request **six times**, 10 ms apart, then waits **1 second** before reading the 11-byte reply. |

That's LG doing what note 11 found: on Apple Silicon, LG panels need each request repeated. nubless sends twice (`crates/nubless-hw/src/twice.rs`) and reads much sooner; it works on the 27GM950B, so there's no reason to copy LG's 1 second wait.

No `0x50` sender appears on this path. No DDC/CI input switch was found in LG Switch for Mac (it has a KVM setting, and input switching for webOS monitors over the network), so this doesn't contradict note 11, which found input switching only works on `0x50`.

## Which codes the app sends

Read from the `DDCCIEngine` methods that build each request:

| Feature | Code | Name in LG's Windows opcode table (note 08) | On the 27GM950B (fw 3.05) |
|---|---|---|---|
| Sharpness | `0x87` | `IMAGE_SHARPNESS` | Answers 50 of 100; not declared in its capabilities |
| KVM / USB switch | `0x55` | `DPVL_KEYBOARD_MOUSE_SWITCH` | Answers 0 of 1; not declared |
| PBP on | `0xD7` | `MISC_AUXILLARY_POWER_OUTPUT` (sic; the standard MCCS meaning) | Answers 1; not declared |

So LG uses `0xD7` for PBP/PIP, as a normal `0x51` write. That matches the summary on the ddcutil LG input-switching wiki ("x51: PBP controls"), and not the later note on the same page that puts PBP on `0x50` (see note 11). The values are uncertain: LG's Windows enum `T_E_PBP_MODE_OPTION` (note 08) and the ddcutil wiki number the layouts differently. The 27GM950B's menu wasn't checked for PBP/PIP; until it is, `0xD7` might be the standard auxiliary-power control on this model, so nothing should write it.

## Other ways it reaches a monitor

None of these was tried on the 27GM950B.

- **Realtek USB hub bridge** (`iUSB2I2C.framework`). Reaches the monitor's internal I2C bus through its USB hub (Realtek RTS5411), using Realtek's own commands over HID. Exports raw `ReadI2C` / `WriteI2C`, bus speed, and firmware updates for the hub and the scaler.
- **USB serial port.** The protocol-engine headers describe a monitor that shows up as a USB serial device (TI TUSB3410, `0451:3410`, or an NXP chip shown as `usbmodem`) at 19200 baud, with an Apple Silicon entry point (`InitializeForSiliconByComport`) and a separate packet size for uploading calibration tables.
- **USB HID.** Both apps use Apple's HID calls. The main app has separate brightness and contrast handling for monitors with an NVIDIA G-Sync module, which is likely what HID is for; that wasn't traced.

Either USB route would get around the Mac HDMI limit (note 11), but only on a monitor whose USB upstream port is connected and that exposes one of these devices. To check a 27GM950B: connect its USB upstream to the Mac and look for a Realtek HID device or a `usbmodem` serial port. That check is read-only.

## Firmware updates

The download flow is in note 03. On the Mac, flashing goes over **USB only**, with one handler per chip in the monitor: MStar scaler, Realtek and other power-delivery controllers, Thunderbolt, NXP, a DFU path, and a vision board. The app tells users that HDMI isn't supported for firmware updates on Apple Silicon Macs, which is the same limit note 11 found. Flashing stays out of scope for nubless. Reading the firmware level (`0xC9`) and comparing it with LG's latest-version file (note 03) would be safe.

## Screen Split

Window tiling only; it never talks to the monitor. The Multi Monitor app uses the macOS Accessibility API (`AXUIElement…`) to move and resize other apps' windows, with an overlay window for each layout.

## webOS monitors (networked)

LG's webOS monitors run the same system as LG TVs and connect over Wi-Fi or Ethernet. LG Switch treats them like TVs:

1. **Find** them with a discovery engine (its method isn't visible in the binary), or by typing an IP address.
2. **Pair** over a WebSocket: a `register` request with a PIN pairing type, the PIN the monitor shows, and a client key the app stores encrypted. Up to four monitors.
3. **Control** with webOS's SSAP requests: switching inputs (`ssap://tv/switchInput`, input ids such as `com.webos.app.hdmi1`), system info, and a pointer socket to drive the monitor's cursor and keyboard.

The 27GM950B has no network hardware, so none of this applies to it. Supporting webOS monitors would mean implementing SSAP, as open-source LG TV libraries do, without reusing the signed app manifest LG Switch sends, which is LG's credential.
