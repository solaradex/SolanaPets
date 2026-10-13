/**
 * Day/night cycle.
 *
 * Advances a 0..1 phase over CONFIG.dayLengthSec and lerps between
 * keyframed palettes. Habitats read `palette` every frame and restyle
 * their layers (sky gradient, star/moon alpha, water tint, silhouette
 * color, firefly visibility). Pure logic — no PixiJS dependency.
 */

export interface Palette {
  skyTop: number;
  skyBottom: number;
  starAlpha: number;
  moonAlpha: number;
  waterDeep: number;
  waterShallow: number;
  waterGlow: number;
  ridgeFar: number;
  ridgeNear: number;
  ambient: number; // overall light 0..1
  fireflyAlpha: number;
}

interface Key {
  t: number;
  p: Palette;
}

const NIGHT: Palette = {
  skyTop: 0x060a1c,
  skyBottom: 0x16294d,
  starAlpha: 1,
  moonAlpha: 1,
  waterDeep: 0x081426,
  waterShallow: 0x12304f,
  waterGlow: 0x7fd4ff,
  ridgeFar: 0x0c1428,
  ridgeNear: 0x111c33,
  ambient: 0.62,
  fireflyAlpha: 1,
};

const DAWN: Palette = {
  skyTop: 0x3a2350,
  skyBottom: 0xe08a6d,
  starAlpha: 0.12,
  moonAlpha: 0.22,
  waterDeep: 0x14304d,
  waterShallow: 0x3d5a7a,
  waterGlow: 0xffc9a3,
  ridgeFar: 0x1c2038,
  ridgeNear: 0x27304a,
  ambient: 0.85,
  fireflyAlpha: 0.25,
};

const DAY: Palette = {
  skyTop: 0x3d6ea5,
  skyBottom: 0xc4e6f2,
  starAlpha: 0,
  moonAlpha: 0.05,
  waterDeep: 0x1d5f8a,
  waterShallow: 0x5fa8c9,
  waterGlow: 0xfff3c4,
  ridgeFar: 0x2b4a6b,
  ridgeNear: 0x3a5c7d,
  ambient: 1,
  fireflyAlpha: 0,
};

const DUSK: Palette = {
  skyTop: 0x2b1a4d,
  skyBottom: 0xf08a4b,
  starAlpha: 0.35,
  moonAlpha: 0.5,
  waterDeep: 0x10294d,
  waterShallow: 0x4a3a5e,
  waterGlow: 0xffb37a,
  ridgeFar: 0x161c34,
  ridgeNear: 0x232c48,
  ambient: 0.74,
  fireflyAlpha: 0.6,
};

const KEYS: Key[] = [
  { t: 0.0, p: NIGHT },
  { t: 0.16, p: DAWN },
  { t: 0.3, p: DAY },
  { t: 0.62, p: DAY },
  { t: 0.78, p: DUSK },
  { t: 0.92, p: NIGHT },
  { t: 1.0, p: NIGHT },
];

function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

function lerpColor(a: number, b: number, t: number): number {
  const ar = (a >> 16) & 255;
  const ag = (a >> 8) & 255;
  const ab = a & 255;
  const br = (b >> 16) & 255;
  const bg = (b >> 8) & 255;
  const bb = b & 255;
  const r = Math.round(lerp(ar, br, t));
  const g = Math.round(lerp(ag, bg, t));
  const bl = Math.round(lerp(ab, bb, t));
  return (r << 16) | (g << 8) | bl;
}

function lerpPalette(a: Palette, b: Palette, t: number): Palette {
  return {
    skyTop: lerpColor(a.skyTop, b.skyTop, t),
    skyBottom: lerpColor(a.skyBottom, b.skyBottom, t),
    starAlpha: lerp(a.starAlpha, b.starAlpha, t),
    moonAlpha: lerp(a.moonAlpha, b.moonAlpha, t),
    waterDeep: lerpColor(a.waterDeep, b.waterDeep, t),
    waterShallow: lerpColor(a.waterShallow, b.waterShallow, t),
    waterGlow: lerpColor(a.waterGlow, b.waterGlow, t),
    ridgeFar: lerpColor(a.ridgeFar, b.ridgeFar, t),
    ridgeNear: lerpColor(a.ridgeNear, b.ridgeNear, t),
    ambient: lerp(a.ambient, b.ambient, t),
    fireflyAlpha: lerp(a.fireflyAlpha, b.fireflyAlpha, t),
  };
}

export class DayNightCycle {
  /** Phase 0..1. Starts at night so the moonlit cove opens in its hero state. */
  phase = 0.02;
  readonly lengthSec: number;
  palette: Palette = NIGHT;

  constructor(lengthSec: number) {
    this.lengthSec = lengthSec;
    this.refresh();
  }

  update(dt: number): void {
    this.phase = (this.phase + dt / this.lengthSec) % 1;
    this.refresh();
  }

  private refresh(): void {
    let i = 0;
    while (i < KEYS.length - 2 && KEYS[i + 1]!.t <= this.phase) i++;
    const a = KEYS[i]!;
    const b = KEYS[i + 1]!;
    const t = (this.phase - a.t) / Math.max(1e-6, b.t - a.t);
    this.palette = lerpPalette(a.p, b.p, Math.min(1, Math.max(0, t)));
  }

  phaseName(): string {
    const p = this.phase;
    if (p < 0.16 || p >= 0.92) return 'Night';
    if (p < 0.3) return 'Dawn';
    if (p < 0.62) return 'Day';
    return 'Dusk';
  }
}
