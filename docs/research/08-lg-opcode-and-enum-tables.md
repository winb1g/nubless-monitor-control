# 08 · LG's opcode and enum tables (from `SwitchAppModel.dll`)

Enum **names and numeric values are not obfuscated** in LG Switch 7.60 (only code and strings are), so they can be read straight from the assembly metadata with `tools/enum-dump`. They are LG's own vocabulary for the DDC/CI "ProtocolEngine". Values are decimal as stored; the hex column is the VCP code number.

> Reading is not understanding. A name here tells you what LG calls a code; it does not tell you the value encoding for *this* monitor. Treat every row as a lead and confirm with `tools/vcp-read.ps1` / `tools/vcp-diff.ps1` before relying on it. **Never write a code that is not confirmed.**

## `T_E_OPCODE` (211 members)

Standard MCCS names (`MCCS_OPCODE_<GROUP>_...`) use the standard VESA code numbers. Entries named `MANUFACTURER_*`, `SPSW_*`, `MONITOR_*`, `AIMODE_*` etc. are LG-specific. Several names share a number because they are *sub-commands* of another opcode (for example `SPSW_COMMAND_*` are sub-commands of `SMART_PICTURE_SETTING_WIZARD` 0xDB, and `MONITOR_DUAL_MODE_*` are sub-commands of 0xB1).

