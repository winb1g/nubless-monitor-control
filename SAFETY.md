# Safety contract

This file is the contract. A change that weakens any rule below needs a very good reason, a test, and a second reviewer.

## 1. Never brick the display

| Rule | Where it is enforced |
|---|---|
| A write can only be built by `safety::check_write`; the transport only accepts that type (`ValidatedWrite` has private fields). | `crates/nubless-core/src/safety.rs`, `transport.rs` |
| Hard deny-list, checked before anything else: factory restores (`0x04 0x05 0x06 0x08 0x0A`), LUT/RPC (`0x73`–`0x76`), EDID (`0x78`), power (`0xD6 0xD8`), OSD lock/language (`0xCA 0xCC`), presets (`0xB0`), LG AI/Dual-Mode/wizard sub-command channels (`0xA6 0xA7 0xB1 0xDB`), LG calibration/gamma/color matrix (`0xE4`–`0xEB`), and the unmapped LG codes `0xEF 0xF0 0xFD`–`0xFF`. | `safety::deny_reason`; test `deny_list_beats_any_profile` |
| Only codes in the model profile can be written; only if `writable`; only if the evidence is good enough for the active policy; only if the monitor declares the code; only values from the profile's range or list. | `safety::check_write`, `profile::WritePolicy::allows` |
| Default policy: standard MCCS codes need a read-back match with LG's UI; LG-private codes need a recorded write round-trip. `--experimental` lowers that to "strong" evidence, never lower. | `WritePolicy` |
| No raw "write any VCP code" command exists. | CLI has none, by design |
| Firmware updates: never. | not implemented, not planned |

## 2. Never let the settings get into a mess

The goal is prevention: you should never reach a state you then have to fix, in the app or with the OSD. There is deliberately no change-history screen; the journal below is internal crash protection only.

| Rule | Where |
|---|---|
| **Safe ranges.** Values that could make the picture unreadable are outside the slider range (contrast starts at 20). Your own earlier values are still restored exactly. | profile ranges in `profiles/gm950b.rs`, test `reset_restores_values_below_the_safe_range` |
| **Original settings are captured at startup**, on the first connection and before any write is possible. If they can't be saved, the app makes no changes at all. "Reset to my original settings" puts them back. The file is only ever added to, never overwritten. | `baseline::ensure`, app worker `writes_allowed`, tests in `baseline.rs` |
| **Interlocks** keep the monitor in feature combinations its firmware expects (section 3). | `rules.rs` |
| If the current value can't be read, nothing is written (we couldn't undo it). | `Controller::write`, test `unreadable_current_value_blocks_the_write` |
| Every write is journaled **before** it is sent, with the old value, so even a crash leaves an undo record. The journal file is written then renamed, so it can't be half-written. | `journal.rs` |
| After writing, read back. If the monitor didn't take the value, write the old one back and say which setting may be locking it. | `Controller::apply`, test `refused_write_is_rolled_back` |
| In the CLI, changes that can hide the picture or resync the panel must be confirmed within 15 s, or they revert. The app relies on safe ranges instead, plus a short "Undo" toast that restores the value from before the gesture. | `Risk::Visibility`/`Resync`, CLI `confirmed_within` |
| Undo always works for anything nubless wrote, even if the old value is outside our UI range or the policy has changed since. | `controller::check_restore`, tests `experimental_write_stays_undoable_after_policy_changes`, `snapshot_restore_round_trip_including_out_of_range_values` |
| Anything that could cut the DDC/CI link (input switch, power) is a `DeadEnd` risk and is never written, whatever its confidence. | `Risk::DeadEnd` |
| Turning a feature **off** is always allowed, immediately, without a question. Every unavailable control says why and what to change. | `rules.rs`, tests `turning_off_is_always_immediate`, `every_unavailable_control_explains_itself` |
| A failed read is "unknown", never 0. Reads are retried; all messages are paced (≥ 60 ms apart). | `Controller::with_retries`, `Timing::HARDWARE` |

## 3. Behave like LG Switch where it matters

Feature interlocks follow LG Switch's own logic (evidence in [`docs/rules.md`](docs/rules.md)), so the monitor never ends up in a combination LG's firmware doesn't expect.

## Reporting a problem

If nubless ever leaves your monitor in a state you had to fix with the OSD, that is a **bug of the highest priority**. Please open an issue with `nubless journal` output and `nubless caps`.
