# Design system (desktop app)

The desktop UI (`apps/desktop`) has one source of truth for how things look and move:

| What | Where |
|---|---|
| Tokens: color, radius, spacing, type, motion | `src/app.css` (`:root`, plus two dark-theme blocks) |
| Shared classes: `.page`, `.page-head`, `.subtitle`, `.btn`, `.link`, `.tag`, `.eyebrow`, `.mono`, `.muted`, `.small`, `.skeleton`, `.hatch`, `.led` | `src/app.css` |
| Svelte transition presets | `src/lib/motion.ts` (durations read from `app.css`) |
| Building blocks: `Section`, `Item`, `Toggle`, `Slider`, `Segmented`, `ModePicker`, `Lock`, `Hint`, `Logo`, `MonitorArt` | `src/lib/components/` |
| How picture modes are named and grouped | `src/lib/modes.ts` |
| Every component in every state, light and dark | the design kit: `npm run dev`, then open `/#/kit` |
| Enforcement | `node scripts/lint-tokens.mjs` (`npm run lint:tokens`), run in CI |

The look is flat and typographic: no cards, fills or shadows around content. Structure comes from 2px ink rules, hairlines, and type. Rule of thumb: a component's `<style>` uses tokens for every color, radius, font, size and duration. The lint fails on raw hex or `rgb()` colors, gradients, shadows, literal fonts, radii and durations. A line that truly needs a literal (a drawing, say) can end with `lint-tokens: allow`.

## Color

Two jobs, and only two:

- **Ink** is text, every control outline (2px), and "selected" (an ink fill with ground-colored text).
- **Signal** (lime) is "on" and "live": a switch that is on, the Simulated LED, the Update button, the cursor block in the logo. It is always a fill, and on the light ground always inside an ink outline.

| Token | Light | Dark | For |
|---|---|---|---|
| `--ground` | `#E4E4DF` | `#121210` | The page |
| `--wash` | `#D8D8D2` | `#1D1D1A` | Hover and pressed |
| `--ink` | `#0E0E0E` | `#ECECE6` | Text, outlines, selected fills |
| `--dim` | `#55554F` | `#A3A39B` | Secondary text |
| `--rule` | `#A9A9A2` | `#3A3A36` | Dividers. Decoration only; a control's edge is always ink |
| `--signal` | `#C6F135` | `#C6F135` | On and live, as a fill |
| `--sel-mark` | `#C6F135` | `#121210` | AI tags and markers sitting on an ink fill |
| `--ok` `--warn` `--err` | `#2F6B2F` `#9A4A06` `#B3261E` | `#7BD88F` `#FFB000` `#FF8A6B` | Status text and markers, never fills |

`--ink-hover` and `--signal-hover` are the hovered fills, `--on-signal` is the text color on signal (always `#0E0E0E`), `--toast-*` and `--scrim` are for toasts and dialog backdrops.

Contrast (WCAG): ink on ground 15.1:1 light / 15.8:1 dark, dim 5.9:1 / 7.4:1, the status colors 4.9–5.1:1 light / 8.1–10.8:1 dark, ink on signal 14.7:1. **Signal on the light ground is 1.03:1**, which is why it is never text, a line or a lone shape there. In dark it is 14.3:1 and may also be text.

The dark theme redefines only what changes. Add a color to `:root` and, if it should differ in dark, to **both** dark blocks (the media query and `[data-theme="dark"]`). The lint fails if the two blocks drift apart.

## Shape, spacing, type

**Radius.** `--r-sm` 2px (every control) · `--r-md` 4px (dialogs, popovers) · `--r-lg` 6px (the tray frame). Corners are almost square on purpose.

**Spacing.** A 4px grid: `--space-1` 4 … `--space-10` 40. `--hit` (44px) is the minimum size of anything you click, even when it looks smaller (the stepper's chevrons and the slider's − and + are 18px glyphs with invisible padding).

**Type.** Archivo (`--font`) for words, JetBrains Mono (`--font-mono`) for numbers, keys, labels and anything that reads as an instrument. Both are bundled (`@fontsource-variable`), because the Tauri CSP only allows `font-src 'self'`. Sizes: `--fs-2xs` 10 (tags) · `-xs` 11 (mono labels) · `-sm` 13 · `-md` 14 · `-base` 15 · `-lg` 17 · `-xl` 20 (row titles) · `-2xl` 22 (section titles) · `-3xl` 28 (readouts) · `-4xl` 44 (page titles). Headings are weight 800; the `.eyebrow` class is the small uppercase mono label.

## Motion

One curve, `--ease` (`cubic-bezier(0.2, 0, 0, 1)`: fast out, settles). No spring, no overshoot, no hover lift, no float, no pulse.

| Token | Value | For |
|---|---|---|
| `--instant` | 0ms | Numerals and readouts: they change at once |
| `--fast` | 100ms | Hover, press, switch, chip, a toast leaving |
| `--base` | 160ms | Fills (a hotkey moving a slider), a toast or section arriving |
| `--slow` | 260ms | A dialog arriving |
| `--hatch` | 600ms | One step of the busy bar |
| `--blink` | 1s | The Simulated LED, in steps |
| `--undo-window` | 5s | The timer line under a toast with Undo (matches `store.svelte.ts`) |

