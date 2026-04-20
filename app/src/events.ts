// ── Event binding ─────────────────────────────────────────────────────────────
// bindEvents()         — attaches DOM listeners after every render()
// handleButtonClick()  — central switch for all data-action buttons
// handleGlobalKeydown() — keyboard capture for hotkey assignment

import { invoke } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';
import { ctx } from './ctx';
import { render, setSaveMsg } from './render';
import { t, tf, setLang } from './i18n';
import { serializeHotkey } from './hotkeys';
import { activeProfile } from './utils';
import {
  updateTimer, patchTimer, addTimer, removeTimer,
  createProfile, cloneProfile, renameProfile, deleteProfile,
  modalConfirm, modalCancel,
  encodeProfiles, decodeProfiles, exportConfig, importConfig,
  saveState, openOverlays, closeOverlays, startWatch, stopWatch,
  resetOverlayLayout, toggleOverlayEditMode,
} from './actions';

// ── bindEvents ────────────────────────────────────────────────────────────────

export function bindEvents(): void {
  // Global keyboard listener — registered exactly once across all renders.
  if (!ctx.globalKeydownBound) {
    document.addEventListener('keydown', handleGlobalKeydown);
    ctx.globalKeydownBound = true;
  }

  ctx.root.querySelectorAll<HTMLElement>('[data-action]').forEach(el => {
    const action = el.dataset.action;
    if (!action) return;

    if (el instanceof HTMLButtonElement || el.tagName === 'BUTTON') {
      el.addEventListener('click', () => handleButtonClick(el, action));
      return;
    }

    if (el instanceof HTMLSelectElement) {
      el.addEventListener('change', () => {
        if (action === 'switch-profile') {
          ctx.selectedProfile = el.value;
          ctx.state.activeProfile = el.value;
          ctx.showHotkeyLabels = activeProfile().showHotkeyLabels ?? true;
          void emit('hotkey-label-changed', ctx.showHotkeyLabels);
          ctx.runningTimers.clear();
          void invoke('stop_all_timers').catch(() => {});
          void invoke<number>('update_hotkey_registrations', { statePayload: ctx.state }).catch(() => {});
          if (ctx.overlaysOpen) {
            void invoke('open_overlays', { statePayload: ctx.state }).catch(() => {});
          }
          render();
        } else if (action === 'switch-lang') {
          setLang(el.value as 'ru' | 'en');
          render();
        }
      });
      return;
    }

    if (el instanceof HTMLInputElement) {
      const index = Number(el.dataset.index ?? '-1');
      switch (action) {
        case 'duration':
          el.addEventListener('input', () => patchTimer(index, { duration: Number(el.value) }));
          break;
        case 'blink-threshold':
          el.addEventListener('input', () => patchTimer(index, { blinkThreshold: Number(el.value) }));
          break;
        case 'size':
          el.addEventListener('input', () => patchTimer(index, { size: Number(el.value) }));
          break;
        case 'opacity':
          el.addEventListener('input', () => patchTimer(index, { opacity: Number(el.value) }));
          break;
        case 'blink-color':
          el.addEventListener('input', () => patchTimer(index, { blinkColor: el.value }));
          break;
        case 'toggle-enabled':
          el.addEventListener('change', () => updateTimer(index, { enabled: el.checked }));
          break;
        case 'toggle-blink':
          el.addEventListener('change', () => updateTimer(index, { blink: el.checked }));
          break;
        case 'toggle-hide-show':
          el.addEventListener('change', () => {
            ctx.showHotkeyLabels = el.checked;
            activeProfile().showHotkeyLabels = ctx.showHotkeyLabels;
            void emit('hotkey-label-changed', ctx.showHotkeyLabels);
            render();
          });
          break;
        case 'toggle-layout-edit':
          el.addEventListener('change', () => {
            if (ctx.overlaysOpen) { void toggleOverlayEditMode(); }
            else { render(); }
          });
          break;
        case 'toggle-auto-show':
          el.addEventListener('change', () => { ctx.state.autoShowOverlays = el.checked; });
          break;
      }
    }
  });
}

// ── handleButtonClick ─────────────────────────────────────────────────────────

