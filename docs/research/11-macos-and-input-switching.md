# 11 · macOS DDC/CI, and switching inputs over LG's side channel

Found on one 27GM950B (firmware 3.05) connected to a MacBook Pro (M1 Max, macOS 26), 2026-11-18/19. A Dell S2721QS on the same Mac was the control. #15 found and fixed the same DisplayPort problem independently on an M5 Max.

## Each connection, from the Mac

| Mac port → monitor input | DDC/CI | What happens |
|---|---|---|
| Built-in HDMI → HDMI | ✗ | Every write is rejected by the Mac (`0xE0114000`) before it reaches the cable. The Dell on the same port works, so it is this monitor on this port. MonitorControl's troubleshooting wiki lists the "M1 MacBook Pro 14" and 16" HDMI port" as unsupported [1]. On these Macs, HDMI goes through a DP-to-HDMI converter chip (MCDP29xx). The BetterDisplay developer says DDC/CI through it is cut off after the EDID is read, and stray data "usually only drives displays — especially LG displays — mad" (an M1 Max with an LG 27UP850-W on HDMI) [2]. The m1ddc developer had an LG "go crazy" over the M1 Mac mini's HDMI port, needing a power cycle [3]. **Use USB-C or DP.** On an M5 Max (#15), HDMI works. |
| USB-C → USB-C | ✔ with double-send | See below |
| USB-C → DP | ✔ with double-send | See below |

## The null-reply state, and the fix

Sent once, a request gets one good reply after the monitor powers up. After that, every reply is a DDC/CI null message (`6e 80 be`), until the next power cycle. Unplugging the cable doesn't clear it. Reading the reply sooner or later, or spacing requests out, changes nothing.

The bytes after `6e 80 be` look like a valid reply, but they are left over from an earlier one. They don't change when you ask for a different code, so don't treat them as data. (#15 describes the same thing on an M5 Max: the previous reply, with bytes 1–2 overwritten.)

The same header shows up elsewhere: BetterDisplay logged `Malformed DDC reply received [110, 128, 190, …]` (`6e 80 be`) from an LG 27UP650K plugged straight into an M1 Pro MacBook Pro (MacBookPro18,1) [4]. The thread itself is about the same monitor failing behind a Thunderbolt dock. There, opening and quitting MonitorControl, or sending one m1ddc brightness write, made BetterDisplay work afterwards. Nobody there worked out why.

**The fix: write every request twice, 10 ms apart, then read the reply.** That gets a proper reply every time, even from a monitor already in the null state. MonitorControl's `Arm64DDC.swift` writes every request twice by default (`numOfWriteCycles` defaults to 2, with a 10 ms sleep before each write and a 50 ms wait before the read) [5]. Its code gives no reason. kvm-follow, on an LG 34WQ650, also says to send every command twice: "LG panels swallow the first one often enough to be annoying" [6]. LG's own Mac app goes further: it writes every set three times and every read request six times ([`12-lg-switch-mac.md`](12-lg-switch-mac.md)). nubless does the same on macOS (`crates/nubless-hw/src/twice.rs`, #15). On the M1 Max, the same approach was checked over USB-C and DP: capabilities fingerprint, all 23 controls read, a brightness write confirmed in the OSD.

Each input keeps its own brightness (USB-C read 70 while DP read 52).

## Input source

VCP `0x60` reads `0x0F` on both DP and USB-C, so it can't tell them apart. The ddcutil wiki says recent LG monitors "no longer support channel switching via the standard setvcp methods" [7]. kvm-follow found that `0x60` writes on a 34WQ650 are accepted and do nothing [6]. Not tried here. Writing `0x60` stays blocked.

### Switching: VCP `0xF4` over LG's side channel

The command is a normal Set VCP Feature for `0xF4`, except the sender address is `0x50` instead of `0x51`. The ddcutil wiki calls this the "service/factory/manufacturer sidechannel 'DDC2AB'". It says the command doesn't report back whether the switch worked, so ddcutil needs `--noverify` [7]. The BetterDisplay developer confirms that, for input select, "some LG displays expect the command to be sent with data address 0x50 instead of 0x51. All else is the same." [8] The monitor sends no reply, so the screen is the only check.

The wiki contradicts itself on addresses. Its summary lists `x50` for input switching and `x51` for PBP, but a note under the 45GX950A-B says PBP uses `0x50` and input switching uses `0x51` [7]. On this monitor, `0x50` switched inputs.

```
I2C 0x37, write:  50 84 03 F4 <hi> <lo> <checksum>
checksum = 0x6E ^ 0x50 ^ 0x84 ^ 0x03 ^ 0xF4 ^ hi ^ lo
```

Sent twice, 10 ms apart, like every other request. After each switch below, the monitor still answered normal reads.

Once, a switch didn't happen. In the USB-C test, the first `0xD1` (double-sent, both writes accepted by the Mac) didn't switch, and sending it again a few seconds later did. The cause is unknown. Until it's understood, a control built on this should let the user retry, and not assume the first send worked.

| Input | Value | Seen | Status |
|---|---|---|---|
| DisplayPort | `0xD0` | Switched from USB-C; the picture came back on DP | **Confirmed** |
| USB-C | `0xD1` | Showed another computer's picture from its USB-C port, while this Mac stayed on DP (2026-11-19). Earlier, with nothing on USB-C, it went black. | **Confirmed** |
| HDMI 1 | `0x90` | Showed a test device's picture on HDMI 1, on the first send (2026-11-19). Earlier, with nothing on HDMI, it went black. | **Confirmed** |
| HDMI 2 | `0x91` | Showed the test device's picture on HDMI 2 (2026-11-19). Sent twice: nobody was watching the screen after the first send. | **Confirmed** |
| — | `0xD2` | Nothing (sent while on DP) | Not an input on this model. The wiki's value list (from firmware) has `MONITOR_USB_C = 0xD2` [7], and m1ddc's help lists 210 as "USB-C / DP 3" [9], but no model in the wiki's test results confirms `0xD2`. |

Other LG models use the same values. In the ddcutil wiki's test results, `0xD1` = USB-C is confirmed on the 27UN850-WY, 27UP850-W, 27UP85NP-W, 29U531A, 38BR85QC, 40WP95C-W and 45GX950A-B. `0x90`/`0x91` = HDMI 1/2 and `0xD0` = DisplayPort are confirmed on most models listed, including the 45GX950A-B UltraGear [7]. The wiki's value list calls `0xD1` `MONITOR_DP2`, but this monitor has no second DP port.

The codes do vary by model. The 29UM69G uses `0xC0` for DP and `0xE0` for USB-C. The 32UD99-W uses them the other way round [7]. The 34WQ650 uses `0x1D1` for USB-C [6]. So a value from another model needs checking on the model it's used for.

**Checked with a real source on each input:**

- [x] `0xD1` shows a picture from a USB-C source (2026-11-19, on the second send)
- [x] `0x90` shows a picture on HDMI 1 (2026-11-19)
- [x] `0x91` shows a picture on HDMI 2 (2026-11-19)

All four inputs are confirmed on this unit (firmware 3.05). Other units or firmware versions are untested.

Other notes:

- On this model the switch works even while another input is showing. With another computer's picture on USB-C, this Mac sent `0xD0` over DP and got its picture back. So the computer that loses the picture can still send the next switch.
- With **Auto Input Switch** on, an input with no signal falls back to one that has a signal after a few seconds. This is the easy way back after testing a code.
- `0xF4` read on the standard channel (`0x51`) isn't the input. LG Switch's opcode table names it `MANUFACTURER_GET_RATIO_LIST` ([`08-lg-opcode-and-enum-tables.md`](08-lg-opcode-and-enum-tables.md)), and it reads 7 with Adaptive-Sync Off and 6 otherwise ([`../rules.md`](../rules.md)), so the aspect ratios on offer may change with Adaptive-Sync. Writing `0xF4` on the standard channel is on nubless's never-write list: only the `0x50` side channel switches inputs.
- Send only the values in the table. kvm-follow warns: "Do not brute-force the sidechannel", because it's LG's factory service channel [6].
- kvm-follow also found DDC/CI registers are global to the monitor, so either computer can switch inputs whatever is showing [6]. That matches the first point here.

## In nubless

- **CLI:** `nubless switch-input <displayport|usb-c|hdmi-1|hdmi-2>` asks before it sends (`--yes` skips the question), then says the monitor doesn't confirm and to run it again if nothing changed.
- **App:** the Input section on Displays has a button for each input. Each one asks first, then shows a toast with the same advice. No button is shown as selected, because the current input can't be read.
- **How it's kept safe:** `safety::check_input_switch` only allows the four values in the profile's `INPUT_SWITCH`, the ones that showed a picture. It isn't a normal control, so it's never read, snapshotted, journaled or undone, and it's sent once without automatic retries. Even with "Change all together", the app only switches the monitor whose window you used.
- **Platforms:** Apple Silicon Macs (`crates/nubless-hw/src/side_channel.rs`). Windows' DDC/CI API can't change the sender address [6], and the Linux path (`ddcutil --i2c-source-addr=x50` shows it's possible [7]) hasn't been written or tried. Both say it isn't supported yet.

## Sources

Each source was read for the claims cited above, on 2026-11-19.

1. MonitorControl wiki, *Monitor Troubleshooting*, "Known incompatibilities": https://github.com/MonitorControl/MonitorControl/wiki/Monitor-Troubleshooting
2. BetterDisplay #1361, *Monitor LG 27UP850-W support* (MacBookPro18,2 M1 Max, built-in HDMI), reply from waydabber: https://github.com/waydabber/BetterDisplay/issues/1361
3. m1ddc #3, *Mac mini m1 - HDMI port*, reply from waydabber: https://github.com/waydabber/m1ddc/issues/3
4. BetterDisplay discussion #4916, *UGREEN Revodok Max 2131 Thunderbolt 5 dock - DDC support* (LG 27UP650K, MacBookPro18,1): https://github.com/waydabber/BetterDisplay/discussions/4916
5. MonitorControl `Arm64DDC.swift`, `performDDCCommunication`: https://github.com/MonitorControl/MonitorControl/blob/main/MonitorControl/Support/Arm64DDC.swift
6. kvm-follow README (LG 34WQ650, Mac mini M4 Pro and MacBook Pro M1 Pro): https://github.com/xscanlordx-code/kvm-follow
7. ddcutil wiki, *Switching input source on LG monitors*, including "Test results": https://github.com/rockowitz/ddcutil/wiki/Switching-input-source-on-LG-monitors
8. BetterDisplay discussion #4246, *DDC Input Source Control implementation on latest LG displays*, reply from waydabber: https://github.com/waydabber/BetterDisplay/discussions/4246
9. m1ddc README, "Available commands", `input-alt`: https://github.com/waydabber/m1ddc
