# 03 · Firmware update flow

## Naming

`MODEL_<model>_<tag><ver>_<tag><ver>_..._<YYYYMMDD>.zip`, for example:

```
MODEL_27GM950B_US3.06_PV4.23.16_NV1.11_20260624.zip
```

Each `<tag><ver>` is the version of one firmware component inside the monitor. Tags seen in the code: `US`, `PV`, `NV`, `SV`, `RV`, `TV`, `VB`, `LV`, `DV`, `MV`. Which ones exist depends on the model (the 27GN950 uses `SV`+`NV`; a 32GQ950 uses `SV`+`DV`+`NV`; a 27UP850-L2L uses `SV`+`PV`+`NV`). The trailing date is the release date.

LG Switch's history screen shows the same numbers comma-separated: `3.05, 4.23.16, 1.11` = `US, PV, NV` for the 27GM950B.

**The `US` number equals the monitor's own DDC/CI "display firmware level"** (VCP `0xC9`, raw `0x0305` = 3.05). Verified on one 27GM950B (3.05 reported by the monitor and by LG Switch's history file).

## Check

1. Determine the model name (LG Switch resolves it from the monitor; for a 27GM950B it is `27GM950B`).
2. Build the URL (`FirmwareUpdateHandler.getDownloadUpdateCheckURL`):
   `https://lmu.lge.com/ExternalService/onscreencontrol/fw/` + model + *family suffix* + `/FWLatestVersion.txt`
3. GET it. The body is the zip file name. Compare the version parts with what the monitor reports (the app reads them through `FirmwareUpdateDLL.dll!GetFirmwareVersion`, i.e. over the same I2C/HID path used for flashing, then compares numerically per component: `FIRMWARE_VERSION_GREATER/EQUAL/LESSER/ERROR`).
4. Download URL: same directory + the zip name (`getFWUpdateFileDownloadFileURL`). Saved as `fw.zip`, extracted into a folder named `fw` (constants `FIRMWARE_FILE_DOWNLOAD_TEMP_NAME`, `FIRMWARE_FILE_UNZIP_FOLDER_NAME`).
5. Flash via `LGFirmwareUpdater.exe` / `FirmwareUpdateDLL.dll` (USB HID to I2C; scaler vendor families implemented as `CRealtekScalarFirmware`, `CMStarScalarFirmware`, `CNXPFirmware`, `CTIPowerDeliveryFirmware`, `CRealtekPowerDeliveryFirmware`, `CHubFirmware`, `CThunderbolt(5)Firmware`, …). Progress and results are broadcast as `WM_FW_UPDATE_*` window messages.
6. On success the app writes the history file (see `02-local-files-and-registry.md`).

For some models the check is also gated by a native probe (`checkTBT4FWUpdateAvailability`) that chooses `_Win` vs `_Mac` style paths.

## Per-model path table (family suffix)

Extracted from the linear IL of `getDownloadUpdateCheckURL`. Models are grouped by consecutive comparisons that share one suffix. **Inferred grouping, spot-checked live:** `27GN950_DSC`, `32GQ950_DSC` and `27UP850-L2L_PD` returned real firmware names; `49GR85DC_NEW_Res` returned `NotAllowedPage` (the grouping or the model-name key is probably different for that family). Models **not** in this table (including **27GM950B**) use *no suffix*: `fw/<MODEL>/FWLatestVersion.txt`.

