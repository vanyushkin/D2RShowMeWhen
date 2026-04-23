// ── Render ────────────────────────────────────────────────────────────────────
// Generates the full UI HTML and injects it into #app.
//
// Dependency-injection note:
//   render() calls bindEvents() after each DOM replacement. To avoid a circular
//   import (render → events → render), bootstrap.ts wires the callback at
//   startup via setBindEventsCallback().

import { ctx } from './ctx';
import { t, tf, lang } from './i18n';
import {
  activeProfile,
  ensureProfileSelection,
  resolveAssetUrl,
  hotkeyPlaceholder,
  platformLabel,
  escapeHtml,
} from './utils';
import type { TimerConfig } from './types';

// ── Dependency-injection slot ─────────────────────────────────────────────────

let _bindEvents: (() => void) | null = null;

/** Called once from bootstrap.ts to wire in the event binder. */
export function setBindEventsCallback(fn: () => void): void {
  _bindEvents = fn;
}

// ── Status helper ─────────────────────────────────────────────────────────────

export function setSaveMsg(msg: string, kind: 'idle' | 'success' | 'error' = 'idle'): void {
  ctx.saveMsg = msg;
  ctx.saveMsgKind = kind;
  render();
}

// ── Sub-renderers (return HTML strings) ───────────────────────────────────────

export function renderModal(): string {
  const { modalKind, modalInputValue: _mi } = ctx;

  if (modalKind === 'confirm-delete-profile') {
    return `<div class="inline-modal">
      <span class="modal-label">${t('modalDeleteConfirm').replace('{name}', escapeHtml(ctx.selectedProfile))}</span>
      <button class="btn sm danger" data-action="modal-confirm">${t('btnDeleteConfirm')}</button>
      <button class="btn sm" data-action="modal-cancel">${t('btnCancel')}</button>
    </div>`;
  }
  if (modalKind === 'export-choice') {
    return `<div class="inline-modal">
      <span class="modal-label">${t('exportChoice')}</span>
      <button class="btn sm primary" data-action="export-all">${t('exportAll')}</button>
      <button class="btn sm" data-action="export-current">${t('exportCurrent')}</button>
      <button class="btn sm" data-action="modal-cancel">${t('btnCancel')}</button>
    </div>`;
  }
  if (modalKind === 'export-config') {
    return `<div class="inline-modal stacked">
      <span class="modal-label">${t('exportLabel')}</span>
      <textarea class="modal-textarea" readonly spellcheck="false"></textarea>
      <div class="modal-actions">
        <button class="btn sm primary" data-action="modal-copy">${t('btnCopy')}</button>
        <button class="btn sm" data-action="modal-cancel">${t('btnClose')}</button>
      </div>
    </div>`;
  }
  if (modalKind === 'import-config') {
    return `<div class="inline-modal stacked">
      <span class="modal-label">${t('importLabel')}</span>
      <textarea class="modal-textarea" placeholder="${t('importPlaceholder')}" spellcheck="false"></textarea>
      <div class="modal-actions">
        <button class="btn sm primary" data-action="modal-confirm">${t('btnImport')}</button>
        <button class="btn sm" data-action="modal-cancel">${t('btnCancel')}</button>
      </div>
    </div>`;
  }
  // new-profile / clone-profile / rename-profile
  const label = modalKind === 'new-profile'
    ? t('modalNewName')
    : modalKind === 'clone-profile'
      ? t('modalCloneName').replace('{name}', escapeHtml(ctx.selectedProfile))
      : t('modalRenameTo').replace('{name}', escapeHtml(ctx.selectedProfile));
  return `<div class="inline-modal">
    <span class="modal-label">${label}</span>
    <input class="modal-input" type="text" placeholder="${t('modalProfileName')}"
      autocomplete="off" spellcheck="false" />
    <button class="btn sm primary" data-action="modal-confirm">${t('btnOK')}</button>
    <button class="btn sm" data-action="modal-cancel">${t('btnCancel')}</button>
  </div>`;
}

