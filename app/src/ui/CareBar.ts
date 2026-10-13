/**
 * CareBar — the five big thumb-friendly care buttons.
 *
 * Pure DOM (not Pixi) for accessibility, native touch behaviour and
 * crisp text rendering. Fixed to the bottom safe area; the game canvas
 * renders behind it.
 */
import { CareAction } from '../config';

const ICONS: Record<CareAction, string> = {
  // fish — feed
  feed: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4.5 12c2.8-3.8 7.2-3.8 10 0-2.8 3.8-7.2 3.8-10 0z"/><path d="M14.5 12l5-3.5v7l-5-3.5z"/><circle cx="8" cy="11.4" r="0.6" fill="currentColor"/></svg>',
  // ball — play
  play: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="8.2"/><path d="M12 3.8v16.4M3.8 12h16.4"/><path d="M6.2 6.2c2 1.6 2 9.9 0 11.6M17.8 6.2c-2 1.6-2 9.9 0 11.6"/></svg>',
  // droplet + sparkle — clean
  clean: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3.5c2.8 3.8 5.5 6.6 5.5 9.7a5.5 5.5 0 1 1-11 0c0-3.1 2.7-5.9 5.5-9.7z"/><path d="M18.5 3.5l.7 1.8 1.8.7-1.8.7-.7 1.8-.7-1.8-1.8-.7 1.8-.7z"/></svg>',
  // crescent moon — rest
  rest: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 13.2A8.2 8.2 0 1 1 10.8 4a6.6 6.6 0 0 0 9.2 9.2z"/></svg>',
  // heart — affection
  love: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20.5S4 15.4 4 9.7C4 7.2 6 5.2 8.5 5.2c1.5 0 2.8.8 3.5 2 .7-1.2 2-2 3.5-2C18 5.2 20 7.2 20 9.7c0 5.7-8 10.8-8 10.8z"/></svg>',
};

const ORDER: Array<{ action: CareAction; label: string }> = [
  { action: 'feed', label: 'Feed' },
  { action: 'play', label: 'Play' },
  { action: 'clean', label: 'Clean' },
  { action: 'rest', label: 'Rest' },
  { action: 'love', label: 'Love' },
];

export class CareBar {
  readonly el = document.createElement('div');
  private buttons = new Map<CareAction, HTMLButtonElement>();

  constructor(onCare: (action: CareAction) => void) {
    this.el.className = 'carebar';
    for (const { action, label } of ORDER) {
      const btn = document.createElement('button');
      btn.className = 'carebar-btn';
      btn.type = 'button';
      btn.setAttribute('aria-label', label);
      btn.innerHTML = `${ICONS[action]}<span>${label}</span>`;
      btn.addEventListener('click', () => onCare(action));
      this.el.appendChild(btn);
      this.buttons.set(action, btn);
    }
  }

  /** Disable all buttons except show a spinner state on the active one. */
  setBusy(active: CareAction | null): void {
    for (const [action, btn] of this.buttons) {
      btn.disabled = active !== null;
      btn.classList.toggle('is-active', active === action);
    }
  }
}
