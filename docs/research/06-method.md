# 06 · Method (reproducible) and boundaries

## What was done

All steps were performed on the author's own PC, on an installed copy of LG Switch 7.60, **without launching LG Switch, without changing any file in its install folder, and without writing to the monitor**.

1. **Inventory**: file sizes, versions, which binaries are managed (.NET) vs native.
2. **Plain strings**: scanned binaries (ASCII + UTF-16) for URLs and keywords. LG's managed assemblies had no readable URLs.
3. **Metadata dump** (`tools/dump-metadata`): uses `System.Reflection.Metadata` to list types/methods. Result: names are scrambled (`lll11iiilll1`…) and the **user-string heap is empty**, so strings are encrypted at runtime.
4. **IL walker** (`tools/il-walk`): prints each method's IL with call/field/type operands resolved. In `FirmwareUpdateHandler.getDownloadUpdateCheckURL` every string is `ldc.i4 <number>; call <decryptor>` (decryptor = method token `06000071` in `SwitchAppModel.dll`).
5. **String decryption** (`tools/decrypt-strings.ps1`): loads `SwitchAppModel.dll` in a **separate Windows PowerShell 5.1 process** (same CLR as the app; the assembly targets .NET Framework 4.7.2) and calls only the static string-decryptor method with the integers found in step 4. No application code is run beyond what the DLL does when loaded. Result: 2,793 strings decoded, 0 errors.
6. **Local state**: read-only look at `%APPDATA%\LGSwitch`, `C:\Users\Public\Documents\LG Switch*`, `HKCU\Software\LG Soft India`.
7. **Live endpoint checks**: plain HTTPS GETs and HEADs against `lmu.lge.com`, at the same low rate as a normal client.
8. **DDC/CI**: standard Windows `dxva2.dll` calls (`tools/vcp-scan.ps1`), reads only.

### Re-running it

```powershell
# 1) build the two helpers (needs the .NET SDK)
dotnet build tools\dump-metadata -c Release -o out\dump
dotnet build tools\il-walk       -c Release -o out\il
$bin = 'C:\Program Files (x86)\LG Electronics\LG Switch\bin'

# 2) IL of the firmware handler (or '*' '*' for the whole assembly)
dotnet out\il\il-walk.dll "$bin\SwitchAppModel.dll" "FirmwareUpdateManager.FirmwareUpdateHandler" "getDownloadUpdateCheckURL"

# 3) decrypt a list of tokens (one integer per line) with the app's own decryptor
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools\decrypt-strings.ps1 `
  -bin $bin -tokenFile tokens.txt -outFile strings.tsv
```

Decoded output stays local (see `.gitignore`).

## What was deliberately **not** done

- No traffic interception / TLS interception, no debugger attached to LG processes, no patching.
- No download or analysis of the firmware zip, the encrypted AI model (`Vit_B16_*.enc`, `decrypt.json`), or any DRM/licensing logic.
- No writes to VCP codes. The first write test will be a **brightness change by one step and back** (a standard code), with explicit confirmation.
- No attempt to flash firmware.

## Tools you might want later (portable, no install)

Only needed if we go beyond this: **ILSpy / `ilspycmd`** (readable C# of the managed assemblies once strings are decoded; ilspycmd is a portable `dotnet tool install --tool-path`), **de4dot-cex** (if the obfuscator is identified and string decryption automated), **Ghidra** (for the native `LGMonitorDDCCISDK.dll` / `SmartMonitorControlService.exe` if the private opcode protocol is not recoverable by black-box diffing), **API Monitor** or **Frida** (to observe the exact `dxva2`/`SetVCPFeature` calls LG makes, by watching `SmartMonitorControlService.exe`). The black-box VCP diff in `05-screen-settings-panel.md` is cheaper and should be tried first.
