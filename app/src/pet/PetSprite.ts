/**
 * PetSprite — the contract for real pet art.
 *
 * When production sprite sheets arrive, they drop into
 * `assets/pets/<name>/` following the SpriteSheetSpec below and this
 * loader swaps the procedural pet for real art with ZERO changes to
 * game logic (Luna, MoodEngine, CareBar all talk to the Pet interface).
 *
 * Expected layout (example: Luna):
 *   assets/pets/luna/front_idle.png      — 4-frame strip
 *   assets/pets/luna/front_happy.png
 *   assets/pets/luna/side_idle.png
 *   assets/pets/luna/back_idle.png
 *   assets/pets/luna/expressions.png     — happy/curious/playful/serene faces
 *   assets/pets/luna.json                — frame rects + anchors (TexturePacker format)
 */
import { Container } from 'pixi.js';
import { Mood } from '../config';

export type PetView = 'front' | 'side' | 'back' | 'three-quarter';
export type PetExpression = 'happy' | 'curious' | 'playful' | 'serene';

export interface SpriteSheetSpec {
  petName: string;
  views: PetView[];
  expressions: PetExpression[];
  /** Frames per animation strip, in order. */
  frames: Record<string, number>;
  /** Pixels per frame, assumed uniform within a sheet. */
  frameSize: { w: number; h: number };
}

/** Minimal interface every pet (procedural or sprite-based) must satisfy. */
export interface Pet {
  readonly container: Container;
  update(dt: number, time: number): void;
  reactTo(action: 'feed' | 'play' | 'clean' | 'rest' | 'love'): void;
  setMood(mood: Mood): void;
}

/**
 * Attempts to load real sprite art; resolves null when absent so the
 * caller falls back to the procedural pet. Not wired to any UI yet —
 * it is the documented seam for the art pipeline.
 */
export async function loadPetSprite(_petName: string): Promise<Pet | null> {
  // TODO(art): fetch assets/pets/<name>.json, build AnimatedSprites per view.
  return null;
}

export const LUNA_SHEET_SPEC: SpriteSheetSpec = {
  petName: 'luna',
  views: ['front', 'side', 'back', 'three-quarter'],
  expressions: ['happy', 'curious', 'playful', 'serene'],
  frames: {
    idle: 4,
    walk: 6,
    swim: 6,
    happy: 4,
    eat: 4,
    sleep: 2,
  },
  frameSize: { w: 256, h: 256 },
};
