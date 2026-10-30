# Needs verification on a real 27GM950B

Each item needs a person with the monitor. The exact commands and what to look for are in [`hardware-test-cards.md`](hardware-test-cards.md); record results with the test card in [`mapping-guide.md`](mapping-guide.md), then raise the control's `confidence` in `crates/nubless-core/src/profiles/gm950b.rs` in the same PR (the tests tell you what else must change).

## Writes (one step and back)

- [x] **Brightness** `0x10`: `nubless set brightness 79` showed 79 in the OSD; `nubless undo` restored 80. Firmware 3.05, 2026-10-18, LG services running alongside. First hardware write.
- [x] **Contrast** `0x12`: write, Keep, Revert and the 5 s timeout agreed with the OSD. Firmware 3.05, 2026-10-30.
- [x] **Volume / mute** `0x62` / `0x8D`: agreed with the OSD. Firmware 3.05, 2026-10-30.
- [x] **Response Time** `0xF7` (fw 3.05, 2026-10-30; the owner reported the OSD agreed for every option, we did not record each raw value): which raw value is *Normal / Fast / Faster*? We assume 3 / 2 / 1. The OSD has an *Off* as well (assumed 0, matching LG's enum and the capabilities string): confirm.
- [x] **Adaptive-Sync/VRR** `0xF8`: Off / Basic / Extended = 0 / 2 / 3, each read with the OSD on that option (fw 3.05, 2026-10-30). Does the screen blank while it switches?
- [x] **Black Stabilizer** `0xF9` (fw 3.05, 2026-10-30, owner-reported): is the OSD range 0–100? Does writing 51 show 51?

## Encodings still unknown

- [x] **Picture mode** `0x15`: all eight tile values read on the wire ([research 09](research/09-ai-channel-and-picture-modes.md)).
- [ ] Switch picture mode from nubless (Labs): Gamer 1 and back, then Calibration1 with AI Scene Optimization on (expect the question, AI off, then the switch).
- [x] **Color temperature**: presets on `0x14` (Warm 3, Medium 5, Cool 9, Custom 11, Manual 65502); the Manual slider is `0x69` in kelvin (W8 4000, 0 6500, C1 6800, C10 10000). Read-only. Still open: the other 15 Manual notches, whether Custom's sliders are `0x16`/`0x18`/`0x1A` in R/G/B order (they unlocked at 50/50/50), and any write. Fw 3.05, 2026-10-30.
- [x] **AI Scene Optimization**, **AI Sound**, **AI Upscaling**: decoded from LG Switch as sub-commands 2, 6 and 4 on VCP 0xA6 ([research 09](research/09-ai-channel-and-picture-modes.md)).
- [x] Turn on Labs and switch AI Scene Optimization and AI Sound from nubless once each, checking the OSD (first writes on 0xA6).
- [x] AI Upscaling: Off 1, Low 2, High 4, read with the OSD on each (fw 3.05, 2026-10-30). The old 1-while-Off report was right. Writing is untested.
- [ ] **Dual-Mode** `0xB1`: what does *Conversion* send? (Read-only until understood.)

## Motion Blur Reduction (needs Adaptive-Sync Off)

- [x] With Adaptive-Sync **Off**, Motion Blur Reduction has two options. `0xFA` reads 1 for On and 2 for Off (fw 3.05, 2026-10-30). Writing Off as 0 or 2 is untested.
- [x] Adaptive-Sync Basic hides Motion Blur Reduction and `0xFA` reads 2: the monitor clears it (fw 3.05, 2026-10-30). `0xF4` reads 6 then, 7 with Adaptive-Sync Off.
- [x] Back to Off: Motion Blur Reduction was Off, not restored (fw 3.05, 2026-10-30).

## Keep or revert (5 s)

- [ ] CLI: a risky change with no answer is put back after 5 s and the OSD agrees (`--revert-after` changes the time).
- [x] App: Response Time, Black Stabilizer and Contrast each ask "Keep this change?"; Keep, Revert and no answer all end where the OSD says they should.
- [ ] Does a read right after a Response Time / VRR write already show the new value, or only after a moment? (The controller waits one extra read; say if that is not enough.)

## Recovery questions (dead-end checks)

- [ ] **Input switch** `0x60`: after switching to an input with no signal, does DDC/CI on DisplayPort still answer? If yes, input switching can be unblocked with an auto-revert.
- [ ] Does the monitor ever stop answering DDC/CI after a write (and need a power cycle)? Keep the journal if it does.

## LG Switch behaviour

- [ ] Does LG Switch ask its AI Picture Mode / My Display Preset question **even when nothing would be disabled**?
- [x] With AI Scene Optimization on, which controls are greyed out? None of the sliders on this monitor (only webOS monitors use those flags); non-AI picture modes are refused by the monitor.
- [x] In which picture modes is AI unavailable? Everything except Personalized Picture, Custom, Gamer 1, Gamer 2, Max Brightness, FPS, RTS and Vivid.