export function handleButtonClick(el: HTMLElement, action: string): void {
  const index = Number(el.dataset.index ?? '-1');

  switch (action) {
    // Profile management
    case 'new-profile':    createProfile(); break;
    case 'clone-profile':  cloneProfile();  break;
    case 'rename-profile': renameProfile(); break;
    case 'delete-profile': deleteProfile(); break;

    // Timer management
    case 'add-timer':    addTimer();           break;
    case 'remove-timer': removeTimer(index);   break;
    case 'toggle-settings':
      ctx.expandedTimerIndex = ctx.expandedTimerIndex === index ? null : index;
      render();
      break;
    case 'select-icon':
      updateTimer(index, { icon: el.dataset.icon ?? '' });
      break;
    case 'test-timer': {
      const timer = activeProfile().timers[index];
      if (timer) {
        void invoke('trigger_timer', {
          timerIndex: index,
          durationSecs: timer.duration,
          blinkThresholdSecs: timer.blinkThreshold,
        });
      }
      break;
    }

    // Overlay controls
    case 'save-state':       void saveState();            break;
    case 'open-overlays':    void openOverlays();         break;
    case 'close-overlays':   void closeOverlays();        break;
    case 'toggle-edit-mode': void toggleOverlayEditMode(); break;
    case 'reset-layout':     void resetOverlayLayout();   break;
    case 'start-watch':      void startWatch();           break;
    case 'stop-watch':       void stopWatch();            break;

    // Per-timer hotkey capture
    case 'hotkey':
      if (ctx.capturingHotkeyIndex === index && ctx.capturingHotkeyField === 'hotkey') {
        ctx.capturingHotkeyIndex = null;
        ctx.capturingHotkeyField = 'hotkey';
        ctx.capturingGlobalHotkey = null;
        render();
      } else {
        ctx.capturingHotkeyIndex = index;
        ctx.capturingHotkeyField = 'hotkey';
        ctx.capturingGlobalHotkey = null;
        setSaveMsg(t('msgCapturingTimer'), 'idle');
      }
      break;
    case 'hotkey2':
      if (ctx.capturingHotkeyIndex === index && ctx.capturingHotkeyField === 'hotkey2') {
        ctx.capturingHotkeyIndex = null;
        ctx.capturingHotkeyField = 'hotkey';
        ctx.capturingGlobalHotkey = null;
        render();
      } else {
        ctx.capturingHotkeyIndex = index;
        ctx.capturingHotkeyField = 'hotkey2';
        ctx.capturingGlobalHotkey = null;
        setSaveMsg(t('msgCapturingTimer'), 'idle');
      }
      break;
    case 'clear-hotkey':
      updateTimer(index, { hotkey: '' });
      setSaveMsg(t('msgHotkeyCleared'), 'success');
      break;
    case 'clear-hotkey2':
      updateTimer(index, { hotkey2: '' });
      setSaveMsg(t('msgHotkeyCleared'), 'success');
      break;

    // Global hotkey capture
    case 'capture-hide-show':
      if (ctx.capturingGlobalHotkey === 'hideShow') { ctx.capturingGlobalHotkey = null; render(); }
      else { ctx.capturingGlobalHotkey = 'hideShow'; ctx.capturingHotkeyIndex = null; setSaveMsg(t('msgCapturingHideShow'), 'idle'); }
      break;
    case 'capture-hide-show2':
      if (ctx.capturingGlobalHotkey === 'hideShow2') { ctx.capturingGlobalHotkey = null; render(); }
      else { ctx.capturingGlobalHotkey = 'hideShow2'; ctx.capturingHotkeyIndex = null; setSaveMsg(t('msgCapturingHideShow2'), 'idle'); }
      break;
    case 'capture-layout-edit':
      if (ctx.capturingGlobalHotkey === 'layoutEdit') { ctx.capturingGlobalHotkey = null; render(); }
      else { ctx.capturingGlobalHotkey = 'layoutEdit'; ctx.capturingHotkeyIndex = null; setSaveMsg(t('msgCapturingLayoutEdit'), 'idle'); }
      break;
    case 'capture-layout-edit2':
      if (ctx.capturingGlobalHotkey === 'layoutEdit2') { ctx.capturingGlobalHotkey = null; render(); }
      else { ctx.capturingGlobalHotkey = 'layoutEdit2'; ctx.capturingHotkeyIndex = null; setSaveMsg(t('msgCapturingLayoutEdit2'), 'idle'); }
      break;

    // macOS permission
    case 'open-privacy-settings':
      void invoke('open_privacy_settings').catch(() => {});
      break;
    case 'reset-input-monitoring':
      void invoke('reset_input_monitoring_permission')
        .then(() => setSaveMsg(t('msgResetPermissionDone'), 'success'))
        .catch(err => setSaveMsg(`${t('msgResetPermissionError')} ${String(err)}`, 'error'));
      break;

    // Export / import
    case 'export-config': exportConfig(); break;
    case 'import-config': importConfig(); break;
    case 'export-all':
      ctx.exportBase64 = encodeProfiles(ctx.state.profiles);
      ctx.modalKind = 'export-config';
      render();
      break;
    case 'export-current':
      ctx.exportBase64 = encodeProfiles({ [ctx.selectedProfile]: activeProfile() });
      ctx.modalKind = 'export-config';
      render();
      break;
    case 'modal-copy':
      void navigator.clipboard.writeText(ctx.exportBase64)
        .then(() => setSaveMsg(t('msgCopied'), 'success'))
        .catch(() => setSaveMsg(t('msgCopyFailed'), 'error'));
      break;

    // Modal
    case 'modal-confirm': modalConfirm(); break;
    case 'modal-cancel':  modalCancel();  break;
  }
}

