// Motion presets for Svelte transitions (in:, out:, animate:). Durations are
// not numbers here: each preset reads a CSS token from app.css (--instant,
// --fast, --base, --slow) when its transition starts, so app.css is the one
// place to tune timing. When the user asks for reduced motion every preset
// runs instantly, matching what app.css does for CSS transitions.
//
// The motion is mechanical: one curve, short rises, no overshoot, and nothing
// leaves slowly. Use a preset instead of inline numbers: `in:fly={rise}`, not
// `in:fly={{ y: 8, duration: 160 }}`.

import { cubicOut } from "svelte/easing";

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

// The presets below are objects with getters, so the duration is read each
// time a transition starts rather than once when this module loads.

/** A section or row arriving: rises 8px. */
export const rise = { y: 8, easing: cubicOut, get duration() { return ms("base"); } };

/** A small inline hint dropping in beside a heading. */
export const hint = { y: -4, easing: cubicOut, get duration() { return ms("fast"); } };

/** A page appearing. It rises in; the old page leaves at once (no exit). */
export const pageIn = { y: 8, easing: cubicOut, get duration() { return ms("base"); } };
export const pageOut = { get duration() { return ms("instant"); } };

/** The tray popover opening. */
export const popover = { y: 8, easing: cubicOut, get duration() { return ms("base"); } };

/** Toasts: rise in (160ms), leave quickly (100ms), and reflow the stack. */
export const toastIn = { y: 8, easing: cubicOut, get duration() { return ms("base"); } };
export const toastOut = { y: 4, easing: cubicOut, get duration() { return ms("fast"); } };
export const toastReflow = { get duration() { return ms("fast"); } };

/** A dialog arriving over its scrim. */
export const dialogIn = { y: 8, easing: cubicOut, get duration() { return ms("slow"); } };

/** A scrim or dot fading in and out. */
export const fadeQuick = { get duration() { return ms("fast"); } };