| Name | Decimal | Hex |
|---|---:|---:|
| `MISC_DEGAUSS` | 1 | `0x01` |
| `MONITOR_DUAL_MODE_ON_LIST` | 1 | `0x01` |
| `SPSW_COMMAND_START_STOP` | 1 | `0x01` |
| `SPSW_SET_OPTION_VALUE` | 1 | `0x01` |
| `MISC_NEW_CONTROL_VALUE` | 2 | `0x02` |
| `MONITOR_DUAL_MODE_OFF_LIST` | 2 | `0x02` |
| `SPSW_COMMAND_GAMMMA` | 2 | `0x02` |
| `MISC_SOFT_CONTROL_VALUE` | 3 | `0x03` |
| `SPSW_COMMAND_CONTRAST` | 3 | `0x03` |
| `PRESET_RESTORE_FACTORY_DEFAULT` | 4 | `0x04` |
| `SPSW_COMMAND_BLACK_STABILIZER` | 4 | `0x04` |
| `PRESET_RESTORE_FACTORY_LUMINANCE_DEFAULT` | 5 | `0x05` |
| `SPSW_COMMAND_COLOR_TEMP` | 5 | `0x05` |
| `PRESET_RESTORE_FACTORY_GEOMETRY_DEFAULT` | 6 | `0x06` |
| `SPSW_COMMAND_COLORSATURATION` | 6 | `0x06` |
| `SPSW_COMMAND_SHARPNESS` | 7 | `0x07` |
| `PRESET_RESTORE_FACTORY_COLOR_DEFAULT` | 8 | `0x08` |
| `SPSW_COMMAND_BEFORE_AFTER` | 8 | `0x08` |
| `PRESET_RESTORE_FACTORY_TV_DEFAULT` | 10 | `0x0A` |
| `SPSW_COMMAND_VERSION` | 10 | `0x0A` |
| `IMAGE_COLOR_TEMPARATURE_INCREMENT` | 11 | `0x0B` |
| `IMAGE_COLOR_TEMPARATURE_REQUEST` | 12 | `0x0C` |
| `IMAGE_CLOCK` | 14 | `0x0E` |
| `IMAGE_LUMINANCE` | 16 | `0x10` |
| `IMAGE_FLESH_TONE_ENHANCEMENT` | 17 | `0x11` |
| `MONITOR_DUAL_MODE_LAST_ON` | 17 | `0x11` |
| `IMAGE_CONTRAST` | 18 | `0x12` |
| `MONITOR_DUAL_MODE_LAST_OFF` | 18 | `0x12` |
| `IMAGE_BLACKLIGHT_CONTROL` | 19 | `0x13` |
| `IMAGE_COLOR_PRESET` | 20 | `0x14` |
| `MANUFACTURER_PICTURE_MODE_VALUE` | 21 | `0x15` |
| `IMAGE_RED_VEDIO_GAIN` | 22 | `0x16` |
| `IMAGE_USER_COLOR_VISION_COMPENSATION` | 23 | `0x17` |
| `IMAGE_GREEN_VEDIO_GAIN` | 24 | `0x18` |
| `IMAGE_BLUE_VEDIO_GAIN` | 26 | `0x1A` |
| `IMAGE_FOCUS` | 28 | `0x1C` |
| `IMAGE_AUTO_SETUP` | 30 | `0x1E` |
| `IMAGE_AUTO_COLOR_SETUP` | 31 | `0x1F` |
| `GEOMETRY_HORIZONTAL_POSITION` | 32 | `0x20` |
| `MONITOR_DUAL_MODE_OPTION` | 32 | `0x20` |
| `MONITOR_DUAL_MODE_SWITCH_ON` | 33 | `0x21` |
| `GEOMETRY_HORIZONTAL_SIZE` | 34 | `0x22` |
| `MONITOR_DUAL_MODE_SWITCH_OFF` | 34 | `0x22` |
| `GEOMETRY_HORIZONTAL_PINCUSHION` | 36 | `0x24` |
| `GEOMETRY_HORIZONTAL_PINCUSHION_BALANCE` | 38 | `0x26` |
| `GEOMETRY_HORIZONTAL_CONVERGENCE_RB` | 40 | `0x28` |
| `GEOMETRY_HORIZONTAL_CONVERGENCE_MG` | 41 | `0x29` |
| `GEOMETRY_HORIZONTAL_LINEARITY` | 42 | `0x2A` |
| `GEOMETRY_HORIZONTAL_LINEARITY_BALANCE` | 44 | `0x2C` |
| `IMAGE_GRAY_SCALE_EXPANSION` | 46 | `0x2E` |
| `MONITOR_DUAL_MODE_SWITCH` | 47 | `0x2F` |
| `GEOMETRY_VERTICAL_POSITION` | 48 | `0x30` |
| `GEOMETRY_VERTICAL_SIZE` | 50 | `0x32` |
| `GEOMETRY_VERTICAL_PINCUSHION` | 52 | `0x34` |
| `GEOMETRY_VERTICAL_PINCUSHION_BALANCE` | 54 | `0x36` |
| `GEOMETRY_VERTICAL_CONVERGENCE_RB` | 56 | `0x38` |
| `GEOMETRY_VERTICAL_CONVERGENCE_MG` | 57 | `0x39` |
| `GEOMETRY_VERTICAL_LINEARITY` | 58 | `0x3A` |
| `BENDABLE_MONITOR_CONTROL` | 59 | `0x3B` |
| `GEOMETRY_VERTICAL_LINEARITY_BALANCE` | 60 | `0x3C` |
| `IMAGE_CLOCK_PHASE` | 62 | `0x3E` |
| `GEOMETRY_HORIZONTAL_PARELLELOGRAM` | 64 | `0x40` |
| `GEOMETRY_VERTICAL_PARELLELOGRAM` | 65 | `0x41` |
| `GEOMETRY_HORIZONTAL_KEYSTONE` | 66 | `0x42` |
| `GEOMETRY_VERTICAL_KEYSTONE` | 67 | `0x43` |
| `GEOMETRY_ROTATIONS` | 68 | `0x44` |
| `MANUFACTURER_GET_PICTURE_MODE_LIST3` | 69 | `0x45` |
| `GEOMETRY_TOP_CORNER_FLARE` | 70 | `0x46` |
| `MANUFACTURER_GET_PICTURE_MODE_LIST4` | 71 | `0x47` |
| `GEOMETRY_TOP_CORNER_HOOK` | 72 | `0x48` |
| `GEOMETRY_BOTTOM_CORNERF_LARE` | 74 | `0x4A` |
| `GEOMETRY_BOTTOM_CORNER_HOOK` | 76 | `0x4C` |
| `MANUFACTURER_GET_PICTURE_MODE_LIST` | 77 | `0x4D` |
| `MANUFACTURER_GET_PICTURE_MODE_LIST1` | 78 | `0x4E` |
| `MANUFACTURER_GET_PICTURE_MODE_LIST2` | 79 | `0x4F` |
| `MISC_ACTIVE_CONTROL` | 82 | `0x52` |
| `MISC_PERFORMANCE_PRESERVATION` | 84 | `0x54` |
| `DPVL_KEYBOARD_MOUSE_SWITCH` | 85 | `0x55` |
| `IMAGE_HORIZONTAL_MOIRE` | 86 | `0x56` |
| `IMAGE_VERTICAL_MOIRE` | 88 | `0x58` |
| `IMAGE_SIX_AXIS_SATURATION_CONTROLRED` | 89 | `0x59` |
| `IMAGE_SIX_AXIS_SATURATION_CONTROLYELLOW` | 90 | `0x5A` |
| `IMAGE_SIX_AXIS_SATURATION_CONTROLGREEN` | 91 | `0x5B` |
| `IMAGE_SIX_AXIS_SATURATION_CONTROLCYAN` | 92 | `0x5C` |
| `IMAGE_SIX_AXIS_SATURATION_CONTROLBLUE` | 93 | `0x5D` |
| `IMAGE_SIX_AXIS_SATURATION_CONTROLMAGENTA` | 94 | `0x5E` |
| `MISC_INPUT_SOURCE` | 96 | `0x60` |
| `AUDIO_SPEAKER_VOLUME` | 98 | `0x62` |
| `MANUFACTURER_VOLUME` | 98 | `0x62` |
| `AUDIO_MICROPHONE_VOLUME` | 100 | `0x64` |
| `MISC_AMBIENT_LIGHT_SENSOR` | 102 | `0x66` |
| `COLOR_TEMP_VALUE` | 105 | `0x69` |
| `COLOR_TEMP_LIST` | 106 | `0x6A` |
| `IMAGE_RED_VEDIOBLACK_LEVEL` | 108 | `0x6C` |
| `IMAGE_GREEN_VEDIOBLACK_LEVEL` | 110 | `0x6E` |
| `IMAGE_BLUE_VEDIOBLACK_LEVEL` | 112 | `0x70` |
| `IMAGE_GAMMA` | 114 | `0x72` |
| `IMAGE_LUTSIZE` | 115 | `0x73` |
| `IMAGE_SINGLEPOINT_LUT_OPERATION` | 116 | `0x74` |
| `IMAGE_BLOCK_LUT_OPERATION` | 117 | `0x75` |
| `MISC_REMOTR_PROCEDURE_CALL` | 118 | `0x76` |
| `MISC_EDID_OPERATION` | 120 | `0x78` |
| `IMAGE_GET_GAMMA_LIST` | 122 | `0x7A` |
| `IMAGE_ADJUST_ZOOM` | 124 | `0x7C` |
| `GEOMETRY_HORIZONTAL_MIRROR_FLIP` | 130 | `0x82` |
| `GEOMETRY_VERTICAL_MIRROR_FLIP` | 132 | `0x84` |
| `SMART_PICTURE_SETTING_WIZARD_APPLIED` | 133 | `0x85` |
| `GEOMETRY_DISPLAY_SCALING` | 134 | `0x86` |
| `IMAGE_SHARPNESS` | 135 | `0x87` |
| `IMAGE_VELOCITY_SCAN_MODULATION` | 136 | `0x88` |
| `IMAGE_COLOR_SATURATION` | 138 | `0x8A` |
| `MISC_TVCHANNEL_UPORDOWN` | 139 | `0x8B` |
| `IMAGE_TV_SHARPNESS` | 140 | `0x8C` |
| `AUDIO_MUTE` | 141 | `0x8D` |
| `AUDIO_TREBLE` | 141 | `0x8D` |
| `IMAGE_TV_CONTRAST` | 142 | `0x8E` |
| `IMAGE_HUE` | 144 | `0x90` |
| `AUDIO_BASS` | 145 | `0x91` |
| `IMAGE_TV_BLACKLEVELLUMINANCE` | 146 | `0x92` |
| `AUDIO_BALANCELR` | 147 | `0x93` |
| `AUDIO_STEREO_MODE` | 148 | `0x94` |
| `GEOMETRY_WINDOW_POSITION_TLX` | 149 | `0x95` |
| `GEOMETRY_WINDOW_POSITION_TLY` | 150 | `0x96` |
| `GEOMETRY_WINDOW_POSITION_BRX` | 151 | `0x97` |
| `GEOMETRY_WINDOW_POSITION_BRY` | 152 | `0x98` |
| `IMAGE_WINDOW_BACKGROUND` | 154 | `0x9A` |
| `IMAGE_SIX_AXIS_HUE_CONTROLRED` | 155 | `0x9B` |
| `IMAGE_SIX_AXIS_HUE_CONTROLYELLOW` | 156 | `0x9C` |
| `IMAGE_SIX_AXIS_HUE_CONTROLGREEN` | 157 | `0x9D` |
| `IMAGE_SIX_AXIS_HUE_CONTROLCYAN` | 158 | `0x9E` |
| `IMAGE_SIX_AXIS_HUE_CONTROLBLUE` | 159 | `0x9F` |
| `IMAGE_SIX_AXIS_HUE_CONTROLMAGENTA` | 160 | `0xA0` |
| `GET_MONITOR_ID_EXTENDED` | 161 | `0xA1` |
| `SPSW_GET_CONFIG_STATUS` | 161 | `0xA1` |
| `GET_MONITOR_ID_EXTENDED_2` | 162 | `0xA2` |
| `IMAGE_AUTO_SETUP_ONOFF` | 162 | `0xA2` |
| `IMAGE_WINDOCONTROL_ONOFF` | 164 | `0xA4` |
| `IMAGE_WINDOW_SELECT` | 165 | `0xA5` |
| `AIMODE_DISPLAY_SETTINGS` | 166 | `0xA6` |
| `AIMODE_PCPREDICTION_CLASS` | 167 | `0xA7` |
| `IMAGE_SCREEN_ORIENTATION` | 170 | `0xAA` |
| `DISPLAY_HORIZONTAL_FREQUENCY` | 172 | `0xAC` |
| `DISPLAY_VERTICAL_FREQUENCY` | 174 | `0xAE` |
| `MONITOR_MODEL_NAME` | 175 | `0xAF` |
| `PRESET_SETTINGS` | 176 | `0xB0` |
| `MONITOR_DUAL_MODE` | 177 | `0xB1` |
| `MISC_FLATPANEL_SUBPIXEL_LAYOUT` | 178 | `0xB2` |
| `MISC_DISPLAY_TECHNOLOGY_TYPE` | 182 | `0xB6` |
| `DPVL_MONITOR_STATUS` | 183 | `0xB7` |
| `DPVL_PACKET_COUNT` | 184 | `0xB8` |
| `DPVL_MONITOR_X_ORIGIN` | 185 | `0xB9` |
| `DPVL_MONITOR_Y_ORIGIN` | 186 | `0xBA` |
| `DPVL_HEADER_ERROR_COUNT` | 187 | `0xBB` |
| `DPVL_BODY_CRC_ERROR_COUNT` | 188 | `0xBC` |
| `DPVL_CLIENT_ID` | 189 | `0xBD` |
| `DPVL_LINK_CONTROL` | 190 | `0xBE` |
| `DISPLAY_DISPLY_USAGE_TIME` | 192 | `0xC0` |
| `MISC_DISPLAY_DESCRIPTOR_LENGTH` | 194 | `0xC2` |
| `MISC_TRANSMIT_DISPLAY_DESCRIPTOR` | 195 | `0xC3` |
| `MISC_ENABLE_DISPLAY_OF_DISPLAY_DESCRIPTOR` | 196 | `0xC4` |
| `MISC_APPLICATION_ENABLEKEY` | 198 | `0xC6` |
| `MISC_DISPLAY_ENABLEKEY` | 199 | `0xC7` |
| `DISPLAY_DISPLY_CONTROLLER_TYPE` | 200 | `0xC8` |
| `DISPLAY_DISPLY_FIRMWARE_LEVEL` | 201 | `0xC9` |
| `DISPLAY_OSD` | 202 | `0xCA` |
| `DISPLAY_OSD_LANGUAGE` | 204 | `0xCC` |
| `MISC_STATUS_INDICATORS` | 205 | `0xCD` |
| `MISC_AUXILLARY_DISPLAY_SIZE` | 206 | `0xCE` |
| `MISC_AUXILLARY_DISPLAY_DATA` | 207 | `0xCF` |
| `MISC_OUTPUT_SELECT` | 208 | `0xD0` |
| `IMAGE_STEREO_VEDIO_MODE` | 212 | `0xD4` |
| `DISPLAY_POWER_MODE` | 214 | `0xD6` |
| `MISC_AUXILLARY_POWER_OUTPUT` | 215 | `0xD7` |
| `DPVL_POWER_ON_OFF` | 216 | `0xD8` |
| `GEOMETRY_SCAN_MODE` | 218 | `0xDA` |
| `SMART_PICTURE_SETTING_WIZARD` | 219 | `0xDB` |
| `IMAGE_DISPLAY_APPLICATION` | 220 | `0xDC` |
| `MISC_OPERATION_MODE` | 222 | `0xDE` |
| `DISPLAY_VCP_VERSION` | 223 | `0xDF` |
| `EYE_FINE_WINDOW` | 224 | `0xE0` |
| `DUAL_WEB_MODE` | 225 | `0xE1` |
| `HDR_VALUE` | 226 | `0xE2` |
| `SMARTDUALMON` | 227 | `0xE3` |
| `MANUFACTURER_CALIBATION_COMMAND` | 228 | `0xE4` |
| `MANUFACTURER_GETGAMMA_FROM_SCALER_COMMAND` | 229 | `0xE5` |
| `MANUFACTURER_WRITE_GAMMA_LUT` | 230 | `0xE6` |
| `MANUFACTURER_WRITE_COLOR_MATRIX` | 231 | `0xE7` |
| `MANUFACTURER_SCALERBLOCK_ENABLE_DISABLE_COMMAND` | 232 | `0xE8` |
| `MANUFACTURER_ENABLE_DISABLE_SCALER_PATTERN` | 233 | `0xE9` |
| `MANUFACTURER_DISPLAY_SCALER_INTERNAL_PATTERN` | 234 | `0xEA` |
| `MANUFACTURER_WRITE_NEW_COLOR_MATRIX` | 235 | `0xEB` |
| `SMARTRESOLUTION` | 236 | `0xEC` |
| `MANUFACTURER_PIPMODE_OPCODE` | 237 | `0xED` |
| `SCREENSPLIT_OSD_MENU` | 238 | `0xEE` |
| `MANUFACTURER_DISPLAYID` | 239 | `0xEF` |
| `MANUFACTURER_HEARTBEAT` | 240 | `0xF0` |
| `MANUFACTURER_FENGINE_BRIGHTNESS` | 241 | `0xF1` |
| `MANUFACTURER_FENGINE_ACE` | 242 | `0xF2` |
| `MANUFACTURER_FENGINE_RCM` | 243 | `0xF3` |
| `MANUFACTURER_GET_RATIO_LIST` | 244 | `0xF4` |
| `MANUFACTURER_RATIO_VALUE` | 245 | `0xF5` |
| `MANUFACTURER_SES` | 246 | `0xF6` |
| `MANUFACTURER_RESPONSE_TIME` | 247 | `0xF7` |
| `MANUFACTURER_FREE_SYNC` | 248 | `0xF8` |
| `MANUFACTURER_BLACK_STABILIZER` | 249 | `0xF9` |
| `MANUFACTURER_MOTION_BLUR` | 250 | `0xFA` |
| `MANUFACTURER_SCALER_ISP_AREA` | 251 | `0xFB` |
| `MANUFACTURER_FENGINE_MODE_SELECT` | 252 | `0xFC` |
| `MANUFACTURER_POWER_LAMP_INDICATOR` | 253 | `0xFD` |
| `MANUFACTURER_IMAGE_GAMMA_PRESET` | 254 | `0xFE` |
| `SPSW_GET_OPTION_VALUE` | 255 | `0xFF` |