Two decorative loops exist and no others: the **hatch** bar (`.hatch`, ticks marching: anything waiting on the monitor) and the **LED** (`.led.blink`: only while the monitor is simulated; Connected is the same square, solid). `prefers-reduced-motion` turns all of it off: `app.css` shortens CSS animations and transitions, and every `motion.ts` preset returns 0.

## Motion presets (`lib/motion.ts`)

Never write durations in a transition; pick a preset. Each reads its duration token when the transition starts, so retuning `app.css` retunes these too.

| Preset | Use with | When |
|---|---|---|
| `rise` | `in:fly` | A section, row or page block arriving. `Section` and `Item` do this for you. |
| `hint` | `in:fly` | A small inline note dropping in beside a heading (the loading `Hint`). |
| `pageIn` / `pageOut` | `in:fly` / `out:fade` | Switching pages: the new one rises, the old one leaves at once. |
| `popover` | `in:fly` | The tray popover opening. |
| `toastIn` / `toastOut` / `toastReflow` | `in:fly` / `out:fly` / `animate:flip` | Toasts arriving, leaving and re-stacking. |
| `dialogIn` | `in:fly` | A dialog over its scrim. |
| `fadeQuick` | `transition:fade` | Scrims and dots appearing and disappearing. |

Need a duration in script (a timeout that matches an animation)? Use `ms("fast")` from `motion.ts`.

## Components

- **`Section`**: a titled block of a page. `title`, `hint` (plain small print) or `aside` (a snippet: a `Lock`, a loading `Hint`), `loading` (dims the body and runs the busy bar under the title while values are re-read).
- **`Item`**: a settings row. `title`, `body` (a string or a snippet), `lead` (heavy rule, for the first row), `mark` (a snippet after the title: the Labs flask), the control as children, and `below` for full-width content such as the shortcut list.
- **`Toggle`**: the ON/OFF key. `checked`, `pending` (shows "…" and the busy bar), `disabled`, `label`, `onchange`.
- **`Slider`**: an outlined bar with a mono readout. `fine` puts − and + at the ends (±1 per press, hold to repeat; a run of presses counts as one change, so Undo goes back to where it started). `compact` is the tray's size. The readout inverts while you drag. The cursor over the bar is the pointer.
- **`Segmented`**: a choice among a few named values; the selected one is an ink fill.
- **`ModePicker`**: the picture mode. `layout="chips"` shows every mode at once, grouped by use (Everyday, Gaming, Color work, Accessibility; see `modes.ts`). `layout="stepper"` is `‹ name ›` with a position counter and ticks, and writes only the mode you stop on (400ms), so passing over modes never reaches the monitor. Both go through `pickPictureMode`, so the AI-feature interlock still asks first.
- **`Lock`**: why a control can't be used. Labs controls get the flask and `LABS`; anything else gets the padlock and a dashed `LOCKED` tag. Click for the reason.
- **`Hint`**, **`Logo`**, **`MonitorArt`** (the dashed "no signal" drawing for the empty state).

Everything is also in the design kit (`/#/kit`) in every state.

## Adding a feature

1. Put it in the page's grid or stack. For a titled block of controls use `Section`:

   ```svelte
   <Section title="Color temperature" hint="Warmer at night">
     <Slider control={byId["color-temperature"]} fine />
   </Section>
   ```

2. For a settings-style row (words on the left, one control on the right), use `Item`:

   ```svelte
   <Item title="Night shift" body="Warmer colors after sunset.">
     <Toggle checked={on} label="Night shift" onchange={setNightShift} />
   </Item>
   ```

3. If the section waits on the monitor, pass `loading` and show a `<Hint loading>` in its `aside`. If a control can't be used, show a `Lock` with the reason instead of hiding it.
4. Style only what is new, with tokens. If you need a value the scale doesn't have, add a token to `app.css` (and to this page) rather than a one-off number. Run `npm run lint:tokens`.
5. Check it in the design kit and in the app (`cd apps/desktop && npm run dev`) in light and dark, and with reduced motion on. A picture mode the app doesn't know yet lands in an "Other" group until it is added to `modes.ts`.

## Logo and icons

The logo is the wordmark: "nubless" in Archivo 800 with a lime cursor block after it (the `Logo` component, sized by one number). The app icon (`src-tauri/icons`, `public/logo.svg`, `docs/assets/logo.svg`) is the cursor block on an ink tile. To change the icon, edit `public/logo.svg`, copy it to `docs/assets/logo.svg`, and run `npx tauri icon public/logo.svg` (keep the files already in `src-tauri/icons`).

## README screenshots

`npm run screenshots` (in `apps/desktop`) rebuilds `docs/assets/window-*.png` and `tray-*.png` from the demo monitor. It needs a Chromium: Playwright's (`npx playwright install chromium`) or any Chrome via `CHROME_PATH`. `?preview=empty` on the dev URL shows the no-monitor state.
