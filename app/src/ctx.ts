// ── Application context ───────────────────────────────────────────────────────
// Single mutable object holding all runtime state so any module can import and
// read/write without needing circular imports or setter boilerplate.
//
// Two conceptual sections:
//   1. App data  — bootstrap payload, persisted AppState, profile selection
//   2. UI state  — ephemeral interaction flags that survive renders

import type {
  BootstrapPayload,
  AppState,
  IconAsset,
  RunningTimer,
  ModalKind,
  GlobalHotkeyField,
} from './types';

export const ctx = {
  // ── App data ───────────────────────────────────────────────────────────────
  bootstrap: undefined as unknown as BootstrapPayload,
  state: undefined as unknown as AppState,
  selectedProfile: '',

  // ── UI state ───────────────────────────────────────────────────────────────

  // Hotkey capture
  capturingHotkeyIndex: null as number | null,     // per-timer row being captured
  capturingHotkeyField: 'hotkey' as 'hotkey' | 'hotkey2',
  capturingGlobalHotkey: null as GlobalHotkeyField | null,

  // Timer rows
  expandedTimerIndex: null as number | null,

  // Inline modal
  modalKind: 'none' as ModalKind,
  modalInputValue: '',
  exportBase64: '',                                 // computed when export-config opens

  // Status bar
  saveMsg: '',
  saveMsgKind: 'idle' as 'idle' | 'success' | 'error',

  // Listener permission error (persists across renders)
  listenerError: '',

  // Overlay window state
  overlaysOpen: false,
  overlayEditMode: false,
  watching: false,

  // Per-profile UI flag (mirrored from profile.showHotkeyLabels)
  showHotkeyLabels: true,

  // UI zoom level (100 = default). Read from localStorage on startup,
  // applied via document.documentElement.style.zoom.
  uiScale: 100,

  // ── Lookup maps & DOM ─────────────────────────────────────────────────────
  iconMap: new Map<string, IconAsset>(),
  runningTimers: new Map<number, RunningTimer>(),
  root: document.querySelector<HTMLDivElement>('#app')!,
  globalKeydownBound: false,
};