## `T_E_RESPONSE_TIME`

| Value | Name |
|---:|---|
| 0 | `eRESPONSE_TIME_OFF` |
| 1 | `eRESPONSE_TIME_HIGH` |
| 2 | `eRESPONSE_TIME_MIDDLE` |
| 3 | `eRESPONSE_TIME_LOW` |
| 255 | `eRESPONSE_TIME_UNSUPPORTED` |

## `T_E_FREE_SYNC`

| Value | Name |
|---:|---|
| 0 | `eFREE_SYNC_OFF` |
| 1 | `eFREE_SYNC_ON` |
| 2 | `eFREE_SYNC_BASIC` |
| 3 | `eFREE_SYNC_EXTENDED` |
| 255 | `eFREE_SYNC_UNSUPPORTED` |

## `T_E_BLACKSTABILIZER_VAL`

| Value | Name |
|---:|---|
| 0 | `eBLACKSTABILIZER_OFF` |
| 1 | `eBLACKSTABILIZER_LOW` |
| 2 | `eBLACKSTABILIZER_MIDDLE` |
| 3 | `eBLACKSTABILIZER_HIGH` |
| 255 | `eBLACKSTABILIZER_UNSUPPORTED` |

## `T_E_COLOR_TEMP`

| Value | Name |
|---:|---|
| 0 | `eColorTemp_Warm` |
| 1 | `eColorTemp_Medium` |
| 2 | `eColorTemp_Cool` |
| 3 | `eColorTemp_Custom` |
| 4 | `eColorTemp_6500K` |
| 5 | `eColorTemp_7500K` |
| 6 | `eColorTemp_9300K` |
| 7 | `eColorTemp_Manual1` |
| 8 | `eColorTemp_Manual2` |
| 9 | `eColorTemp_Manual3` |

