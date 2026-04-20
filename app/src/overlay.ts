/**
 * Overlay window script.
 *
 * Each overlay-N window loads this, parses ?index=N from the URL,
 * fetches its config via invoke('get_overlay_config'), then drives
 * a canvas-based timer display.
 *
 * Timer state is updated via two mechanisms (belt-and-suspenders):
 *   1. listen('timer_tick') — low-latency event from Rust broadcast
 *   2. setInterval → invoke('get_active_timer_state') — polling fallback
 *      that catches any events missed during async init.
 *
 * Drag in edit mode is handled purely by data-tauri-drag-region on
 * #drag-region div — no JS startDragging() call needed.
 * Position saving is done from the Rust side in set_overlays_edit_mode.
 */

import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// ── Types ─────────────────────────────────────────────────────────────────────

type OverlayTimerConfig = {
  timerIndex: number;
  iconPath: string;
  duration: number;
  blinkThreshold: number;
  blinkColor: string;
  blink: boolean;
  size: number;
  opacity: number;
  hotkey: string;
  hotkey2: string;
  showHotkeyLabels: boolean;
  winSize: number;
};

type TimerTickPayload = {
  timerIndex: number;
  remainingSecs: number;
  totalSecs: number;
  blinking: boolean;
  finished: boolean;
};

// ── Init ──────────────────────────────────────────────────────────────────────

const params    = new URLSearchParams(window.location.search);
const timerIndex = parseInt(params.get('index') ?? '0', 10);

const canvas = document.getElementById('c') as HTMLCanvasElement;
const ctx    = canvas.getContext('2d')!;

// ── Mutable state ─────────────────────────────────────────────────────────────

let config:    OverlayTimerConfig | null = null;
let iconImage: HTMLImageElement   | null = null;

let remaining        = 0;
let total            = 1;
let editMode         = false;
let timerRunning     = false;
let blinkOn          = false;
let blinkTimer: ReturnType<typeof setInterval> | null = null;
let showHotkeyLabel  = true;

// ── Canvas helpers ────────────────────────────────────────────────────────────

function resizeCanvas(size: number): void {
  canvas.width         = size;
  canvas.height        = size;
  canvas.style.width   = size + 'px';
  canvas.style.height  = size + 'px';
}

function hexToRgba(hex: string, alpha: number): string {
  const m = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  if (!m) return `rgba(255,91,91,${alpha})`;
  return `rgba(${parseInt(m[1], 16)},${parseInt(m[2], 16)},${parseInt(m[3], 16)},${alpha})`;
}

// ── Draw ──────────────────────────────────────────────────────────────────────