// ── handleGlobalKeydown ───────────────────────────────────────────────────────

export function handleGlobalKeydown(ev: KeyboardEvent): void {
  if (ctx.capturingHotkeyIndex === null && ctx.capturingGlobalHotkey === null) return;

  ev.preventDefault();
  ev.stopPropagation();

  // Clear key (Esc / Backspace / Delete)
  if (ev.key === 'Escape' || ev.key === 'Backspace' || ev.key === 'Delete') {
    if (ctx.capturingHotkeyIndex !== null) {
      const idx   = ctx.capturingHotkeyIndex;
      const field = ctx.capturingHotkeyField;
      ctx.capturingHotkeyIndex = null;
      ctx.capturingHotkeyField = 'hotkey';
      updateTimer(idx, { [field]: '' });
      setSaveMsg(t('msgHotkeyCleared'), 'success');
    }
    if (ctx.capturingGlobalHotkey === 'hideShow')    { ctx.capturingGlobalHotkey = null; ctx.state.hideShowHotkey    = ''; setSaveMsg(t('msgHideShowCleared'),    'success'); }
    if (ctx.capturingGlobalHotkey === 'hideShow2')   { ctx.capturingGlobalHotkey = null; ctx.state.hideShowHotkey2   = ''; setSaveMsg(t('msgHideShow2Cleared'),   'success'); }
    if (ctx.capturingGlobalHotkey === 'layoutEdit')  { ctx.capturingGlobalHotkey = null; ctx.state.layoutEditHotkey  = ''; setSaveMsg(t('msgLayoutEditCleared'),  'success'); }
    if (ctx.capturingGlobalHotkey === 'layoutEdit2') { ctx.capturingGlobalHotkey = null; ctx.state.layoutEditHotkey2 = ''; setSaveMsg(t('msgLayoutEdit2Cleared'), 'success'); }
    return;
  }

  const hotkey = serializeHotkey(ev);
  if (!hotkey) return; // bare modifier key — ignore

  if (ctx.capturingHotkeyIndex !== null) {
    const idx   = ctx.capturingHotkeyIndex;
    const field = ctx.capturingHotkeyField;
    ctx.capturingHotkeyIndex = null;
    ctx.capturingHotkeyField = 'hotkey';
    updateTimer(idx, { [field]: hotkey });
    setSaveMsg(tf('msgHotkeySet', { key: hotkey }), 'success');
    return;
  }

  if (ctx.capturingGlobalHotkey === 'hideShow')    { ctx.capturingGlobalHotkey = null; ctx.state.hideShowHotkey    = hotkey; setSaveMsg(tf('msgHideShowSet',    { key: hotkey }), 'success'); render(); return; }
  if (ctx.capturingGlobalHotkey === 'hideShow2')   { ctx.capturingGlobalHotkey = null; ctx.state.hideShowHotkey2   = hotkey; setSaveMsg(tf('msgHideShow2Set',   { key: hotkey }), 'success'); render(); return; }
  if (ctx.capturingGlobalHotkey === 'layoutEdit')  { ctx.capturingGlobalHotkey = null; ctx.state.layoutEditHotkey  = hotkey; setSaveMsg(tf('msgLayoutEditSet',  { key: hotkey }), 'success'); render(); return; }
  if (ctx.capturingGlobalHotkey === 'layoutEdit2') { ctx.capturingGlobalHotkey = null; ctx.state.layoutEditHotkey2 = hotkey; setSaveMsg(tf('msgLayoutEdit2Set', { key: hotkey }), 'success'); render(); return; }
}