## `T_E_GAMMA_MODE`

| Value | Name |
|---:|---|
| 0 | `eGammaOff` |
| 1 | `eGamma0` |
| 2 | `eGamma1` |
| 3 | `eGamma2` |
| 4 | `eMode1` |
| 5 | `eMode2` |
| 6 | `eMode3` |
| 7 | `eMode4` |
| 8 | `eGamma1dot8` |
| 9 | `eGamma2dot0` |
| 10 | `eGamma2dot2` |
| 11 | `eGamma2dot4` |
| 12 | `eGamma2dot6` |
| 13 | `eDicomGammaCurve` |

## `T_E_ASPECT_RATIO`

| Value | Name |
|---:|---|
| 0 | `eHorizontalDefault` |
| 1 | `e_16_9` |
| 2 | `e_21_9` |
| 3 | `eVerticalDefault` |
| 4 | `e_9_16` |
| 5 | `e_9_21` |

## `T_E_PBP_MODE_OPTION`

| Value | Name |
|---:|---|
| 0 | `ePBPOff` |
| 1 | `eOneRationOne_PBP` |
| 2 | `eFiveRationNine_PBP` |
| 3 | `eSmall_PBP` |
| 4 | `eLarge_PBP` |

## `T_E_AISUPPORTEDPICTUREMODE_VALUES`