function draw(): void {
  if (!config) return;

  const size     = config.winSize;
  const pad      = 8;
  const iconSize = size - 2 * pad;
  const cx       = size / 2;
  const cy       = size / 2;
  const r        = iconSize / 2;

  ctx.clearRect(0, 0, size, size);

  // 1. Icon clipped to circle
  ctx.save();
  ctx.beginPath();
  ctx.arc(cx, cy, r, 0, Math.PI * 2);
  ctx.clip();

  if (iconImage) {
    ctx.drawImage(iconImage, pad, pad, iconSize, iconSize);
  } else {
    ctx.fillStyle = '#1e3a6e';
    ctx.fillRect(pad, pad, iconSize, iconSize);
    ctx.fillStyle    = '#8ab4e8';
    ctx.font         = `bold ${Math.max(10, Math.floor(iconSize / 4))}px Arial`;
    ctx.textAlign    = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText('D2', cx, cy);
  }
  ctx.restore();

  // 2. White ring background
  ctx.beginPath();
  ctx.arc(cx, cy, r - 2, 0, Math.PI * 2);
  ctx.strokeStyle = 'rgba(255,255,255,0.14)';
  ctx.lineWidth   = 4;
  ctx.stroke();

  // 3. Progress arc — counterclockwise from top (matches original PyQt paintEvent)
  const progress  = total > 0 ? Math.max(0, Math.min(1, remaining / total)) : 1.0;
  const isNearEnd = remaining > 0 && remaining <= config.blinkThreshold;
  const arcColor  = isNearEnd ? config.blinkColor : '#3ce68f';

  if (progress > 0) {
    ctx.beginPath();
    ctx.arc(cx, cy, r - 2, -Math.PI / 2, -Math.PI / 2 - 2 * Math.PI * progress, true);
    ctx.strokeStyle = arcColor;
    ctx.lineWidth   = 4;
    ctx.lineCap     = 'round';
    ctx.stroke();
  }

  // 4. Blink flash
  if (blinkOn && config.blink && isNearEnd) {
    ctx.beginPath();
    ctx.arc(cx, cy, r, 0, Math.PI * 2);
    ctx.fillStyle = hexToRgba(config.blinkColor, 85 / 255);
    ctx.fill();
  }

  // 5. Hotkey label — shown when showHotkeyLabel is true (toggled via global hotkey).
  if (showHotkeyLabel || editMode) {
    const label = config.hotkey || config.hotkey2 || `T${config.timerIndex + 1}`;
    ctx.font         = `bold ${Math.max(7, Math.floor(size / 8))}px "Segoe UI", Arial, sans-serif`;
    ctx.textAlign    = 'center';
    ctx.textBaseline = 'bottom';
    ctx.shadowColor  = 'rgba(0,0,0,0.95)';
    ctx.shadowBlur   = 4;
    ctx.fillStyle    = editMode ? '#ffffff' : 'rgba(255,255,255,0.75)';
    ctx.fillText(label, cx, size - 2);
    ctx.shadowBlur   = 0;
  }

  // 6. Edit-mode dashed border indicator.
  if (editMode) {
    ctx.beginPath();
    ctx.arc(cx, cy, r + 2, 0, Math.PI * 2);
    ctx.strokeStyle = 'rgba(255,255,255,0.5)';
    ctx.lineWidth   = 1.5;
    ctx.setLineDash([4, 3]);
    ctx.stroke();
    ctx.setLineDash([]);
  }
}

// ── Blink management ──────────────────────────────────────────────────────────

function startBlink(): void {
  if (blinkTimer) return;
  blinkTimer = setInterval(() => { blinkOn = !blinkOn; draw(); }, 180);
}

function stopBlink(): void {
  if (blinkTimer) { clearInterval(blinkTimer); blinkTimer = null; }
  blinkOn = false;
}

// ── Timer state update ────────────────────────────────────────────────────────

function applyTimerState(p: TimerTickPayload): void {
  if (p.timerIndex !== timerIndex) return;

  if (p.finished) {
    remaining    = 0;
    total        = config?.duration ?? 1;
    timerRunning = false;
    stopBlink();
    draw();
    // Ask Rust to hide this overlay window (Rust is authoritative for window visibility).
    if (!editMode) void invoke('hide_overlay', { index: timerIndex });
  } else {
    remaining    = p.remainingSecs;
    total        = p.totalSecs;
    if (!timerRunning) {
      // First tick — ask Rust to show this window.
      timerRunning = true;
      void invoke('show_overlay', { index: timerIndex });
    }
    const nearEnd = remaining <= (config?.blinkThreshold ?? 5);
    if (config?.blink && nearEnd) { startBlink(); } else { stopBlink(); }
    draw();
  }
}

// ── Bootstrap ─────────────────────────────────────────────────────────────────

