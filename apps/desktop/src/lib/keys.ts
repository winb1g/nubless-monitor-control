// Key combinations as the backend stores them ("Ctrl+Alt+PageUp": modifiers,
// then a KeyboardEvent.code) and as people read them ("Ctrl", "Alt", "Page Up").

const MODIFIER_CODES = /^(Control|Shift|Alt|Meta|OS)(Left|Right)?$/;

/**
 * The combination a key press makes, or null while only modifiers are down.
 * A global shortcut needs Ctrl, Alt or the Windows/Command key, so it can't
 * steal ordinary typing; Shift alone isn't enough.
 */
export function comboFrom(e: KeyboardEvent): { keys: string } | { error: string } | null {
  if (MODIFIER_CODES.test(e.code) || !e.code) return null;
  const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean);
  if (!e.ctrlKey && !e.altKey && !e.metaKey) {
    return { error: "Add Ctrl, Alt or the Windows key, so the shortcut doesn't get in the way of typing." };
  }
  return { keys: [...mods, e.code].join("+") };
}

const NAMES: Record<string, string> = {
  Super: navigator.platform.startsWith("Mac") ? "⌘" : "Win",
  PageUp: "Page Up",
  PageDown: "Page Down",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
};

/** "Ctrl+Alt+KeyM" -> ["Ctrl", "Alt", "M"], for showing as key caps. */
export function keyCaps(keys: string): string[] {
  return keys
    .split("+")
    .filter(Boolean)
    .map((k) => NAMES[k] ?? k.replace(/^Key(?=[A-Z]$)/, "").replace(/^Digit(?=\d$)/, "").replace(/^Numpad/, "Num "));
}