| Value | Name |
|---:|---|
| 0 | `eAIMODE_PICTUREMODE_PPW` |
| 1 | `eAIMODE_PICTUREMODE_CUSTOM` |
| 2 | `eAIMODE_PICTUREMODE_GAMER1` |
| 3 | `eAIMODE_PICTUREMODE_GAMER2` |
| 4 | `eAIMODE_PICTUREMODE_MAX_BRIGHTNESS` |
| 5 | `eAIMODE_PICTUREMODE_FPS` |
| 6 | `eAIMODE_PICTUREMODE_RTS` |
| 7 | `eAIMODE_PICTUREMODE_VIVID` |

## `T_E_GAMEPICTURMODE_CATEGORY`

| Value | Name |
|---:|---|
| 0 | `eGeneral` |
| 1 | `eGame` |
| 2 | `eAdvanced` |
| 3 | `eMedical` |
| 4 | `eColorgamut` |

## `T_E_MONITOR_TYPE`

| Value | Name |
|---:|---|
| 0 | `PROFILE` |
| 1 | `GSYNC` |
| 2 | `GSYNC_PROFILE` |
| 3 | `GSYNC1` |
| 4 | `GSYNC2` |
| 5 | `GSYNC1_PROFILE` |
| 6 | `GSYNC2_PROFILE` |
| 7 | `GSYNCR4` |
| 8 | `GSYNCR425GR75FG` |
| 9 | `APPPRESETDEFAULTPICMODE` |
| 10 | `NORMAL` |
| 11 | `FOCUSVIEW` |
| 12 | `MD_MONITOR` |
| 13 | `SMART_MONITOR` |

