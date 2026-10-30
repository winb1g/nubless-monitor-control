# Hardware test cards: Labs controls

> **Status, 2026-10-30:** every card below passed on firmware 3.05 and the controls left Labs. The cards stay as the procedure for the next control to enter Labs, and the branch URL in step 2 is out of date: run from `main`.

One card per control that is still behind Labs. Everything below is done **in the app**, with the monitor's own on-screen menu (OSD) open next to it as the judge. A passing card is what lets a control leave Labs; see [Promoting a control](#promoting-a-control).

The mock monitor, the fault tests and the browser tests already prove the *software* does the right thing with any answer the monitor gives. These cards only answer what software can't: **does this raw value mean what we think, and does the panel take it?**

## Before you start (5 minutes)

1. Quit LG Switch (its service also talks DDC/CI). DDC/CI must be on in the monitor's menu.
2. Run the app from this branch. On Windows, one command does it all (installs what's missing, fetches the branch, starts the app); paste it into PowerShell:

   ```powershell
   irm https://raw.githubusercontent.com/winb1g/nubless-monitor-control/claude/admiring-sagan-1dh6ll/scripts/try-branch.ps1 | iex
   ```

   (Or by hand: `cd apps/desktop && npm ci && npx tauri dev`, with Git, Node 20+, Rust and the C++ build tools installed.) A test build works too if one was shared. About → "Monitor firmware" shows the firmware: write it down.
3. Open **Settings → Labs** and turn it on. Your original settings are saved automatically the first time the app connects; **Settings → Reset to my original settings** puts everything back at the end.
4. Open **Displays** and keep the OSD up next to it.

Changes to Contrast, Black Stabilizer, Response Time and Adaptive-Sync show **"Keep this change?"** for 5 seconds. Click **Keep** once you've looked at the screen and the OSD. If you don't answer, it puts the old value back, which is also something to check.

Write down what the **OSD** shows, not what the app shows: the OSD is the evidence.

## Cards

### 1. Response Time (Gaming → Response Time)

The OSD offers **Off, Normal, Fast, Faster**. We assume Off / Normal / Fast / Faster.

For each of the four buttons: click it, then look at the OSD.
- Does the OSD show the same name? (If two names are swapped, say which.)
- Did the picture flicker or go black? For how long?
- Click **Keep** on the last one. Then change it again and **wait 5 seconds without answering**: does it go back by itself, and does the OSD agree?

### 2. Adaptive-Sync / VRR (Gaming → Adaptive-Sync/VRR)

The OSD offers **Extended, Basic, Off**. Do this with the PC connected over DisplayPort.

For each of the three buttons: does the OSD match, and does the screen go black while it switches (for how many seconds)? Try **Revert** once and check the OSD.

### 3. Black Stabilizer (Picture → Black Stabilizer)

The OSD slider is 0–100. Drag the app's slider (or use − / +) to **0, 50, 51 and 100** and compare the number in the OSD each time. The prompt should appear once, when you let go. **Revert** should return to where the drag started.

### 4. Contrast (Picture → Contrast)

Set 60, then look at the OSD. Try to drag below 20: the slider stops at 20 on purpose. Use **Revert** once, then **Keep** once.

### 5. Volume and Mute (Sound)

Play audio over DisplayPort/HDMI. Move the volume slider and compare the OSD's volume. Toggle **Mute**: does the sound stop and start, and does the OSD agree?

### 6. The prompt itself

Besides the checks above, tell us if it ever: doesn't appear when it should, appears for something harmless (brightness, volume, mute shouldn't), covers something you need, or is hard to read.

## Not part of this round

- **Reading unmapped OSD settings** (the six picture modes the app doesn't know, Aspect Ratio, Dual-Mode, Local Dimming, Sharpness, Crosshair, FPS Counter, Bass Boost, Sound Out, Auto Input Switch, Hexagon Lighting). That needs the command-line `nubless probe`, or an in-app mapping helper that doesn't exist yet. The list is in [`research/10-osd-inventory.md`](research/10-osd-inventory.md).
- **Command-line checks** (`nubless set ...`, the endurance loop). Automated tests cover the CLI against the simulated monitor; contributors who want to run it on hardware can follow [`mapping-guide.md`](mapping-guide.md).

## Recording a result

Plain words are fine: per card, what the OSD showed for each option, whether it blanked and for how long, and whether Keep / Revert / the 5 s timeout behaved. The record that goes into the PR looks like this:

```
Control:            Response Time
Monitor firmware:   3.05
Raw -> OSD:         0=Off  3=Normal  2=Fast  1=Faster
Write + OSD:        yes
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
