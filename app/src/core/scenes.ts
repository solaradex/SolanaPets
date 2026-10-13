/**
 * Scene registry — which habitat + pet the app boots into.
 *
 * `?pet=lenny` in the URL selects Lenny's wild river (likewise `?pet=leo`
 * for Leo's aurora forest); anything else (or no param) defaults to Luna's
 * moonlit cove. Each new pet registers one entry:
 * a habitat factory and a pet factory. Game.ts consumes only the HabitatLike
 * and Pet interfaces, so adding a scene never touches game logic.
 */
import { Container } from 'pixi.js';
import { AuroraForest } from '../habitat/AuroraForest';
import { DayNightCycle } from '../habitat/DayNightCycle';
import { MoonlitCove } from '../habitat/MoonlitCove';
import { WildRiver } from '../habitat/WildRiver';
import { Leo } from '../pet/Leo';
import { Lenny } from '../pet/Lenny';
import { Luna } from '../pet/Luna';
import { Pet } from '../pet/PetSprite';

/** Structural contract every habitat satisfies (layout/update/anchors). */
export interface HabitatLike extends Container {
  layout(w: number, h: number): void;
  update(dt: number, cycle: DayNightCycle, px: number, py: number): void;
  getPetAnchor(): { x: number; y: number };
  getWaterBounds(): { x: number; y: number; w: number; h: number };
}

export interface SceneDef {
  key: string;
  displayName: string;
  petId: number;
  makeHabitat: () => HabitatLike;
  makePet: () => Pet;
}

export const SCENES: Record<string, SceneDef> = {
  luna: {
    key: 'luna',
    displayName: 'Luna',
    petId: 1,
    makeHabitat: () => new MoonlitCove(),
    makePet: () => new Luna(),
  },
  leo: {
    key: 'leo',
    displayName: 'Leo',
    petId: 6,
    makeHabitat: () => new AuroraForest(),
    makePet: () => new Leo(),
  },
  lenny: {
    key: 'lenny',
    displayName: 'Lenny',
    petId: 2,
    makeHabitat: () => new WildRiver(),
    makePet: () => new Lenny(),
  },
};

export function sceneFromUrl(): SceneDef {
  const q = new URLSearchParams(window.location.search).get('pet');
  const found = q ? SCENES[q.toLowerCase()] : undefined;
  return found ?? SCENES['luna']!;
}