async function init(): Promise<void> {
  // 1. Fetch config from Rust
  config = await invoke<OverlayTimerConfig>('get_overlay_config', { index: timerIndex });
  // Initialise label visibility from the persisted per-profile value.
  showHotkeyLabel = config.showHotkeyLabels;

  // 2. Size canvas
  resizeCanvas(config.winSize);

  // 3. Load icon
  if (config.iconPath) {
    const url = convertFileSrc(config.iconPath);
    const img = new Image();
    await new Promise<void>(resolve => {
      img.onload  = () => resolve();
      img.onerror = () => resolve();
      img.src = url;
    });
    if (img.complete && img.naturalWidth > 0) iconImage = img;
  }

  // 4. Initial draw — full arc (ready state)
  remaining = config.duration;
  total     = config.duration;
  draw();

  // 5. CSS opacity on canvas
  canvas.style.opacity = String(config.opacity / 100);

  // 6. Event subscription (low-latency path)
  await listen<TimerTickPayload>('timer_tick', event => {
    applyTimerState(event.payload);
  });

  // 7. Polling fallback — catches any events missed during async init
  //    and keeps the display accurate even if IPC events are unreliable.
  setInterval(async () => {
    try {
      const state = await invoke<TimerTickPayload | null>('get_active_timer_state', { timerIndex });
      if (state) {
        applyTimerState(state);
      } else if (timerRunning) {
        // Timer finished/cancelled between ticks — reset and hide.
        timerRunning = false;
        remaining    = config?.duration ?? total;
        total        = config?.duration ?? total;
        stopBlink();
        draw();
        if (!editMode) void invoke('hide_overlay', { index: timerIndex });
      }
    } catch { /* ignore poll errors */ }
  }, 250);

  // 8. Hotkey label visibility toggle — fired from main window via emit().
  await listen<boolean>('hotkey-label-changed', event => {
    showHotkeyLabel = event.payload;
    draw();
  });

  // 9. Edit-mode changes from Rust (via set_overlays_edit_mode)
  // Rust already handles show/hide at the window level; this listener only
  // updates local JS state (blink behaviour, dashed border indicator).
  await listen<boolean>('edit-mode-changed', event => {
    editMode = event.payload;
    document.body.classList.toggle('edit-mode', editMode);
    if (!editMode) stopBlink();
    draw();
  });

  // 10. Config refresh — emitted by open_overlays when the window already exists
  //    (e.g. user saves settings while overlays are open). Re-fetch config,
  //    resize the canvas if the timer size changed, reload icon, redraw.
  await listen('config-updated', async () => {
    try {
      const next = await invoke<OverlayTimerConfig>('get_overlay_config', { index: timerIndex });

      const sizeChanged = next.winSize !== config?.winSize;
      showHotkeyLabel = next.showHotkeyLabels;
      config = next;

      if (sizeChanged) resizeCanvas(config.winSize);
      canvas.style.opacity = String(config.opacity / 100);

      // Reload icon if path changed
      if (config.iconPath) {
        const url = convertFileSrc(config.iconPath);
        const img = new Image();
        await new Promise<void>(resolve => {
          img.onload  = () => resolve();
          img.onerror = () => resolve();
          img.src = url;
        });
        iconImage = img.complete && img.naturalWidth > 0 ? img : null;
      } else {
        iconImage = null;
      }

      draw();
    } catch { /* ignore — window may be closing */ }
  });
}

// Draw an immediate placeholder so the window is visually present even before
// config loads. Replaced by the real draw once init() completes.
resizeCanvas(56);
ctx.fillStyle = '#1e3a6e';
ctx.beginPath();
ctx.arc(28, 28, 24, 0, Math.PI * 2);
ctx.fill();
ctx.fillStyle = '#8ab4e8';
ctx.font = 'bold 10px Arial';
ctx.textAlign = 'center';
ctx.textBaseline = 'middle';
ctx.fillText('...', 28, 28);

init().catch(err => {
  console.error('Overlay init failed:', err);
  // Visible error: red background + white text so it's clear something went wrong.
  resizeCanvas(72);
  ctx.fillStyle = '#cc2222';
  ctx.fillRect(0, 0, 72, 72);
  ctx.fillStyle = '#ffffff';
  ctx.font = 'bold 11px monospace';
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText('ERR', 36, 22);
  ctx.font = '8px monospace';
  ctx.fillText(String(err).slice(0, 36), 36, 42);
  ctx.fillText(String(err).slice(36, 72), 36, 56);
});
