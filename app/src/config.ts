/**
 * Global tuning constants for the SolanaPets game client.
 * All magic numbers for pacing, layout and feel live here.
 */

export type CareAction = 'feed' | 'play' | 'clean' | 'rest' | 'love';

export type Mood = 'happy' | 'content' | 'hungry' | 'sleepy' | 'dirty' | 'lonely' | 'bored';

export interface PetStats {
  hunger: number; // 100 = full
  happiness: number;
  cleanliness: number;
  energy: number;
  affection: number;
}

/** Per-action stat deltas. Every action also grants bondXp. */
export interface CareEffect {
  hunger?: number;
  happiness?: number;
  cleanliness?: number;
  energy?: number;
  affection?: number;
  bondXp: number;
}

interface Config {
  /** Full day/night cycle length in seconds (demo pacing). */
  dayLengthSec: number;
  /** Pet stat decay per second — slow enough for a demo, visible over minutes. */
  statDecay: PetStats;
  /** Care action effects. Values match the on-chain care program semantics. */
  careEffects: Record<CareAction, CareEffect>;
  /** Parallax strength in pixels at full pointer deflection. */
  parallaxStrength: number;
  /** Pet render scale range relative to min(viewport)/700. */
  petScaleMin: number;
  petScaleMax: number;
}

export const CONFIG: Config = {
  dayLengthSec: 240,

  statDecay: {
    hunger: 0.06,
    happiness: 0.045,
    cleanliness: 0.03,
    energy: 0.05,
    affection: 0.04,
  },

  careEffects: {
    feed: { hunger: 25, bondXp: 5 },
    play: { happiness: 20, energy: -10, bondXp: 10 },
    clean: { cleanliness: 100, bondXp: 5 }, // sets to full
    rest: { energy: 30, bondXp: 3 },
    love: { affection: 20, happiness: 8, bondXp: 15 },
  },

  parallaxStrength: 46,

  petScaleMin: 0.72,
  petScaleMax: 1.25,
};

export const MOOD_LABELS: Record<Mood, string> = {
  happy: 'Happy',
  content: 'Content',
  hungry: 'Hungry',
  sleepy: 'Sleepy',
  dirty: 'Needs a bath',
  lonely: 'Lonely',
  bored: 'Bored',
};
