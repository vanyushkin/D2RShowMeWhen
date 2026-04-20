// ── Hotkey serialisation helpers ──────────────────────────────────────────────
// Pure functions — no imports from our codebase, safe to use anywhere.

export function isModifierOnly(key: string): boolean {
  return ['Meta', 'Control', 'Alt', 'Shift'].includes(key);
}

export function normalizeKey(key: string): string {
  const map: Record<string, string> = {
    ' ': 'space', ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left',
    ArrowRight: 'right', Escape: 'esc', Enter: 'enter', Tab: 'tab',
    Backspace: 'backspace', Delete: 'delete', Home: 'home', End: 'end',
    PageUp: 'pageup', PageDown: 'pagedown',
  };
  if (map[key]) return map[key];
  if (/^F\d{1,2}$/.test(key)) return key.toLowerCase();
  return key.toLowerCase();
}

/** Converts a KeyboardEvent into a hotkey string like "ctrl+shift+f3".
 *  Returns null if the event is a bare modifier press. */
export function serializeHotkey(ev: KeyboardEvent): string | null {
  const mods: string[] = [];
  if (ev.metaKey)  mods.push('cmd');
  if (ev.ctrlKey)  mods.push('ctrl');
  if (ev.altKey)   mods.push('alt');
  if (ev.shiftKey) mods.push('shift');
  if (isModifierOnly(ev.key)) return null;
  return [...mods, normalizeKey(ev.key)].join('+');
}