## `E_PICTUREMODE_NAME`

| Value | Name |
|---:|---|
| 0 | `eCustom` |
| 1 | `eReader` |
| 2 | `eReader1` |
| 3 | `eReader2` |
| 4 | `ePhoto` |
| 5 | `eCinema` |
| 6 | `eGame` |
| 7 | `eRed_Weakness` |
| 8 | `eGreen_Weakness` |
| 9 | `eDark_Room_1` |
| 10 | `eDark_Room_2` |
| 11 | `eFPS_Game_1` |
| 12 | `eFPS_Game_2` |
| 13 | `eRTS_Game` |
| 14 | `eCustom_Game` |
| 15 | `esRGB` |
| 16 | `Adobe` |
| 17 | `Calibration` |
| 18 | `Calibration1` |
| 19 | `Calibration2` |
| 20 | `DICOM` |
| 21 | `DCI` |
| 22 | `EBU` |
| 23 | `REC709` |
| 24 | `SMPTE_C` |
| 25 | `DCI_P3` |
| 26 | `Custom_AV` |
| 27 | `Vivid1` |
| 28 | `Vivid2` |
| 29 | `Standard` |
| 30 | `Color_Weakness` |
| 31 | `Cinema_AV` |
| 32 | `Text` |
| 33 | `HDR_Effect` |
| 34 | `HDR_Vivid` |
| 35 | `HDR_Standard` |
| 36 | `HDR_Game` |
| 37 | `HDR_Cinema` |
| 38 | `Mono` |
| 39 | `Gamer1` |
| 40 | `Gamer2` |
| 41 | `Vivid` |
| 42 | `REC2020` |
| 43 | `FPS` |
| 44 | `RTS` |
| 45 | `Pathology` |
| 46 | `Dicom_Clinical` |
| 47 | `Extention` |
| 48 | `Dicom_Diagnostic` |
| 49 | `HGIG` |
| 50 | `P3_D65_PQ` |
| 51 | `BT_2100_PQ` |
| 52 | `BT_2100_HLG` |
| 53 | `BT_709` |
| 54 | `BT_2020` |
| 55 | `P3_D65` |
| 56 | `DICOM_Custom` |
| 57 | `P3_D65_PQ2` |
| 58 | `BT_2100_PQ2` |
| 59 | `BT_2100_HLG2` |
| 60 | `eSmart_Picture` |
| 61 | `MaxBrightness` |
| 62 | `Studio1` |
| 64 | `Studio2` |
| 65 | `Studio3` |

