# 01 · Server endpoints

Host: **`lmu.lge.com`** (Akamai NetStorage). Plain GETs, no authentication observed. HTTP redirects to HTTPS. **No CORS headers**, so browsers can't call them directly.
Recovered by decrypting the string tables of `SwitchAppModel.dll` and `ModeSwitchModule.dll` (see `06-method.md`) and confirmed live on 2026-10-18.

> LG answers **HTTP 200 + body `NotAllowedPage`** (15 bytes, `Last-Modified: 2020-11-24`) for unknown paths. Always validate the body, never the status.

## LG-hosted (`lmu.lge.com`)

| URL | Verified result | Used for |
|---|---|---|
| `https://lmu.lge.com/ExternalService/onscreencontrol/fw/<MODEL>[suffix]/FWLatestVersion.txt` | firmware zip file name, e.g. `MODEL_27GM950B_US3.06_PV4.23.16_NV1.11_20260624.zip` (51 bytes, `Last-Modified` 2026-07-21 for 27GM950B) | **monitor firmware version check** (per model; the suffix is a family path, see `03`) |
| `https://lmu.lge.com/ExternalService/onscreencontrol/fw/<MODEL>[suffix]/<zip name>` | `200 application/zip`, 13,683,937 bytes for 27GM950B 3.06 | firmware download. The app saves it as `fw.zip` and extracts into a folder `fw`. |
| `https://lmu.lge.com/ExternalService/LGSwitch/Win_3.0/LGSwitchLatestVersion.txt` | `LGSwitch_7.60.zip` | LG Switch self-update version check |
| `https://lmu.lge.com/ExternalService/LGSwitch/Win_3.0/<zip name>` | `LGSwitch_7.60.zip`: `200`, 964,329,448 bytes, `Last-Modified` 2026-05-22 | LG Switch self-update package (HEAD only; not downloaded) |
| `https://lmu.lge.com/ExternalService/LGSwitch/config/ai_config.json` | `{"ai_picture_mode_show":{"status":"Enable","date":"2025.11.22","ai_model_version":"20250818","history":["2025.11.22 first implementation"]}}` | AI Picture Mode feature flag + model version |
| `https://lmu.lge.com/ExternalService/LGSwitch/WallPapers/wallpaper_list_new.txt` | 718 bytes, lines `thumbs.zip;video.zip` (`Last-Modified` 2024-07-04) | wallpaper pack list |
| `https://lmu.lge.com/ExternalService/LGSwitch/WallPapers/thumbnails_and_previews/ThumbPreview_NN.zip`, `.../Videos_New/Video_NN.zip` | referenced in the list above | wallpaper packs |
| `https://lmu.lge.com/ExternalService/onscreencontrol/service_notice/service_notice_list.json` and `.../notices` | `NotAllowedPage` (nothing published) | "service notices" (unused/unpublished at time of testing) |
| `https://lmu.lge.com/LGSwitch/LGSwitch2.0/LGSwitchPrivacyPolicy.html` | `200 text/html`, 5,955 bytes, `Last-Modified` 2023-04-06 | privacy policy page (also shipped locally as `LGSwitchPrivacyPolicy.html`) |

## Other LG sites referenced

| URL | Used for |
|---|---|
| `https://www.lg.com/global/registrationQR?salesmodel={salesModelNumber}&serialno={serialNumber}` | Product registration (opened from the update/registration UI with the monitor's sales model + serial number) |
| `https://www.lge.co.kr/my-page/reg-owner-product?sku={salesModelNumber}&sn={serialNumber}`, `https://www.lge.co.kr/my-page/manage-products` | Korean registration/management pages |
| `https://www.lg.com/common/index.jsp` | LG home page (menu "LG Home Page") |
| `https://opensource.lge.com` | open-source notices, referenced in the bundled license text |
| `http://www.lg.co.kr` | company URL embedded in the exe's version metadata |

## Third-party

| URL | Used for |
|---|---|
| `https://developers.google.com/calendar/api/v3/reference/events/insert`, Google OAuth endpoints (`oauth2/v3/certs`, `gstatic.com/iap/verify/public_key-jwk`) | Google Calendar integration for the Scheduler / video-call features (OAuth token stored as `token.json`; the app has code to remove it) |
| `https://drive.google.com/file/d/1-5LAX5oTDA8Xrv_bsc9MRmFA7qT9oxkg/view?usp=sharing` | A Google Drive document linked from the app (not fetched) |
| `https://zoom.us`, `https://www.webex.com`, `https://www.microsoft.com/en-in/microsoft-teams` | Video-call app launchers |
| `http://kopico.go.kr`, `http://privacy.kisa.or.kr`, `http://spo.go.kr`, `https://ecrm.cyber.go.kr` | Korean privacy/consumer-agency links inside the localized privacy text |
| `https://github.com/...` (CommunityToolkit, nlohmann/json, rapidxml, NXPmicro/lpcusbsio, Prism, mpv-player, …) | Open-source license notices |

## Firmware "history" is local, not server-side

The server keeps **only the latest** firmware per model (an older 3.05 file name for the 27GM950B returned `NotAllowedPage`). LG Switch's *Monitor Software Update History* screen is generated from a local file; see `02-local-files-and-registry.md`.

## What is *not* there

No telemetry/analytics endpoint, no login, and no firmware **listing** endpoint was found in the decoded strings. The only thing the app asks the server is the single `FWLatestVersion.txt` for the connected monitor's model. Five executables (the UI exe, the updater service, `SwitchCLI.exe`, the control service and the Thunderbolt-4 updater) use a stricter string decryptor that returned a placeholder when called from outside their own process, so **their strings are not decoded**; `SwitchAppModel.dll` (all networking/firmware logic) and `ModeSwitchModule.dll` (all views) are fully decoded.