export function renderExpandedSettings(timer: TimerConfig, index: number): string {
  const iconChoices = ctx.bootstrap.icons
    .map(asset => {
      const url = resolveAssetUrl(asset);
      const img = url
        ? `<img src="${url}" alt="${asset.label}" />`
        : `<span>${asset.label.slice(0, 2).toUpperCase()}</span>`;
      const active = asset.fileName === timer.icon ? 'active' : '';
      return `<button class="icon-option ${active}" data-action="select-icon"
                data-index="${index}" data-icon="${asset.fileName}" title="${asset.label}">
                ${img}<span>${asset.label}</span>
              </button>`;
    })
    .join('');

  return `
    <div class="timer-settings">
      <div class="timer-settings-grid">
        <div>
          <div class="field-label">${t('labelBlinkThreshold')}</div>
          <input class="field-input" type="number" min="1" max="60"
            value="${timer.blinkThreshold}" data-action="blink-threshold" data-index="${index}" />
        </div>
        <div>
          <div class="field-label">${t('labelBlinkColor')}</div>
          <input class="color-input" type="color"
            value="${timer.blinkColor}" data-action="blink-color" data-index="${index}" />
        </div>
        <div style="display:flex;align-items:center;gap:6px;padding-top:14px;">
          <label class="toggle" style="flex-shrink:0">
            <input type="checkbox" ${timer.blink ? 'checked' : ''}
              data-action="toggle-blink" data-index="${index}" />
            <span class="toggle-slider"></span>
          </label>
          <span class="field-label" style="margin:0">${t('labelBlinkNearEnd')}</span>
        </div>
      </div>
      <div class="field-label" style="margin-bottom:5px">${t('labelIcon')}</div>
      <div class="icon-gallery">${iconChoices}</div>
    </div>
  `;
}

export function renderTimerRowWrap(timer: TimerConfig, index: number): string {
  const isCapturing  = ctx.capturingHotkeyIndex === index && ctx.capturingHotkeyField === 'hotkey';
  const isCapturing2 = ctx.capturingHotkeyIndex === index && ctx.capturingHotkeyField === 'hotkey2';
  const isExpanded   = ctx.expandedTimerIndex === index;
  const running      = ctx.runningTimers.get(index);

  const icon    = ctx.iconMap.get(timer.icon);
  const iconUrl = icon ? resolveAssetUrl(icon) : null;
  const iconImg = iconUrl
    ? `<img src="${iconUrl}" alt="${icon?.label ?? ''}" />`
    : `<span style="font-size:11px;color:var(--muted)">?</span>`;

  const durationCell = running
    ? (() => {
        const pct = Math.max(0, (running.remainingSecs / running.totalSecs) * 100);
        const bc = running.blinking ? 'blinking' : '';
        return `<div class="countdown-display">
          <span class="countdown-time ${bc}">${running.remainingSecs}s</span>
          <div class="countdown-bar-wrap">
            <div class="countdown-bar-fill ${bc}" style="width:${pct}%"></div>
          </div>
        </div>`;
      })()
    : `<input class="duration-input" type="number" min="1" max="36000"
         value="${timer.duration}" data-action="duration" data-index="${index}" />`;

  const hotkeyLabel        = timer.hotkey || '';
  const hotkeyCapturingCls = isCapturing  ? 'capturing' : (!hotkeyLabel       ? 'empty' : '');
  const hotkey2Label       = timer.hotkey2 || '';
  const hotkey2CapturingCls = isCapturing2 ? 'capturing' : (!hotkey2Label      ? 'empty' : '');
  const ph = hotkeyPlaceholder(ctx.bootstrap.platform.kind);

  const hotkeyText  = isCapturing  ? t('btnPressKey') : (hotkeyLabel  || ph);
  const hotkey2Text = isCapturing2 ? t('btnPressKey') : (hotkey2Label || ph);

  const rowActiveClass = running ? 'timer-active' : '';
  const rowBlinkClass  = running?.blinking ? 'timer-blinking' : '';

  return `
    <div class="timer-row-wrap" data-index="${index}">
      <div class="timer-row ${rowActiveClass} ${rowBlinkClass}">
        <span class="row-num">${index + 1}</span>

        <button class="btn icon-only" data-action="test-timer" data-index="${index}"
          title="${t('titleTestTimer')}">${iconImg}</button>

        <div class="duration-cell">${durationCell}</div>

        <input class="row-input" type="number" min="40" max="250"
          value="${timer.size}" data-action="size" data-index="${index}"
          title="${t('labelSize')}" />

        <input class="row-input" type="number" min="15" max="100"
          value="${timer.opacity}" data-action="opacity" data-index="${index}"
          title="${t('labelOpacity')}" />

        <div class="hotkey-cell">
          <button class="hotkey-btn ${hotkeyCapturingCls}"
            data-action="hotkey" data-index="${index}"
            title="Click to capture hotkey">${hotkeyText}</button>
          ${hotkeyLabel && !isCapturing ? `<button class="hotkey-clear" data-action="clear-hotkey" data-index="${index}" title="Clear hotkey">×</button>` : ''}
        </div>

        <div class="hotkey-cell">
          <button class="hotkey-btn ${hotkey2CapturingCls}"
            data-action="hotkey2" data-index="${index}"
            title="Click to capture alt layout hotkey">${hotkey2Text}</button>
          ${hotkey2Label && !isCapturing2 ? `<button class="hotkey-clear" data-action="clear-hotkey2" data-index="${index}" title="Clear hotkey">×</button>` : ''}
        </div>

        <div class="toggle-wrap">
          <label class="toggle">
            <input type="checkbox" ${timer.enabled ? 'checked' : ''}
              data-action="toggle-enabled" data-index="${index}" />
            <span class="toggle-slider"></span>
          </label>
        </div>

        <button class="btn icon-only" data-action="toggle-settings" data-index="${index}"
          title="${t('titleTimerSettings')}" style="${isExpanded ? 'border-color:var(--accent);' : ''}">⚙</button>

        <button class="btn icon-only danger" data-action="remove-timer" data-index="${index}"
          title="${t('titleRemoveTimer')}">✕</button>
      </div>
      ${isExpanded ? renderExpandedSettings(timer, index) : ''}
    </div>
  `;
}

