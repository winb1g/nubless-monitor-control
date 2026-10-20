# Hardware test cards: Labs controls

One card per control that is still behind Labs (or only read-checked). Each takes a few minutes with a 27GM950B and its on-screen menu (OSD). A passing card is what lets a control leave Labs; see [Promoting a control](#promoting-a-control).

The mock monitor, the fault tests and the browser tests already prove the *software* does the right thing with any answer the monitor gives. These cards only answer the questions software can't: **does this raw value mean what we think, and does the panel take it?**

## Before you start

1. Quit LG Switch (its service also talks DDC/CI). DDC/CI must be on in the monitor's menu.
2. `nubless list` shows the monitor as supported; `nubless get firmware` prints the firmware (record it; the evidence stamp needs it, for example `3.05`).
3. Save a way back: `nubless snapshot save before.json`. At the end: `nubless snapshot restore before.json`. (`nubless snapshot restore --baseline` also works.)
4. Use `--experimental` for Labs controls. Risky changes ask **"Keep this change?"** and put the old value back after **5 s** without a `y`. Use `--revert-after 10` if you need longer to look at the screen.

`-> y` below means: answer `y` at the "Keep this change?" question. Write down what the **OSD** shows, not what nubless says: the OSD is the evidence.

## Cards

### Contrast `0x12` (safe policy, no `--experimental`)

```
nubless get contrast                        # OSD contrast shows the same number
nubless set contrast 60   -> y              # OSD shows 60
nubless set contrast 10                     # must be refused (safe range starts at 20)
nubless undo                                # OSD shows 70 again (or what it was)
```

### Volume `0x62` and Mute `0x8D`

```
nubless --experimental get volume; nubless --experimental get mute
nubless --experimental set volume 25        # OSD volume shows 25 (play audio over DisplayPort/HDMI to hear it)
nubless --experimental set mute muted       # audio stops; raw 1?
nubless --experimental set mute unmuted     # audio returns; raw 2?
nubless undo; nubless undo                  # both go back
```

### Response Time `0xF7`

The OSD offers **Off, Normal, Fast, Faster**. We assume raw `0 / 3 / 2 / 1` (Off = 0 from LG's enum and the monitor's own `F7(00 01 02 03)`; the rest from the same enum, label order inferred).

```
nubless --experimental set response-time off      -> y    # OSD says: ______ (raw 0)
nubless --experimental set response-time normal   -> y    # OSD says: ______ (raw 3)
nubless --experimental set response-time fast     -> y    # OSD says: ______ (raw 2)
nubless --experimental set response-time faster   -> y    # OSD says: ______ (raw 1)
nubless --experimental set response-time faster --yes; nubless --experimental get response-time
                                                          # does the first read already show Faster? yes / no
nubless --experimental set response-time normal --revert-after 3     # answer nothing: back to what it was in 3 s?
```

Record whether the picture blanked or flickered and for how long.

### Adaptive-Sync / VRR `0xF8`

The OSD offers Extended, Basic, Off. We assume raw `0 / 2 / 3` = Off / Basic / Extended. Do this with the PC's display connected over DisplayPort.

```
nubless --experimental set adaptive-sync off      -> y    # OSD says: ______ (raw 0)
nubless --experimental set adaptive-sync basic    -> y    # OSD says: ______ (raw 2)
nubless --experimental set adaptive-sync extended -> y    # OSD says: ______ (raw 3)
```

Record: does the screen go black while it switches, and for how many seconds? Does `nubless get adaptive-sync` right after the write already show the new value, or the old one for a moment? (If the old one, tell us: the controller waits one extra read for that, and the number tells us whether it is enough.)

### Black Stabilizer `0xF9`

The monitor reports a maximum of 255; nubless allows `0..=100`. Compare with the OSD's own scale.

```
nubless --experimental set black-stabilizer 0     -> y    # OSD shows ___
nubless --experimental set black-stabilizer 50    -> y    # OSD shows ___
nubless --experimental set black-stabilizer 51    -> y    # OSD shows ___ (is the scale 0-100, 1:1?)
nubless --experimental set black-stabilizer 100   -> y    # OSD shows ___
```

The OSD list did not record this slider's scale. If its maximum is not 100, or the numbers don't match 1:1, **stop and report it**: the UI range is wrong.

### Read-only mapping (no writes: safe to do first)

These only need `nubless probe` and the OSD; see [`research/10-osd-inventory.md`](research/10-osd-inventory.md) for the full list of what is still unmapped. Highest value first:

1. **Picture modes** `0x15`: select RTS, Vivid, Reader, HDR Effect, sRGB and Max Brightness in the OSD one at a time and run `nubless get picture-mode` (or `probe diff`). Six new raw values unlock all 14 modes.
2. **Aspect Ratio** `0xF5`: Full Wide, Original, Just Scan, then `nubless get aspect-ratio` for each.
3. **Dual-Mode** `0xB1`: all four options, then `nubless get dual-mode`.
4. **AI Upscaling**: write down its OSD options, then `nubless --experimental get ai-upscaling` in each.
5. Unmapped settings (Local Dimming, Sharpness, Crosshair, FPS Counter, Bass Boost, Sound Out, Auto Input Switch, Hexagon Lighting): one `probe save` / `probe diff` pair per option.

### The "Keep this change?" prompt in the app

With Labs on in the app (Settings → Labs):

- Change **Response Time**: the prompt appears with a shrinking line and "Reverts in N s"; **Keep** leaves it, **Revert** puts it back at once, no answer puts it back after 5 s. Check the OSD agrees each time.
- Drag **Black Stabilizer**: one prompt when you let go; **Revert** returns to where the drag started.
- Change **Contrast**: same prompt; **Revert** works.

### Endurance (optional, 5 minutes)

Bash: `for i in $(seq 1 100); do nubless set brightness 40 --yes; nubless set brightness 41 --yes; done`
PowerShell: `1..100 | % { nubless set brightness 40 --yes; nubless set brightness 41 --yes }`

Pass: it never errors, the OSD still answers afterwards, and `nubless get` still matches the OSD. If DDC/CI ever stops answering, note what you changed last and keep `nubless journal`.

## Recording a result

Paste this into the PR for each card:

```
Control:            Response Time
Monitor firmware:   3.05
OSD options seen:   Normal, Fast, Faster
Raw -> OSD:         3=Normal  2=Fast  1=Faster
Write + OSD:        yes
Undo + OSD:         yes
Keep prompt:        kept / reverted / timed out: all behaved
Blanking:           none | ~2 s black on change
Date / tester:      2026-MM-DD / name
```

## Promoting a control

A passing card changes the profile, never just the docs. In one PR:

1. `crates/nubless-core/src/profiles/gm950b.rs`: raise the control's `confidence` (`WriteConfirmed` once every value was written and seen in the OSD), correct any label or range the card disproved, and add the stamp to its `note`: `(fw 3.05, 2026-MM-DD)`.
2. `evidence_tests::PINNED` in the same file: the new level. This is the guard that makes a promotion deliberate.
3. If the control leaves Labs, update `labs_is_exactly_the_documented_set` in the same test module and the "What works today" table in `README.md`.
4. Tick the item in [`verify-on-hardware.md`](verify-on-hardware.md).

`cargo test --workspace` fails until each of these agrees, on purpose.
