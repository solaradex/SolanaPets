/**
 * MoodEngine — pure game logic mapping care stats to mood.
 *
 * Stats decay slowly in real time; care actions restore them (see
 * CONFIG.careEffects). The lowest stat under the threshold determines
 * the pet's mood, which drives facial expression, idle behaviour and
 * the HUD mood pill. No rendering here — Luna reads `mood` each frame.
 */
import { CONFIG, CareAction, Mood, PetStats } from '../config';

const clamp = (v: number) => Math.max(0, Math.min(100, v));

export class MoodEngine {
  stats: PetStats = {
    hunger: 86,
    happiness: 90,
    cleanliness: 92,
    energy: 84,
    affection: 88,
  };
  bondXp = 0;

  /** Real-time decay. Call every frame with dt seconds. */
  update(dt: number): void {
    const d = CONFIG.statDecay;
    this.stats.hunger = clamp(this.stats.hunger - d.hunger * dt);
    this.stats.happiness = clamp(this.stats.happiness - d.happiness * dt);
    this.stats.cleanliness = clamp(this.stats.cleanliness - d.cleanliness * dt);
    this.stats.energy = clamp(this.stats.energy - d.energy * dt);
    this.stats.affection = clamp(this.stats.affection - d.affection * dt);
  }

  /** Apply a care action's effects. Returns the bond XP gained. */
  applyCare(action: CareAction): number {
    const e = CONFIG.careEffects[action];
    const s = this.stats;
    switch (action) {
      case 'feed':
        s.hunger = clamp(s.hunger + (e.hunger ?? 0));
        break;
      case 'play':
        s.happiness = clamp(s.happiness + (e.happiness ?? 0));
        s.energy = clamp(s.energy + (e.energy ?? 0));
        break;
      case 'clean':
        s.cleanliness = e.cleanliness ?? 100; // full refresh
        break;
      case 'rest':
        s.energy = clamp(s.energy + (e.energy ?? 0));
        break;
      case 'love':
        s.affection = clamp(s.affection + (e.affection ?? 0));
        s.happiness = clamp(s.happiness + (e.happiness ?? 0));
        break;
    }
    this.bondXp += e.bondXp;
    return e.bondXp;
  }

  get mood(): Mood {
    const s = this.stats;
    const entries: Array<[Mood, number]> = [
      ['hungry', s.hunger],
      ['sleepy', s.energy],
      ['dirty', s.cleanliness],
      ['lonely', s.affection],
      ['bored', s.happiness],
    ];
    let worst: Mood = 'happy';
    let worstVal = 70;
    for (const [m, v] of entries) {
      if (v < worstVal) {
        worstVal = v;
        worst = m;
      }
    }
    if (worstVal >= 70) return s.happiness >= 82 ? 'happy' : 'content';
    return worst;
  }
}
