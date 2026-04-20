// ── Pure utilities ────────────────────────────────────────────────────────────
// No side-effects, no render() calls.  Safe to import from any module.

import { convertFileSrc, isTauri } from '@tauri-apps/api/core';
import { ctx } from './ctx';
import type { AppState, IconAsset, Profile, PlatformKind, TimerConfig } from './types';

// ── State helpers ─────────────────────────────────────────────────────────────

export function cloneState(s: AppState): AppState {
  return JSON.parse(JSON.stringify(s)) as AppState;
}

export function activeProfile(): Profile {
  return ctx.state.profiles[ctx.selectedProfile];
}

export function ensureProfileSelection(): void {
  if (!ctx.selectedProfile || !ctx.state.profiles[ctx.selectedProfile]) {
    ctx.selectedProfile = ctx.state.activeProfile;
  }
  if (!ctx.selectedProfile || !ctx.state.profiles[ctx.selectedProfile]) {
    const first = Object.keys(ctx.state.profiles)[0];
    ctx.selectedProfile = first;
    ctx.state.activeProfile = first;
  }
}

// ── Factory helpers ───────────────────────────────────────────────────────────

export function defaultTimer(iconName = ''): TimerConfig {
  return {
    enabled: true,
    hotkey: '',
    hotkey2: '',
    duration: 10,
    icon: iconName,
    size: 100,
    opacity: 100,
    blink: true,
    blinkThreshold: 5,
    blinkColor: '#ff5b5b',
  };
}

// ── Asset helpers ─────────────────────────────────────────────────────────────

export function resolveAssetUrl(asset: IconAsset): string | null {
  if (asset.assetUrl) return asset.assetUrl;
  if (isTauri()) return convertFileSrc(asset.filePath);
  return null;
}

// ── Platform helpers ──────────────────────────────────────────────────────────

export function platformLabel(kind: PlatformKind): string {
  switch (kind) {
    case 'macos':         return 'macOS arm64';
    case 'windows':       return 'Windows';
    case 'linux-x11':     return 'Linux X11';
    case 'linux-wayland': return 'Linux Wayland';
    default:              return 'Unknown';
  }
}

export function hotkeyPlaceholder(kind: PlatformKind): string {
  return kind === 'macos' ? 'cmd+1, f9…' : 'ctrl+1, f9…';
}

// ── String helpers ────────────────────────────────────────────────────────────

export function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

// ── Profile import helpers ────────────────────────────────────────────────────

export function resolveImportName(name: string, existing: Record<string, Profile>): string {
  if (!existing[name]) return name;
  let i = 1;
  while (existing[`${name}_${i}`]) i++;
  return `${name}_${i}`;
}
