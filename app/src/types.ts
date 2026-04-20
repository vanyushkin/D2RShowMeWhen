// ── Types mirroring Rust models ───────────────────────────────────────────────

export type PlatformKind = 'macos' | 'windows' | 'linux-x11' | 'linux-wayland' | 'unknown';

export type TimerConfig = {
  enabled: boolean;
  hotkey: string;
  hotkey2: string;
  duration: number;
  icon: string;
  size: number;
  opacity: number;
  blink: boolean;
  blinkThreshold: number;
  blinkColor: string;
};

export type Profile = {
  timers: TimerConfig[];
  positions: Record<string, [number, number]>;
  showHotkeyLabels: boolean;
};

export type AppState = {
  activeProfile: string;
  profiles: Record<string, Profile>;
  hideShowHotkey: string;
  hideShowHotkey2: string;
  hideShowEnabled: boolean;
  layoutEditHotkey: string;
  layoutEditHotkey2: string;
  layoutEditEnabled: boolean;
  autoShowOverlays: boolean;
};

export type IconAsset = {
  fileName: string;
  label: string;
  filePath: string;
  assetUrl: string | null;
};

export type BootstrapPayload = {
  state: AppState;
  icons: IconAsset[];
  platform: { kind: PlatformKind; arch: string };
  storagePath: string;
  migratedFrom: string | null;
};

export type SaveResult = { storagePath: string };

export type TimerTickPayload = {
  timerIndex: number;
  remainingSecs: number;
  totalSecs: number;
  blinking: boolean;
  finished: boolean;
};

export type RunningTimer = {
  remainingSecs: number;
  totalSecs: number;
  blinking: boolean;
};

export type ModalKind =
  | 'none'
  | 'new-profile'
  | 'clone-profile'
  | 'rename-profile'
  | 'confirm-delete-profile'
  | 'export-choice'
  | 'export-config'
  | 'import-config';

export type GlobalHotkeyField = 'hideShow' | 'hideShow2' | 'layoutEdit' | 'layoutEdit2';
