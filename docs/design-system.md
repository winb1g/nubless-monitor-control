# Design system (desktop app)

The desktop UI (`apps/desktop`) has one source of truth for how things look and move:

| What | Where |
|---|---|
| Tokens: color, shadow, radius, spacing, type, motion | `src/app.css` (`:root`, plus two dark-theme blocks) |
| Shared classes: `.page`, `.subtitle`, `.card`, `.btn`, `.muted`, `.faint`, `.small`, `.skeleton`, `.busy-ring` | `src/app.css` |
| Svelte transition presets | `src/lib/motion.ts` (durations read from `app.css`) |
| Building blocks: `Card`, `IconBadge`, `Hint`, `Toggle`, `Slider`, `Segmented`, `Lock` | `src/lib/components/` |

Rule of thumb: a component's `<style>` uses tokens for every color, shadow, radius, font size and duration. Spacing between things uses the `--space-*` scale; padding that shapes a single control (a chip's `3px 9px`) can stay with that control. Decorative loops (the monitor's screen sweep, the pulsing status dot) keep their own durations.

## Tokens

**Color.** Surfaces `--bg`, `--panel`, `--panel-2`; lines `--line`, `--line-strong`; text `--text`, `--text-2` (secondary), `--text-3` (faint). Brand: `--brand-1`/`--brand-2` make `--grad` (diagonal fills) and `--grad-x` (sideways fills such as the slider track); `--accent`, `--accent-ink` (accent text), `--accent-soft` (accent wash), `--on-accent` (text on the gradient). Status pairs: `--mint`/`--mint-soft`, `--sun`/`--sun-soft`, `--coral`/`--coral-soft`. Toasts sit on `--inverse` with `--inverse-*` colors in both themes. Also `--knob`, `--scrim`.

The dark theme redefines only what changes. Add a new color to `:root` and, if it should differ in dark mode, to **both** dark blocks (the media query and `[data-theme="dark"]`).

**Shadow.** `--shadow-sm` (hairline lift), `--shadow` (cards), `--shadow-lg` (dialogs, popovers), `--shadow-popover` (the tray), `--shadow-knob` (thumbs). Colored glow under gradient fills: `--glow-sm`, `--glow`, `--glow-lg`, all built from `--glow-rgb`.

**Radius.** `--r-xs` 8 · `--r-sm` 10 · `--r-md` 12 (buttons, nav) · `--r-lg` 14 (popovers, toasts, highlighted rows) · `--r-xl` 16 (banners, tiles) · `--r-2xl` 22 (cards, dialogs, tray) · `--r-full` (pills). Icon badges compute theirs from their size.

**Spacing.** A 4px grid: `--space-1` 4 · `-2` 8 · `-3` 12 · `-4` 16 · `-5` 20 · `-6` 24 · `-8` 32 · `-10` 40. `--page-padding` is the padding of every page; cards are `--space-5` inside and `--space-4` apart.

**Type.** `--fs-2xs` 11 · `--fs-xs` 12 (hints, tags, chips) · `--fs-sm` 12.5 (`.small`) · `--fs-md` 13 (compact labels) · `--fs-base` 14 (body) · `--fs-lg` 15 (`h2`) · `--fs-xl` 17 · `--fs-2xl` 20 · `--fs-3xl` 26 (`h1`) · `--fs-4xl` 32. Weights are used directly: 500 body emphasis, 550 controls, 600 titles, 650–700 headings.

**Motion.**

| Token | Value | For |
|---|---|---|
| `--instant` | 60ms | getting out of the way (a page leaving) |
| `--fast` | 140ms | hover, press, color, small fades |
| `--base` | 240ms | state changes, things arriving |
| `--slow` | 380ms | larger moves: sliding pills, cards rising |
| `--spin` | 0.8s | one turn of any busy indicator |
| `--ease` | | settles; the default |
| `--spring` | | overshoots a little; knobs, pills, press feedback |

`prefers-reduced-motion` turns all of it off: `app.css` shortens CSS animations and transitions, and every `motion.ts` preset returns a duration of 0.

## Motion presets (`lib/motion.ts`)

Never write durations in a transition; pick a preset. Each one reads its duration token when the transition starts, so retuning `app.css` retunes these too.

| Preset | Use with | When |
|---|---|---|
| `rise(i)` | `in:fly` | A card or section arriving with its siblings; `i` staggers it. `Card` does this for you. |
| `hint` | `in:fly` | A small inline note dropping in beside a heading (the loading `Hint` uses it). |
| `pageIn` / `pageOut` | `in:fly` / `out:fade` | Switching pages in the main window. |
| `popover` | `in:fly` | The tray popover opening. |
| `toastIn` / `toastOut` / `toastReflow` | `in:fly` / `out:fly` / `animate:flip` | Toasts arriving, leaving and re-stacking. |
| `pop` | `in:scale` / `transition:scale` | Something that pops into place: dialogs, a status badge changing. |
| `fadeQuick` | `transition:fade` | Scrims and dots appearing and disappearing. |

Need a duration in script (a timeout that matches an animation)? Use `ms("fast")` from `motion.ts`. For a spinner, use `.busy-ring` or `IconBadge`'s `busy`/`spin`; don't add another keyframe.

## Components

- **`Card`**: the panel every page is built from. `title`, `hint` (plain small print) or `aside` (a snippet: a `Lock`, a loading `Hint`), `i` (stagger), `row` (icon, text, control in a line), `tint` (accent wash for the one card that leads a page), `loading` (dims the body and runs a sheen while values are re-read).
- **`IconBadge`**: an icon on a rounded tile. `size`, `iconSize`, `tone` (`soft`, `raised`, `tint`, `brand` for on/selected, `neutral`, `mint`, `coral`), `round`, `glow`, `busy` (busy ring), `spin` (spin the icon). Corner radius and glow scale with `size`.
- **`Hint`**: small print beside a heading; `loading` turns it into the accent "Loading…" hint.
- **`Toggle`**, **`Slider`**, **`Segmented`**, **`Lock`**: controls, all with a pending or locked state.

## Adding a feature card

1. Put it in the page's grid or stack, after the cards already there, with the next `i`:

   ```svelte
   <Card title="Color temperature" hint="Warmer at night" i={6}>
     <Slider control={byId["color-temperature"]} />
   </Card>
   ```

2. For a settings-style card (icon, words, one control), use `row`. The text column is the page's own style (`.text` in `Settings.svelte`):

   ```svelte
   <Card row i={4}>
     <IconBadge icon={Moon} size={38} />
     <div class="text">
       <h2>Night shift</h2>
       <p class="muted">Warmer colors after sunset.</p>
     </div>
     <Toggle checked={on} label="Night shift" onchange={setNightShift} />
   </Card>
   ```

3. If the card waits on the monitor, pass `loading` and show a `<Hint loading>` in its `aside`. If a control can't be used, show a `Lock` with the reason instead of hiding it.
4. Style only what is new, with tokens. If you need a value the scale doesn't have, add a token to `app.css` (and to this page) rather than a one-off number.
5. Check it in the browser preview (`cd apps/desktop && npm run dev`) in light and dark, and with reduced motion on.