// ── Main render ───────────────────────────────────────────────────────────────

export function render(): void {
  ensureProfileSelection();
  const timers = activeProfile().timers;

  // Clamp capturing index if rows were removed
  if (ctx.capturingHotkeyIndex !== null && ctx.capturingHotkeyIndex >= timers.length) {
    ctx.capturingHotkeyIndex = null;
  }

  const profileOptions = Object.keys(ctx.state.profiles)
    .sort((a, b) => a.localeCompare(b))
    .map(name => `<option value="${name}" ${name === ctx.selectedProfile ? 'selected' : ''}>${name}</option>`)
    .join('');

  const timerRows = timers.map((timer, i) => renderTimerRowWrap(timer, i)).join('');

  const hideShowCapturing    = ctx.capturingGlobalHotkey === 'hideShow';
  const hideShow2Capturing   = ctx.capturingGlobalHotkey === 'hideShow2';
  const layoutEditCapturing  = ctx.capturingGlobalHotkey === 'layoutEdit';
  const layoutEdit2Capturing = ctx.capturingGlobalHotkey === 'layoutEdit2';

  ctx.root.innerHTML = `
    <div class="titlebar">
      <span class="app-title" data-tauri-drag-region>D2R Show Me When</span>
      <span class="titlebar-drag-fill" data-tauri-drag-region></span>
      <div class="titlebar-right">
        <label class="lang-label">${t('langLabel')}</label>
        <select class="lang-select" data-action="switch-lang">
          <option value="ru" ${lang === 'ru' ? 'selected' : ''}>RU</option>
          <option value="en" ${lang === 'en' ? 'selected' : ''}>EN</option>
        </select>
        <select class="scale-select" data-action="switch-scale" title="${t('scaleLabel')}">
          ${[100, 125, 150, 175, 200, 225, 250].map(s =>
            `<option value="${s}" ${ctx.uiScale === s ? 'selected' : ''}>${s}%</option>`
          ).join('')}
        </select>
        <span class="platform-badge">${platformLabel(ctx.bootstrap.platform.kind)}</span>
        <div class="win-controls">
          <button class="win-btn win-min" data-action="win-minimize" title="Minimize">−</button>
          <button class="win-btn win-close" data-action="win-close" title="Close">✕</button>
        </div>
      </div>
    </div>

    ${ctx.listenerError ? `
    <div class="listener-error-banner visible">
      <div class="listener-error-text">${ctx.listenerError}</div>
      <div class="listener-error-footer">
        <button class="btn sm" data-action="open-privacy-settings">${t('btnOpenPrivacySettings')}</button>
        <button class="btn sm primary" data-action="reset-input-monitoring">${t('btnResetPermission')}</button>
        <span class="listener-error-hint">${t('msgInputMonitoringHint')}</span>
      </div>
    </div>` : ''}

    <div class="profile-bar">
      <span class="profile-label">${t('profile')}</span>
      <select class="profile-select" data-action="switch-profile">${profileOptions}</select>
      <button class="btn sm" data-action="new-profile">${t('btnNew')}</button>
      <button class="btn sm" data-action="clone-profile">${t('btnClone')}</button>
      <button class="btn sm" data-action="rename-profile">${t('btnRename')}</button>
    </div>
    <div class="profile-actions-bar">
      <button class="btn sm danger" data-action="delete-profile">${t('btnDelete')}</button>
      <button class="btn sm" data-action="export-config" title="${t('titleExport')}">↑ ${t('btnExport')}</button>
      <button class="btn sm" data-action="import-config" title="${t('titleImport')}">↓ ${t('btnImport')}</button>
    </div>
    ${ctx.modalKind !== 'none' ? renderModal() : ''}

    <div class="timer-list-header">
      <span>${t('colNum')}</span>
      <span></span>
      <span>${t('colDur')}</span>
      <span>${t('colSize')}</span>
      <span>${t('colOpc')}</span>
      <span>${t('colHotkey')}</span>
      <span>${t('colHotkey2')}</span>
      <span>${t('colOn')}</span>
      <span></span>
      <span></span>
    </div>

    <div id="timer-list">
      ${timerRows}
    </div>

    <div class="add-timer-row">
      <button class="btn sm" data-action="add-timer">${t('btnAddTimer')}</button>
    </div>

    <div class="section-header">${t('sectionOverlays')}</div>

    <div class="overlay-bar">
      <label class="auto-show-label">
        <label class="toggle">
          <input type="checkbox" ${ctx.state.autoShowOverlays ? 'checked' : ''}
            data-action="toggle-auto-show" />
          <span class="toggle-slider"></span>
        </label>
        <span class="auto-show-text">${t('autoShow')}</span>
      </label>
    </div>

    <div class="overlay-bar">
      ${ctx.overlaysOpen
        ? `<button class="btn sm danger" data-action="close-overlays">${t('btnHideOverlays')}</button>
           <button class="btn sm ${ctx.overlayEditMode ? 'primary active' : ''}" data-action="toggle-edit-mode">
             ${ctx.overlayEditMode ? t('btnLockLayout') : t('btnEditLayout')}
           </button>
           <button class="btn sm" data-action="reset-layout" title="${t('msgResetDone')}">${t('btnResetLayout')}</button>
           <button class="btn sm ${ctx.watching ? 'danger' : 'primary'}" data-action="${ctx.watching ? 'stop-watch' : 'start-watch'}">
             ${ctx.watching ? t('btnStop') : t('btnStart')}
           </button>`
        : `<button class="btn sm primary" data-action="open-overlays">${t('btnShowOverlays')}</button>`
      }
      <span class="overlay-hint">${
        ctx.overlaysOpen
          ? (ctx.overlayEditMode ? t('hintEditMode') : ctx.watching ? t('hintWatching') : t('hintPaused'))
          : t('hintHidden')
      }</span>
    </div>

    <div class="section-header-global">
      <span>${t('sectionGlobal')}</span>
      <span></span>
      <span class="field-label">${t('colHotkey')}</span>
      <span class="field-label">${t('colHotkey2')}</span>
    </div>

    <div class="control-row">
      <span class="control-label">${t('labelHideShow')}</span>
      <label class="toggle">
        <input type="checkbox" ${ctx.showHotkeyLabels ? 'checked' : ''} data-action="toggle-hide-show" />
        <span class="toggle-slider"></span>
      </label>
      <button class="set-key-btn ${hideShowCapturing ? 'capturing' : (!ctx.state.hideShowHotkey ? 'empty' : '')}"
        data-action="capture-hide-show">
        ${hideShowCapturing ? t('btnPressKey') : (ctx.state.hideShowHotkey || t('btnSetHotkey'))}
      </button>
      <button class="set-key-btn ${hideShow2Capturing ? 'capturing' : (!ctx.state.hideShowHotkey2 ? 'empty' : '')}"
        data-action="capture-hide-show2">
        ${hideShow2Capturing ? t('btnPressKey') : (ctx.state.hideShowHotkey2 || t('btnSetHotkey'))}
      </button>
    </div>

    <div class="control-row">
      <span class="control-label">${t('labelLayoutEdit')}</span>
      <label class="toggle">
        <input type="checkbox" ${ctx.overlayEditMode ? 'checked' : ''}
          ${!ctx.overlaysOpen ? 'disabled' : ''}
          data-action="toggle-layout-edit" />
        <span class="toggle-slider"></span>
      </label>
      <button class="set-key-btn ${layoutEditCapturing ? 'capturing' : (!ctx.state.layoutEditHotkey ? 'empty' : '')}"
        data-action="capture-layout-edit">
        ${layoutEditCapturing ? t('btnPressKey') : (ctx.state.layoutEditHotkey || t('btnSetHotkey'))}
      </button>
      <button class="set-key-btn ${layoutEdit2Capturing ? 'capturing' : (!ctx.state.layoutEditHotkey2 ? 'empty' : '')}"
        data-action="capture-layout-edit2">
        ${layoutEdit2Capturing ? t('btnPressKey') : (ctx.state.layoutEditHotkey2 || t('btnSetHotkey'))}
      </button>
    </div>

    <div class="status-bar">
      <span class="save-msg ${ctx.saveMsgKind}">${ctx.saveMsg}</span>
      <button class="btn primary" data-action="save-state">${t('btnSave')}</button>
    </div>

    <div class="attribution">
      Based on <strong>D2R Show Me When v3.0 by GlassCannon</strong> — with gratitude
    </div>
  `;

  _bindEvents?.();

  // Re-attach hotkey button focus if in capture mode
  if (ctx.capturingHotkeyIndex !== null) {
    const action = ctx.capturingHotkeyField === 'hotkey2' ? 'hotkey2' : 'hotkey';
    ctx.root.querySelector<HTMLButtonElement>(
      `[data-action="${action}"][data-index="${ctx.capturingHotkeyIndex}"]`
    )?.focus();
  }

  // Restore countdown displays for any running timers
  for (const [index] of ctx.runningTimers) {
    updateTimerRowDisplay(index);
  }

  // Wire modal input without triggering re-render on each keystroke
  if (ctx.modalKind === 'new-profile' || ctx.modalKind === 'clone-profile' || ctx.modalKind === 'rename-profile') {
    const input = ctx.root.querySelector<HTMLInputElement>('.modal-input');
    if (input) {
      input.value = ctx.modalInputValue;
      input.focus();
      input.select();
      input.addEventListener('input', () => { ctx.modalInputValue = input.value; });
      input.addEventListener('keydown', e => {
        if (e.key === 'Enter')  { e.preventDefault(); modalConfirmCallback?.(); }
        if (e.key === 'Escape') { e.preventDefault(); modalCancelCallback?.(); }
      });
    }
  }

  if (ctx.modalKind === 'export-config') {
    const ta = ctx.root.querySelector<HTMLTextAreaElement>('.modal-textarea');
    if (ta) { ta.value = ctx.exportBase64; ta.focus(); ta.select(); }
  }

  if (ctx.modalKind === 'import-config') {
    const ta = ctx.root.querySelector<HTMLTextAreaElement>('.modal-textarea');
    if (ta) {
      ta.value = ctx.modalInputValue;
      ta.focus();
      ta.addEventListener('input', () => { ctx.modalInputValue = ta.value; });
      ta.addEventListener('keydown', e => {
        if (e.key === 'Escape') { e.preventDefault(); modalCancelCallback?.(); }
      });
    }
  }
}

