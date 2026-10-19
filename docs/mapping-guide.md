# Mapping guide: how a control becomes writable

A control's `confidence` in the profile decides whether nubless will write it. Raise it only with evidence.

| Confidence | What it takes | Writable? |
|---|---|---|
| `unknown` | nothing yet | never |
| `candidate` | a plausible code from LG's opcode names or the value shape | never |
| `strong` | LG's opcode/enum name **and** a live read that matches LG's UI | with `--experimental` |
| `read-confirmed` | the read matches the UI across several values | standard MCCS codes: yes. LG codes: `--experimental` |
| `write-confirmed` | every value written and read back, and seen in the OSD | yes |

## Read-only diff (Windows)

```powershell
.\docs\research\tools\vcp-diff.ps1 -Save before.json
# change exactly ONE setting with the OSD or LG Switch
.\docs\research\tools\vcp-diff.ps1 -Compare before.json
```

On Linux: `ddcutil getvcp ALL` before and after.

## Test card (paste into the PR)

```
Control:            Response Time
Monitor firmware:   3.05 (nubless get firmware)
Before (OSD):       Fast        raw: 2
After  (OSD):       Faster      raw: 1
Values:             Normal=_  Fast=_  Faster=_
Write round-trip:   nubless --experimental set response-time faster  -> OSD shows Faster: yes/no
Undo:               nubless undo -> OSD shows Fast: yes/no
```
