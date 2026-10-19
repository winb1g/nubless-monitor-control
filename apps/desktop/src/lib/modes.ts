// How picture modes are named and grouped in the UI. The monitor's profile
// supplies the raw labels ("Calibration1"); this file decides what they are
// called on screen and which group they sit in.

export const MODE_GROUPS = ["Everyday", "Gaming", "Color work", "Accessibility"] as const;

const GROUP_OF: Record<string, number> = {
  "Personalized Picture": 0,
  "Gamer 1": 1,
  "Gamer 2": 1,
  FPS: 1,
  Calibration1: 2,
  Calibration2: 2,
  "DCI-P3": 2,
  "Color Weakness": 3,
};

/** The name shown on screen: "Calibration1" reads "Calibration 1". */
export const modeName = (label: string) => label.replace(/^(Calibration)(\d)$/, "$1 $2");

/** The group a mode sits in; a mode we don't know yet lands in "Other". */
export const groupName = (label: string): string => MODE_GROUPS[GROUP_OF[label]] ?? "Other";

/** Choices split into their groups, in the order above, empty groups left out. */
export function groupModes<T extends { label: string }>(choices: T[]): { name: string; items: T[] }[] {
  return [...MODE_GROUPS, "Other"]
    .map((name) => ({ name, items: choices.filter((c) => groupName(c.label) === name) }))
    .filter((g) => g.items.length > 0);
}
