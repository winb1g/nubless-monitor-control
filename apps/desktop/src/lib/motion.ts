// Motion presets for Svelte transitions (in:, out:, animate:). CSS motion
// lives in app.css (--fast, --base, --slow, --spin, --ease, --spring); the
// durations here mirror those tokens, so change both together.
//
// Use a preset instead of inline numbers: `in:fly={rise(2)}`, not
// `in:fly={{ y: 14, duration: 320, delay: 80 }}`.

import { backOut, cubicOut } from "svelte/easing";

export const DURATION = { fast: 140, base: 240, slow: 380 } as const;

/** Delay between items that arrive together, such as a page's cards. */
const STAGGER = 40;

/** A card or section rising into place; `i` staggers it after its siblings. */
export const rise = (i = 0) => ({ y: 14, duration: 320, delay: i * STAGGER, easing: cubicOut });

/** A small inline hint dropping in beside a heading. */
export const hint = { y: -4, duration: 200 };

/** A page appearing, and leaving fast so the next one isn't kept waiting. */
export const pageIn = { y: 10, duration: 280, delay: 60 };
export const pageOut = { duration: 60 };

/** The tray popover opening. */
export const popover = { y: 10, duration: 220 };

/** Toasts: rise in, settle out, and reflow the stack. */
export const toastIn = { y: 16, duration: 260 };
export const toastOut = { y: 8, duration: 160 };
export const toastReflow = { duration: 220 };

/** Something that pops into place with a little overshoot: dialogs, status badges. */
export const pop = { start: 0.92, duration: 260, easing: backOut };

/** A scrim or dot fading in and out. */
export const fadeQuick = { duration: 160 };