## `T_E_SET_OPCODE_REPLY_MESSAGE`

| Value | Name |
|---:|---|
| 0 | `eBrightness` |
| 1 | `eContrast` |
| 2 | `eSharpness` |
| 3 | `ePictureMode` |
| 4 | `eScreen` |
| 5 | `eKVMSwitch` |
| 6 | `ePBPValue` |
| 7 | `ePWPConfigValue` |
| 8 | `eAll` |
| 9 | `eRed` |
| 10 | `eGreen` |
| 11 | `eBlue` |
| 12 | `eManual` |
| 13 | `eUpdateColorTempOnMonitorChange` |
| 14 | `eUpdateColorTempSelection` |
| 15 | `eBlackStabilizer` |
| 16 | `eResponseTime` |
| 17 | `eFreeSync` |
| 18 | `eSoCUpScalingList` |
| 19 | `eSoCUpScaling` |
| 20 | `eSOCMode` |
| 21 | `eAISound` |
| 22 | `eAspectRatio` |

## `E_COMMANDS`

| Value | Name |
|---:|---|
| 0 | `eSetSSFullScreen` |
| 1 | `eSetSS16_5` |
| 2 | `eSetSS2SBalanced` |
| 3 | `eSetSSHorizontal` |
| 4 | `eSetSS3SLeft` |
| 5 | `eSetSS3SRight` |
| 6 | `eSetSS3STop` |
| 7 | `eSetSS3SBottom` |
| 8 | `eSetSS3SBalanced` |
| 9 | `eSetSS4SBalanced` |
| 10 | `eTeams` |
| 11 | `eWebex` |
| 12 | `eZoomApp` |
| 13 | `eClose` |
| 14 | `eMinimize` |
| 15 | `eHelpManual` |
| 16 | `eLGHomePage` |
| 17 | `eQuickTutorial` |
| 18 | `eConnectMyGoogle` |
| 19 | `eTutorialPrev` |
| 20 | `eTutorialNext` |
| 21 | `eGoogleCalendarPolicy` |
| 22 | `eCustomSplitEdit` |
| 23 | `eCustomSplitApply` |
| 24 | `eColorAdjust` |
| 25 | `eProductRegistration` |

