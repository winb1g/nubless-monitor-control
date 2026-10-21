# tools

All scripts are **read-only** with respect to the monitor and to LG Switch's files.

| Tool | Purpose | Needs |
|---|---|---|
| `vcp-scan.ps1` | Reads every VCP code the monitor declares (capabilities string) and prints `code, current, max`, plus the raw capabilities string. | Windows, PowerShell 5.1 or 7 |
| `vcp-diff.ps1` | `-Save before.json`, change ONE setting in LG Switch / the OSD, then `-Compare before.json` to see which code changed. Retries flaky reads and reports "unstable" codes separately. | same |
| `read-firmware-level.ps1` | Prints the monitor's firmware (VCP `0xC9`), e.g. `3.05 (raw 0x0305)`. | same |
| `vcp-read.ps1` | Reads specific VCP codes (declared or not) with retries: `-Codes 0xF7,0xF8,0xF9`. | same |
| `fw-latest-per-model.ps1` | One GET per model of LG's `FWLatestVersion.txt`, sequential with a delay; writes a TSV of latest firmware + `Last-Modified`. Run rarely. | same + internet |
| `enum-dump/` | .NET tool: prints every enum in a managed assembly with its numeric values (`dotnet enum-dump.dll SwitchAppModel.dll`). LG's enum names are not obfuscated. | .NET SDK |
| `dump-metadata/` | .NET tool: lists types/methods of a managed assembly and any user strings. | .NET SDK |
| `il-walk/` | .NET tool: prints IL with resolved operands for chosen types/methods (`*` = all). | .NET SDK |
| `decrypt-strings.ps1` | Calls an assembly's own static string decryptor with integers you give it, in a separate process (needs Windows PowerShell 5.1 for .NET Framework assemblies). | Windows PowerShell 5.1 |

Notes

- `vcp-diff.ps1` snapshots are only written where you point `-Save`. They contain the monitor's capabilities string and current values, no serial numbers.
- `decrypt-strings.ps1` defaults to `SwitchAppModel.dll` and decryptor token `0x06000071`; for another assembly find the decryptor with `il-walk` (the method called right after `ldc.i4` before every string use) and pass `-asmName` / `-decTok`.
