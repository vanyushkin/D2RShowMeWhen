// ── Actions ───────────────────────────────────────────────────────────────────
// State mutations, Tauri IPC wrappers, timer event handling, and profile/config
// management.  Every function here either mutates ctx.* or invokes Rust commands
// (or both).  All callers of render() live here or in events.ts.

import { invoke } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';
import { ctx } from './ctx';
import { render, setSaveMsg, updateTimerRowDisplay, setModalCallbacks } from './render';
import { activeProfile, defaultTimer, ensureProfileSelection, resolveImportName } from './utils';
import { t, tf } from './i18n';
import type { AppState, Profile, SaveResult, TimerConfig, TimerTickPayload } from './types';

// ── Wire modal keyboard callbacks into render.ts ──────────────────────────────
// Called once from bootstrap so render.ts can call modalConfirm/Cancel on
// Enter/Escape without importing actions.ts directly.

export function wireModalCallbacks(): void {
  setModalCallbacks(modalConfirm, modalCancel);
}

// ── Timer mutations ───────────────────────────────────────────────────────────

/** Mutate a timer and trigger a full re-render. */
export function updateTimer(index: number, patch: Partial<TimerConfig>): void {
  activeProfile().timers[index] = { ...activeProfile().timers[index], ...patch };
  render();
}

/** Mutate a timer WITHOUT re-rendering (keeps focus in number/color inputs). */
export function patchTimer(index: number, patch: Partial<TimerConfig>): void {
  activeProfile().timers[index] = { ...activeProfile().timers[index], ...patch };
}

export function addTimer(): void {
  const profile = activeProfile();
  const iconName = ctx.bootstrap.icons[profile.timers.length % Math.max(ctx.bootstrap.icons.length, 1)]?.fileName ?? '';
  profile.timers.push(defaultTimer(iconName));
  render();
}

export function removeTimer(index: number): void {
  const profile = activeProfile();
  profile.timers.splice(index, 1);
  if (!profile.timers.length) {
    profile.timers.push(defaultTimer(ctx.bootstrap.icons[0]?.fileName ?? ''));
  }
  if (ctx.expandedTimerIndex === index) ctx.expandedTimerIndex = null;
  ctx.runningTimers.delete(index);
  render();
}

// ── Profile CRUD ──────────────────────────────────────────────────────────────

export function createProfile(): void {
  ctx.modalKind = 'new-profile';
  ctx.modalInputValue = '';
  render();
}

export function cloneProfile(): void {
  ctx.modalKind = 'clone-profile';
  ctx.modalInputValue = ctx.selectedProfile + ' Copy';
  render();
}

export function renameProfile(): void {
  ctx.modalKind = 'rename-profile';
  ctx.modalInputValue = ctx.selectedProfile;
  render();
}

export function deleteProfile(): void {
  if (Object.keys(ctx.state.profiles).length <= 1) {
    setSaveMsg(t('msgAtLeastOne'), 'error');
    return;
  }
  ctx.modalKind = 'confirm-delete-profile';
  render();
}

export function modalConfirm(): void {
  const { modalKind } = ctx;

  if (modalKind === 'new-profile') {
    const name = ctx.modalInputValue.trim();
    ctx.modalKind = 'none';
    if (!name) { render(); return; }
    if (ctx.state.profiles[name]) { setSaveMsg(tf('msgProfileExists', { name }), 'error'); render(); return; }
    ctx.state.profiles[name] = {
      timers: [defaultTimer(ctx.bootstrap.icons[0]?.fileName ?? '')],
      positions: {},
      showHotkeyLabels: true,
    };
    ctx.selectedProfile = name;
    ctx.state.activeProfile = name;

  } else if (modalKind === 'clone-profile') {
    const name = ctx.modalInputValue.trim();
    ctx.modalKind = 'none';
    if (!name) { render(); return; }
    if (ctx.state.profiles[name]) { setSaveMsg(tf('msgProfileExists', { name }), 'error'); render(); return; }
    ctx.state.profiles[name] = {
      timers: activeProfile().timers.map(t => ({ ...t })),
      positions: {},
      showHotkeyLabels: activeProfile().showHotkeyLabels ?? true,
    };
    ctx.selectedProfile = name;
    ctx.state.activeProfile = name;

  } else if (modalKind === 'rename-profile') {
    const name = ctx.modalInputValue.trim();
    ctx.modalKind = 'none';
    if (!name || name === ctx.selectedProfile) { render(); return; }
    if (ctx.state.profiles[name]) { setSaveMsg(tf('msgProfileExists', { name }), 'error'); render(); return; }
    ctx.state.profiles[name] = activeProfile();
    delete ctx.state.profiles[ctx.selectedProfile];
    ctx.selectedProfile = name;
    ctx.state.activeProfile = name;

  } else if (modalKind === 'confirm-delete-profile') {
    ctx.modalKind = 'none';
    delete ctx.state.profiles[ctx.selectedProfile];
    const first = Object.keys(ctx.state.profiles)[0];
    ctx.selectedProfile = first;
    ctx.state.activeProfile = first;

  } else if (modalKind === 'import-config') {
    const raw = ctx.modalInputValue.trim();
    ctx.modalKind = 'none';
    if (!raw) { render(); return; }
    try {
      const imported = decodeProfiles(raw);
      let added = 0;
      for (const [name, profile] of Object.entries(imported)) {
        const safeName = resolveImportName(name, ctx.state.profiles);
        ctx.state.profiles[safeName] = profile;
        added++;
      }
      if (added > 0) { setSaveMsg(tf('msgImported', { count: added }), 'success'); }
      else           { render(); }
    } catch {
      setSaveMsg(t('msgImportFailed'), 'error');
    }
    return;
  }

  render();
}