## `T_E_REQUEST_TYPE` (UI to control-service pipe messages)

`eRequestNone`, `eInitializeStatus`, `eInitializeStatusReply`, `eGetSupportedOpcodeList`, `eGetSupportedOpcodeListFailed`, `eSupportedOpcodeList`, `eSetOpcodeRequest`, `eSetOpcodeReply`, `eSetSmartMonitorInfo`, `eGetAllOpcodeRequest`, `eGetAllOpcodeReply`, `eGetOpcodeRequest`, `eGetOpcodeReply`, `eScreenSplit`, `eApplicationPresetm`, `eGetScreenSplitLayout`, `eApplicationPosition`, `ePreviewWindowMoveStarted`, `ePreviewWindowDropped`, `eLayoutInitialization`, `ePreviewLayoutWindowClicked`, `eGetGameWindowOpcode`, `eGetMonitorControlOpcode`, `eGetMonitorControlOpcodeReply`, `eProcessRequestForAllMonitor`, `eExitOSCAppManager`, `eModifiedInfoOfMonitor`, `eGetColorAdjustOpcode`, `eGetProfileOpcode`, `eSetScreenSplitNone`, `eGetMonitorModelName`, `eGetGameSSOpcodeList`, `eGameSSOpcodeList`, `ePivotChange`, `eSmartMonitorInfoReceived`, `eSmartMonitorDisplayPresetInfo`, `eModelNamePIDVID`, `eSetPivotStatus`, `eFWUpdateSdkStatus`, `eSSUpdateHotkey`, `eGetAdvancedOpcodesRequest`, `eSwitchAppMode`, `eScreenSplitUpdate`, `eGetScreenOpcodeList`, `eGetScreenOpcodeListReply`, `eSPSWWizardOn`, `eGetProductRegInfoOpcode`, `eGetProductRegInfoOpcodeReply`, `eGetTBT5FWVersion`, `eGetTBT5FWVersionReply`, `eStartTBT5FWUpdate`, `eStopTBT5FWUpdate`, `eTBT5FWUpdateProgressReply`, `eTBT5FWUpdateSuccess`, `eTBT5FWUpdateFail`, `eGetSoCPCAIOpcode`, `eEventMgrResponse`, `eEventMgrKMSwitchResponse`, `eAIModelPredictionInitResponse`, `eAIModelPredictionResponse`

