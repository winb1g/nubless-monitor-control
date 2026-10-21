# Contributing

Thanks for helping. Read [`SAFETY.md`](SAFETY.md) first; it outranks everything else here.

- **Mapping** (most needed): follow [`docs/mapping-guide.md`](docs/mapping-guide.md) and pick an item from [`docs/verify-on-hardware.md`](docs/verify-on-hardware.md). A test card in the PR is required to raise a control's confidence ([`docs/hardware-test-cards.md`](docs/hardware-test-cards.md) has the exact steps; the tests refuse a promotion that skips the evidence).
- **Code**: `cargo fmt`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`. New behaviour in `nubless-core` needs a test, using the mock monitor (`nubless_core::mock::MockMonitor`, whose `faults` simulate dropped replies, lost writes and a vanished link). CLI behaviour is tested end to end in `crates/nubless-cli/tests`.
- **Desktop UI**: use the tokens and components in [`docs/design-system.md`](docs/design-system.md), and run `npm run check`, `npm run lint:tokens` and `npm run test:ui` in `apps/desktop` (the last drives the app in a browser against the demo monitor; it needs Chrome or Chromium: set `CHROME_PATH` if it isn't found). Open `/#/kit` in `npm run dev` to see every component in every state.
- **Never** add a raw-write command, a firmware path, or a code to a profile that is on the deny-list.
- Do not commit LG binaries, decoded string dumps, or LG's UI text. Describe findings in your own words.
- Other models: not yet. The 27GM950B comes first.
