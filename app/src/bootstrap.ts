// ── Bootstrap ─────────────────────────────────────────────────────────────────
// One-time app startup: subscribes to Tauri events, loads initial state,
// wires the render ↔ events cycle, then calls render() for the first time.

import { invoke } from '@tauri-apps/api/core';
import { emit, listen } from '@tauri-apps/api/event';
import { ctx } from './ctx';
import { cloneState, ensureProfileSelection } from './utils';
import { render, setBindEventsCallback } from './render';
import { bindEvents } from './events';
import { wireModalCallbacks, openOverlays, handleTimerTick, toggleOverlayEditMode } from './actions';
import type { BootstrapPayload, TimerTickPayload } from './types';

export async function bootstrapApp(): Promise<void> {
  // Wire dependency-injection slots before the first render.
  setBindEventsCallback(bindEvents);
  wireModalCallbacks();

  // Subscribe to Rust events before loading state so no events are missed.
  await listen<TimerTickPayload>('timer_tick', event => {
    handleTimerTick(event.payload);
  });

  await listen<Record<string, [number, number]>>('overlay-positions-saved', event => {
    const profile = ctx.state.profiles[ctx.state.activeProfile];
    if (profile) {
      for (const [idx, pos] of Object.entries(event.payload)) {
        profile.positions[idx] = pos;
      }
      void invoke('save_app_state', { payload: ctx.state }).catch(() => {});
    }
  });

  await listen<string>('hotkey_listener_error', event => {
    ctx.listenerError = event.payload;
    render();
  });

  // Load state + icons + platform info from Rust.
  ctx.bootstrap = await invoke<BootstrapPayload>('get_bootstrap_payload');
  ctx.state     = cloneState(ctx.bootstrap.state);
  ctx.selectedProfile = ctx.state.activeProfile;
  ctx.bootstrap.icons.forEach(asset => ctx.iconMap.set(asset.fileName, asset));
  ensureProfileSelection();
  ctx.showHotkeyLabels = ctx.state.profiles[ctx.selectedProfile]?.showHotkeyLabels ?? true;

  // Register hotkeys with the Rust backend.
  await invoke<number>('update_hotkey_registrations', { statePayload: ctx.state });
  await invoke('update_global_hotkeys', { statePayload: ctx.state });

  // Listen for global hotkey actions fired from the Rust hotkey thread.
  await listen<string>('hotkey_action', event => {
    if (event.payload === 'hide_show') {
      ctx.showHotkeyLabels = !ctx.showHotkeyLabels;
      ctx.state.profiles[ctx.selectedProfile].showHotkeyLabels = ctx.showHotkeyLabels;
      void emit('hotkey-label-changed', ctx.showHotkeyLabels);
      render();
    } else if (event.payload === 'layout_edit') {
      if (ctx.overlaysOpen) { void toggleOverlayEditMode(); }
    }
  });

  // Apply platform class so CSS can target platform-specific layout tweaks
  // (e.g. macOS traffic-light button clearance in the titlebar).
  if (ctx.bootstrap.platform.kind === 'macos') {
    document.body.classList.add('platform-macos');
  }

  render();

  if (ctx.state.autoShowOverlays) {
    await openOverlays();
  }
}