export function modalCancel(): void {
  ctx.modalKind = 'none';
  render();
}

// ── Config export / import ────────────────────────────────────────────────────

export function encodeProfiles(profiles: Record<string, Profile>): string {
  const json = JSON.stringify(profiles);
  return btoa(encodeURIComponent(json).replace(/%([0-9A-F]{2})/g, (_, p) => String.fromCharCode(parseInt(p, 16))));
}

export function decodeProfiles(b64: string): Record<string, Profile> {
  const json = decodeURIComponent(
    Array.from(atob(b64.trim()))
      .map(c => '%' + c.charCodeAt(0).toString(16).padStart(2, '0'))
      .join('')
  );
  return JSON.parse(json) as Record<string, Profile>;
}

export function exportConfig(): void {
  ctx.modalKind = 'export-choice';
  render();
}

export function importConfig(): void {
  ctx.modalInputValue = '';
  ctx.modalKind = 'import-config';
  render();
}

// ── Tauri commands ────────────────────────────────────────────────────────────

export async function saveState(): Promise<void> {
  ensureProfileSelection();
  if (ctx.overlayEditMode) {
    ctx.overlayEditMode = false;
    await invoke('set_overlays_edit_mode', { enabled: false }).catch(() => {});
  }
  ctx.state.activeProfile = ctx.selectedProfile;
  try {
    const result = await invoke<SaveResult>('save_app_state', { payload: ctx.state });
    await invoke<number>('update_hotkey_registrations', { statePayload: ctx.state });
    await invoke('update_global_hotkeys', { statePayload: ctx.state });
    if (ctx.overlaysOpen) {
      await invoke('open_overlays', { statePayload: ctx.state });
    }
    setSaveMsg(`✓ ${result.storagePath}`, 'success');
  } catch (err) {
    setSaveMsg(`${t('msgSaveError')} ${String(err)}`, 'error');
  }
}

export async function openOverlays(options: { enterEditMode?: boolean } = {}): Promise<void> {
  try {
    const count    = await invoke<number>('open_overlays', { statePayload: ctx.state });
    const regCount = await invoke<number>('update_hotkey_registrations', { statePayload: ctx.state });
    ctx.overlaysOpen = true;
    ctx.overlayEditMode = false;
    if (!ctx.watching) {
      await invoke('set_watching', { enabled: true });
      ctx.watching = true;
    }
    if (options.enterEditMode) {
      await invoke('set_overlays_edit_mode', { enabled: true });
      ctx.overlayEditMode = true;
    }
    setSaveMsg(tf('msgOverlaysShown', { count, regs: regCount }), 'success');
    render();
  } catch (err) {
    setSaveMsg(`${t('msgOverlaysError')} ${String(err)}`, 'error');
  }
}

export async function closeOverlays(): Promise<void> {
  try {
    if (ctx.watching) {
      await invoke('set_watching', { enabled: false });
      ctx.watching = false;
    }
    await invoke('close_overlays');
    ctx.overlaysOpen = false;
    ctx.overlayEditMode = false;
    render();
  } catch (err) {
    setSaveMsg(`${t('msgOverlaysError')} ${String(err)}`, 'error');
  }
}

export async function startWatch(): Promise<void> {
  try {
    await invoke('set_watching', { enabled: true });
    ctx.watching = true;
    render();
  } catch (err) {
    setSaveMsg(`${t('msgWatchError')} ${String(err)}`, 'error');
  }
}

export async function stopWatch(): Promise<void> {
  try {
    await invoke('set_watching', { enabled: false });
    ctx.watching = false;
    render();
  } catch (err) {
    setSaveMsg(`${t('msgWatchError')} ${String(err)}`, 'error');
  }
}

export async function resetOverlayLayout(): Promise<void> {
  activeProfile().positions = {};
  try {
    if (ctx.overlaysOpen) {
      await invoke('open_overlays', { statePayload: ctx.state });
    }
    setSaveMsg(t('msgResetDone'), 'success');
  } catch (err) {
    setSaveMsg(`${t('msgResetError')} ${String(err)}`, 'error');
  }
}

export async function toggleOverlayEditMode(): Promise<void> {
  ctx.overlayEditMode = !ctx.overlayEditMode;
  try {
    await invoke('set_overlays_edit_mode', { enabled: ctx.overlayEditMode });
    render();
  } catch (err) {
    setSaveMsg(`${t('msgEditModeError')} ${String(err)}`, 'error');
  }
}

// ── Timer event handling ──────────────────────────────────────────────────────

export function handleTimerTick(payload: TimerTickPayload): void {
  if (payload.finished) {
    ctx.runningTimers.delete(payload.timerIndex);
  } else {
    ctx.runningTimers.set(payload.timerIndex, {
      remainingSecs: payload.remainingSecs,
      totalSecs: payload.totalSecs,
      blinking: payload.blinking,
    });
  }
  updateTimerRowDisplay(payload.timerIndex);
}
