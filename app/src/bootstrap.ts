// ── Bootstrap ─────────────────────────────────────────────────────────────────
// One-time app startup: subscribes to Tauri events, loads initial state,
// wires the render ↔ events cycle, then calls render() for the first time.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { LogicalSize } from '@tauri-apps/api/dpi';
import { ctx } from './ctx';
import { cloneState, ensureProfileSelection } from './utils';
import { render, setBindEventsCallback } from './render';
import { bindEvents } from './events';
import { setLang } from './i18n';
import { wireModalCallbacks, openOverlays, handleTimerTick, toggleOverlayEditMode, applyUiScale, autoSave } from './actions';
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
      void invoke('update_overlay_hotkey_labels', { show: ctx.showHotkeyLabels });
      render();
    } else if (event.payload === 'layout_edit') {
      if (ctx.overlaysOpen) { void toggleOverlayEditMode(); }
    }
  });

  // Apply platform class so CSS can target platform-specific layout tweaks
  // (e.g. macOS traffic-light button clearance in the titlebar,
  //  Windows custom close/minimize controls).
  if (ctx.bootstrap.platform.kind === 'macos') {
    document.body.classList.add('platform-macos');
  } else if (ctx.bootstrap.platform.kind === 'windows') {
    document.body.classList.add('platform-windows');
  }

  // ── Restore language preference ───────────────────────────────────────────
  // Primary source: ctx.state.lang (persisted in profiles.json).
  // Fallback: localStorage (migration path for installs predating this field).
  // If neither is set, default to 'en'.
  // After resolving, write back to state so subsequent saves persist it.
  {
    const fromState   = ctx.state.lang;
    const fromStorage = localStorage.getItem('lang') as 'ru' | 'en' | null;
    const resolved    = (fromState || fromStorage || 'en') as 'ru' | 'en';
    setLang(resolved);          // also updates localStorage cache
    if (!fromState) {
      ctx.state.lang = resolved;
      void autoSave();
    }
  }

  // ── Restore UI scale preference ───────────────────────────────────────────
  // Primary source: ctx.state.uiScale (persisted in profiles.json).
  // Fallback: localStorage (migration path for installs predating this field).
  // Zero in state means "not yet set".
  // Window dimensions are restored separately so a manually-resized window
  // is not clobbered by the scale-derived defaults.
  {
    const fromState   = ctx.state.uiScale;
    const fromStorage = parseInt(localStorage.getItem('uiScale') ?? '0', 10);
    const resolved    = fromState || fromStorage || 100;
    ctx.uiScale = resolved;
    if (!fromState) {
      ctx.state.uiScale = resolved;
      void autoSave();
    }
    // Apply CSS zoom (no window resize yet — size is handled below).
    if (resolved !== 100) {
      document.documentElement.style.zoom = String(resolved / 100);
    }
    // Restore saved window dimensions; fall back to scale-derived defaults.
    // BASE_W/H must match tauri.conf.json window[0] width/height and actions.ts.
    const BASE_W = 720;
    const BASE_H = 580;
    const savedW = ctx.state.windowWidth;
    const savedH = ctx.state.windowHeight;
    if (savedW > 0 && savedH > 0) {
      void getCurrentWindow().setSize(new LogicalSize(savedW, savedH));
    } else if (resolved !== 100) {
      void getCurrentWindow().setSize(new LogicalSize(
        Math.round(BASE_W * resolved / 100),
        Math.round(BASE_H * resolved / 100),
      ));
    }
  }

  render();

  // ── Track manual window resizes ───────────────────────────────────────────
  // Saves current logical dimensions 500 ms after the last resize event so
  // rapid resize gestures don't spam the disk.
  {
    let resizeTimer: ReturnType<typeof setTimeout> | null = null;
    window.addEventListener('resize', () => {
      if (resizeTimer) clearTimeout(resizeTimer);
      resizeTimer = setTimeout(async () => {
        try {
          const physSize = await getCurrentWindow().innerSize();
          const factor   = await getCurrentWindow().scaleFactor();
          ctx.state.windowWidth  = Math.round(physSize.width  / factor);
          ctx.state.windowHeight = Math.round(physSize.height / factor);
          void autoSave();
        } catch { /* ignore if window is closing */ }
      }, 500);
    });
  }

  if (ctx.state.autoShowOverlays) {
    await openOverlays();
  }
}
