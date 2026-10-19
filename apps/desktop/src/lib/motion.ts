// Motion presets for Svelte transitions (in:, out:, animate:). Durations are
// not numbers here: each preset reads a CSS token from app.css (--instant,
// --fast, --base, --slow) when its transition starts, so app.css is the one
// place to tune timing. When the user asks for reduced motion every preset
// runs instantly, matching what app.css does for CSS transitions.
//
// Use a preset instead of inline numbers: `in:fly={rise(2)}`, not
// `in:fly={{ y: 14, duration: 320, delay: 80 }}`.

import { backOut, cubicOut } from "svelte/easing";

export type Speed = "instant" | "fast" | "base" | "slow";

const reduced = typeof matchMedia === "function" ? matchMedia("(prefers-reduced-motion: reduce)") : null;
const cache = new Map<Speed, number>();

/** A duration token from app.css in milliseconds; 0 under reduced motion. */
export function ms(speed: Speed): number {
  if (reduced?.matches) return 0;
  let v = cache.get(speed);
  if (v === undefined) {
    const raw = getComputedStyle(document.documentElement).getPropertyValue(`--${speed}`).trim();
    v = raw.endsWith("ms") ? parseFloat(raw) : parseFloat(raw) * 1000;
    if (Number.isNaN(v)) {
      console.warn(`motion: --${speed} is not a duration in app.css`);
      v = 0;
    }
    cache.set(speed, v);
  }
  return v;
}

/** Delay between items that arrive together, such as a page's cards. */
const STAGGER = 40;

/** A card or section rising into place; `i` staggers it after its siblings. */
export const rise = (i = 0) => ({ y: 14, duration: ms("slow"), delay: reduced?.matches ? 0 : i * STAGGER, easing: cubicOut });

// The presets below are objects with getters, so the duration is read each
// time a transition starts rather than once when this module loads.

/** A small inline hint dropping in beside a heading. */
export const hint = { y: -4, get duration() { return ms("base"); } };

/** A page appearing, and leaving fast so the next one isn't kept waiting. */
export const pageIn = { y: 10, get duration() { return ms("base"); }, get delay() { return ms("instant"); } };
export const pageOut = { get duration() { return ms("instant"); } };

/** The tray popover opening. */
export const popover = { y: 10, get duration() { return ms("base"); } };

/** Toasts: rise in, settle out, and reflow the stack. */
export const toastIn = { y: 16, get duration() { return ms("base"); } };
export const toastOut = { y: 8, get duration() { return ms("fast"); } };
export const toastReflow = { get duration() { return ms("base"); } };

/** Something that pops into place with a little overshoot: dialogs, status badges. */
export const pop = { start: 0.92, easing: backOut, get duration() { return ms("base"); } };

/** A scrim or dot fading in and out. */
export const fadeQuick = { get duration() { return ms("fast"); } };