// ── Targeted DOM update for running timer rows ────────────────────────────────
// Called by actions.ts on every timer_tick — avoids a full re-render.

export function updateTimerRowDisplay(index: number): void {
  const wrap = ctx.root.querySelector<HTMLElement>(`.timer-row-wrap[data-index="${index}"]`);
  if (!wrap) return;
  const row = wrap.querySelector<HTMLElement>('.timer-row');
  if (!row) return;

  const running      = ctx.runningTimers.get(index);
  const durationCell = row.querySelector<HTMLElement>('.duration-cell');
  if (!durationCell) return;

  if (running) {
    const pct       = Math.max(0, (running.remainingSecs / running.totalSecs) * 100);
    const blinkCls  = running.blinking ? 'blinking' : '';
    durationCell.innerHTML = `
      <div class="countdown-display">
        <span class="countdown-time ${blinkCls}">${running.remainingSecs}s</span>
        <div class="countdown-bar-wrap">
          <div class="countdown-bar-fill ${blinkCls}" style="width:${pct}%"></div>
        </div>
      </div>`;
    row.classList.add('timer-active');
    row.classList.toggle('timer-blinking', running.blinking);
  } else {
    const timer = activeProfile().timers[index];
    if (timer) {
      durationCell.innerHTML = `<input class="duration-input" type="number" min="1" max="36000"
        value="${timer.duration}" data-action="duration" data-index="${index}" />`;
      // Re-bind the input so patchTimer stays connected
      const el = durationCell.querySelector<HTMLInputElement>('.duration-input')!;
      el.addEventListener('input', () => {
        activeProfile().timers[index] = { ...activeProfile().timers[index], duration: Number(el.value) };
      });
    }
    row.classList.remove('timer-active', 'timer-blinking');
  }
}

// ── Modal callback slots (injected by actions.ts) ─────────────────────────────
// Avoids importing actions from render to prevent a cycle.

let modalConfirmCallback: (() => void) | null = null;
let modalCancelCallback:  (() => void) | null = null;

export function setModalCallbacks(onConfirm: () => void, onCancel: () => void): void {
  modalConfirmCallback = onConfirm;
  modalCancelCallback  = onCancel;
}
