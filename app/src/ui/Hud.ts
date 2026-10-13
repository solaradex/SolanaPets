/**
 * HUD — top overlay: pet name, mood pill, live stat bars, day/night phase.
 * Pure DOM, updated on a throttle from Game (4Hz is plenty for bars).
 */
import { MOOD_LABELS, Mood, PetStats } from '../config';

const STATS: Array<{ key: keyof PetStats; label: string; color: string }> = [
  { key: 'hunger', label: 'Food', color: '#ffb35c' },
  { key: 'happiness', label: 'Joy', color: '#ffd95c' },
  { key: 'cleanliness', label: 'Clean', color: '#7fd4ff' },
  { key: 'energy', label: 'Energy', color: '#9dff8a' },
  { key: 'affection', label: 'Love', color: '#ff8ab3' },
];

const MOOD_DOT: Record<Mood, string> = {
  happy: '#9dff8a',
  content: '#7fd4ff',
  hungry: '#ffb35c',
  sleepy: '#b9a7e6',
  dirty: '#8ad8d8',
  lonely: '#ff8ab3',
  bored: '#c9c9d9',
};

export class Hud {
  readonly el = document.createElement('div');
  private moodPill!: HTMLElement;
  private moodDot!: HTMLElement;
  private bars = new Map<keyof PetStats, HTMLElement>();
  private phaseEl!: HTMLElement;

  constructor(petName: string) {
    this.el.className = 'hud';

    const top = document.createElement('div');
    top.className = 'hud-top';

    const name = document.createElement('div');
    name.className = 'hud-name';
    name.textContent = petName;

    this.moodPill = document.createElement('div');
    this.moodPill.className = 'hud-mood';
    this.moodDot = document.createElement('span');
    this.moodDot.className = 'hud-mood-dot';
    const moodText = document.createElement('span');
    moodText.className = 'hud-mood-text';
    this.moodPill.append(this.moodDot, moodText);

    this.phaseEl = document.createElement('div');
    this.phaseEl.className = 'hud-phase';

    top.append(name, this.moodPill, this.phaseEl);

    const stats = document.createElement('div');
    stats.className = 'hud-stats';
    for (const s of STATS) {
      const wrap = document.createElement('div');
      wrap.className = 'hud-stat';
      const label = document.createElement('span');
      label.textContent = s.label;
      const track = document.createElement('div');
      track.className = 'hud-bar-track';
      const fill = document.createElement('div');
      fill.className = 'hud-bar-fill';
      fill.style.background = s.color;
      track.appendChild(fill);
      wrap.append(label, track);
      stats.appendChild(wrap);
      this.bars.set(s.key, fill);
    }

    this.el.append(top, stats);
  }

  update(stats: PetStats, mood: Mood, phaseName: string): void {
    const text = this.moodPill.querySelector('.hud-mood-text');
    if (text) text.textContent = MOOD_LABELS[mood];
    this.moodDot.style.background = MOOD_DOT[mood];
    for (const [key, fill] of this.bars) {
      fill.style.width = `${Math.round(stats[key])}%`;
    }
    this.phaseEl.textContent = phaseName;
  }
}