| Model | Suffix | URL (after `…/onscreencontrol/`) |
|---|---|---|
| 27UL850-RTK | "_2" | .../fw/27UL850-RTK_2/FWLatestVersion.txt |
| 32UK950 | "-LED" | .../fw/32UK950-LED/FWLatestVersion.txt |
| 34WK95U | "-LED" | .../fw/34WK95U-LED/FWLatestVersion.txt |
| 27GN950 | "_DSC" | .../fw/27GN950_DSC/FWLatestVersion.txt |
| 27GP950 | "_DSC" | .../fw/27GP950_DSC/FWLatestVersion.txt |
| 32GQ950 | "_DSC" | .../fw/32GQ950_DSC/FWLatestVersion.txt |
| 32GQ850 | "_DSC2" | .../fw/32GQ850_DSC2/FWLatestVersion.txt |
| 32GQ850L | "_DSC2" | .../fw/32GQ850L_DSC2/FWLatestVersion.txt |
| 40WP95C | "_Win" | .../fw/40WP95C_Win/FWLatestVersion.txt |
| 40WT95UF | "_Win" | .../fw/40WT95UF_Win/FWLatestVersion.txt |
| 40U990A | "_Win" | .../fw/40U990A_Win/FWLatestVersion.txt |
| 40U990C | "_Win" | .../fw/40U990C_Win/FWLatestVersion.txt |
| 32U990A | "_Win_2" | .../fw/32U990A_Win_2/FWLatestVersion.txt |
| 32BU950A | "_Win" | .../fw/32BU950A_Win/FWLatestVersion.txt |
| 32U990B | "_Win" | .../fw/32U990B_Win/FWLatestVersion.txt |
| 24QP750 | "_2" | .../fw/24QP750_2/FWLatestVersion.txt |
| 32GP850 | "_2" | .../fw/32GP850_2/FWLatestVersion.txt |
| 27UP850NL_PD | (none) | .../fw/27UP850NL_PD/FWLatestVersion.txt |
| 27UP80B_PD | (none) | .../fw/27UP80B_PD/FWLatestVersion.txt |
| 27BP85UN_PD | (none) | .../fw/27BP85UN_PD/FWLatestVersion.txt |
| 27UP850-L2L_PD | (none) | .../fw/27UP850-L2L_PD/FWLatestVersion.txt |
| 27EP950 | "_NEW" | .../fw/27EP950_NEW/FWLatestVersion.txt |
| 27BP95E | "_NEW" | .../fw/27BP95E_NEW/FWLatestVersion.txt |
| 27EQ850 | "_NEW" | .../fw/27EQ850_NEW/FWLatestVersion.txt |
| 32EP950 | "_NEW" | .../fw/32EP950_NEW/FWLatestVersion.txt |
| 32BP95E | "_NEW" | .../fw/32BP95E_NEW/FWLatestVersion.txt |
| 34WQ75C | "_NEW" | .../fw/34WQ75C_NEW/FWLatestVersion.txt |
| 34WQ73A | "_NEW" | .../fw/34WQ73A_NEW/FWLatestVersion.txt |
| 38BQ85C | "_NEW" | .../fw/38BQ85C_NEW/FWLatestVersion.txt |
| 34BQ77QC | "_NEW" | .../fw/34BQ77QC_NEW/FWLatestVersion.txt |
| 34BR55QC | "_NEW" | .../fw/34BR55QC_NEW/FWLatestVersion.txt |
| 38WP85C | "_NEW" | .../fw/38WP85C_NEW/FWLatestVersion.txt |
| 24BP750C | "_NEW" | .../fw/24BP750C_NEW/FWLatestVersion.txt |
| 32GQ950P | "_NEW" | .../fw/32GQ950P_NEW/FWLatestVersion.txt |
| 35WN75C-L2 | "_NEW" | .../fw/35WN75C-L2_NEW/FWLatestVersion.txt |
| 32UQ750 | "_PD" | .../fw/32UQ750_PD/FWLatestVersion.txt |
| 49WQ95C | "_PD" | .../fw/49WQ95C_PD/FWLatestVersion.txt |
| 27BQ75QC | "_PD" | .../fw/27BQ75QC_PD/FWLatestVersion.txt |
| 27GR95QE | "_NEW_Res" | .../fw/27GR95QE_NEW_Res/FWLatestVersion.txt |
| 45GR95QE | "_NEW_Res" | .../fw/45GR95QE_NEW_Res/FWLatestVersion.txt |
| 49GR85DC | "_NEW_Res" | .../fw/49GR85DC_NEW_Res/FWLatestVersion.txt |
Models that LG Switch treats as firmware-update-capable in `getFWUpdateAvailablity` include `27GM950B`, `39GX950B`, `45GX950B` and `25G590B` (these use the default no-suffix path).

## What we have NOT verified

- The contents of the firmware zip (release notes, file layout). Downloading it is a separate, explicit step.
- Cabling requirements and failure behavior of the flasher on the 27GM950B.
- Whether any older firmware remains downloadable. The 3.05 file name was not served.

## Per-model results (2026-10-18)

`tools/fw-latest-per-model.ps1` asked LG's endpoint for each of the **200 model names LG Switch knows** (one GET each, 400 ms apart). Raw results: [`data/fw-per-model-2026-10-18.csv`](../data/fw-per-model-2026-10-18.csv).

- **78 models** have a firmware package published at the default or family-suffix path; **122** return `NotAllowedPage`.
- **45GX950B** has one (`US3.05_PV4.23.16_NV1.11_20260527`, the same component versions as the 27GM950B's 3.05). **39GX950B** and **52G930B** do not (the app lists 39GX950B as firmware-capable, so its package is probably unpublished or on a different path).
- File dates of the latest package per model: 2017 ×2, 2018 ×2, 2019 ×2, 2020 ×4, 2021 ×18, 2022 ×17, 2023 ×18, 2024 ×3, 2025 ×6, 2026 ×6. The oldest still hosted: 27UK850 (2017-11-20), 38WK95C (2017-12-28), 43UD79 (2018-08-13), 32GK850F (2018-11-27).
- Some models share a package with a sibling SKU (`27BP85UN_PD` and `27UP80B_PD` serve the `27UP850NL` file).
- **No version history exists on the server**: one package per model. We did not brute-force paths: the app's own logic makes exactly one request per model.
