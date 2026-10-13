/**
 * MoonlitCove — Luna's habitat.
 *
 * The visual benchmark for all SolanaPets habitats. A 2.5D scene built from
 * five parallax layers:
 *
 *   sky   (0.04) — gradient sky, twinkling stars, glowing moon
 *   far   (0.10) — distant mountain silhouette ridge
 *   mid   (0.20) — nearer cliffs, pine silhouettes, waterfall cliff
 *   water (0.30) — animated shimmering water, moonlight path, waterfall foam
 *   fore  (0.45) — foreground rocks (Luna's perch)
 *
 * Everything is procedural PixiJS (gradients, glows, particles) so the
 * scene is fully art-directable from the day/night palette. When real
 * habitat art arrives it slots into these same layers — see HabitatSpec
 * in README.
 */
import { Container, Graphics, Sprite, Texture } from 'pixi.js';
import { CONFIG } from '../config';
import { css, gradientTexture, radialGlowTexture, softDotTexture } from '../core/textures';
import { DayNightCycle, Palette } from './DayNightCycle';

interface Star {
  sprite: Sprite;
  phase: number;
  speed: number;
}

interface Streak {
  sprite: Sprite;
  speed: number;
  baseAlpha: number;
  phase: number;
}

interface Firefly {
  sprite: Sprite;
  bx: number;
  by: number;
  phase: number;
  speed: number;
  amp: number;
}

interface Mist {
  sprite: Sprite;
  speed: number;
}

interface FoamDot {
  sprite: Sprite;
  t: number;
  dur: number;
  x0: number;
}

const OVERSCAN = 90; // extra draw width so parallax never reveals edges

export class MoonlitCove extends Container {
  private skyLayer = new Container();
  private farLayer = new Container();
  private midLayer = new Container();
  private waterLayer = new Container();
  private foreLayer = new Container();

  private skySprite!: Sprite;
  private skyTex: Texture | null = null;
  private stars: Star[] = [];
  private moon!: Container;
  private moonGlow!: Sprite;
  private moonDisc!: Graphics;

  private farRidge!: Graphics;
  private nearRidge!: Graphics;
  private trees!: Graphics;
  private cliff!: Graphics;

  private waterBase!: Sprite;
  private waterTex: Texture | null = null;
  private moonPath!: Sprite;
  private streaks: Streak[] = [];

  private fallStrips: Sprite[] = [];
  private waterfallX = 0;
  private waterfallBaseY = 0;
  private foam: FoamDot[] = [];

  private mists: Mist[] = [];
  private fireflies: Firefly[] = [];
  private rocks!: Graphics;
  private perch!: Graphics;

  private w = 0;
  private h = 0;
  private horizonY = 0;
  private elapsed = 0;
  private restyleTimer = 0;
  private lastStyleKey = '';
  private petAnchor = { x: 0, y: 0 };

  constructor() {
    super();
    this.addChild(this.skyLayer, this.farLayer, this.midLayer, this.waterLayer, this.foreLayer);
    this.buildSky();
    this.buildRidges();
    this.buildWater();
    this.buildWaterfall();
    this.buildAtmosphere();
    this.buildForeground();
  }

  // ---------------------------------------------------------------- build

  private buildSky(): void {
    this.skySprite = new Sprite();
    this.skyLayer.addChild(this.skySprite);

    const dot = softDotTexture(32);
    for (let i = 0; i < 110; i++) {
      const s = new Sprite(dot);
      s.anchor.set(0.5);
      s.tint = 0xffffff;
      s.blendMode = 'add';
      const sc = 1 + Math.random() * 2.4;
      s.scale.set(sc / 32);
      this.skyLayer.addChild(s);
      this.stars.push({ sprite: s, phase: Math.random() * Math.PI * 2, speed: 0.6 + Math.random() * 2.2 });
    }

    this.moon = new Container();
    this.moonGlow = new Sprite(radialGlowTexture('rgba(244,241,222,0.55)'));
    this.moonGlow.anchor.set(0.5);
    this.moonGlow.blendMode = 'add';
    this.moonDisc = new Graphics();
    this.moon.addChild(this.moonGlow, this.moonDisc);
    this.skyLayer.addChild(this.moon);
  }

  private buildRidges(): void {
    this.farRidge = new Graphics();
    this.farLayer.addChild(this.farRidge);
    this.nearRidge = new Graphics();
    this.midLayer.addChild(this.nearRidge);
    this.trees = new Graphics();
    this.midLayer.addChild(this.trees);
    this.cliff = new Graphics();
    this.midLayer.addChild(this.cliff);
  }

  private buildWater(): void {
    this.waterBase = new Sprite();
    this.waterLayer.addChild(this.waterBase);

    this.moonPath = new Sprite(gradientTexture([
      { offset: 0, color: 'rgba(255,255,255,0.85)' },
      { offset: 1, color: 'rgba(255,255,255,0)' },
    ], 8, 256));
    this.moonPath.anchor.set(0.5, 0);
    this.moonPath.blendMode = 'add';
    this.waterLayer.addChild(this.moonPath);

    const dot = softDotTexture(32);
    for (let i = 0; i < 26; i++) {
      const s = new Sprite(dot);
      s.anchor.set(0.5);
      s.blendMode = 'add';
      this.waterLayer.addChild(s);
      this.streaks.push({
        sprite: s,
        speed: 12 + Math.random() * 30,
        baseAlpha: 0.12 + Math.random() * 0.3,
        phase: Math.random() * Math.PI * 2,
      });
    }
  }

  private buildWaterfall(): void {
    for (let i = 0; i < 3; i++) {
      const strip = new Sprite(gradientTexture([
        { offset: 0, color: 'rgba(255,255,255,0)' },
        { offset: 0.25, color: 'rgba(220,240,255,0.75)' },
        { offset: 1, color: 'rgba(200,230,255,0.15)' },
      ], 16, 256));
      strip.anchor.set(0.5, 0);
      strip.blendMode = 'add';
      this.midLayer.addChild(strip);
      this.fallStrips.push(strip);
    }
    const dot = softDotTexture(32);
    for (let i = 0; i < 10; i++) {
      const s = new Sprite(dot);
      s.anchor.set(0.5);
      s.tint = 0xdff2ff;
      s.blendMode = 'add';
      this.waterLayer.addChild(s);
      this.foam.push({ sprite: s, t: Math.random(), dur: 0.9 + Math.random() * 0.7, x0: 0 });
    }
  }

  private buildAtmosphere(): void {
    const dot = softDotTexture(128);
    for (let i = 0; i < 6; i++) {
      const s = new Sprite(dot);
      s.anchor.set(0.5);
      s.tint = 0x9db8d9;
      s.alpha = 0.07;
      this.midLayer.addChild(s);
      this.mists.push({ sprite: s, speed: 4 + Math.random() * 8 });
    }
    const glow = radialGlowTexture('rgba(190,255,200,0.9)', 64);
    for (let i = 0; i < 14; i++) {
      const s = new Sprite(glow);
      s.anchor.set(0.5);
      s.blendMode = 'add';
      const sc = 0.25 + Math.random() * 0.3;
      s.scale.set(sc);
      this.foreLayer.addChild(s);
      this.fireflies.push({
        sprite: s,
        bx: Math.random(), by: Math.random(),
        phase: Math.random() * Math.PI * 2,
        speed: 0.4 + Math.random() * 0.8,
        amp: 14 + Math.random() * 26,
      });
    }
  }

  private buildForeground(): void {
    this.rocks = new Graphics();
    this.foreLayer.addChild(this.rocks);
    this.perch = new Graphics();
    this.foreLayer.addChild(this.perch);
  }

  // --------------------------------------------------------------- layout

  /** Deterministic pseudo-random for stable ridge shapes across resizes. */
  private lcg(seed: number): () => number {
    let s = seed >>> 0;
    return () => {
      s = (s * 1664525 + 1013904223) >>> 0;
      return s / 4294967296;
    };
  }

  layout(w: number, h: number): void {
    this.w = w;
    this.h = h;
    this.horizonY = h * 0.5;
    const W = w + OVERSCAN * 2;

    this.skySprite.position.set(-OVERSCAN, 0);
    this.skySprite.width = W;
    this.skySprite.height = h;

    const moonR = Math.min(w, h) * 0.075;
    this.moon.position.set(w * 0.24, h * 0.2);
    this.moonGlow.scale.set((moonR * 7) / 256);
    this.moonDisc.clear();
    this.moonDisc.circle(0, 0, moonR).fill(0xf4f1de);
    const rnd = this.lcg(7);
    for (let i = 0; i < 4; i++) {
      const cr = moonR * (0.12 + rnd() * 0.14);
      this.moonDisc
        .circle((rnd() - 0.5) * moonR, (rnd() - 0.5) * moonR, cr)
        .fill({ color: 0xd8d4bd, alpha: 0.5 });
    }

    for (const st of this.stars) {
      st.sprite.position.set(Math.random() * w, Math.random() * this.horizonY * 0.95);
    }

    this.drawRidge(this.farRidge, this.horizonY, h * 0.1, 5, this.lcg(21));
    this.drawRidge(this.nearRidge, this.horizonY + 6, h * 0.07, 7, this.lcg(99));
    this.drawTrees();

    // waterfall cliff on the right
    this.waterfallX = w * 0.8;
    this.waterfallBaseY = this.horizonY + (h - this.horizonY) * 0.42;
    const cw = Math.min(w, h) * 0.16;
    this.cliff.clear();
    this.cliff
      .poly([
        this.waterfallX - cw, this.horizonY - h * 0.06,
        this.waterfallX + cw, this.horizonY - h * 0.1,
        this.waterfallX + cw * 0.9, this.waterfallBaseY,
        this.waterfallX - cw * 0.9, this.waterfallBaseY,
      ])
      .fill(0x0e1626);
    const stripW = cw * 0.32;
    this.fallStrips.forEach((s, i) => {
      s.position.set(this.waterfallX + (i - 1) * stripW * 0.7, this.horizonY - h * 0.08);
      s.width = stripW * (0.7 + i * 0.2);
      s.height = this.waterfallBaseY - (this.horizonY - h * 0.08);
    });

    this.waterBase.position.set(-OVERSCAN, this.horizonY);
    this.waterBase.width = W;
    this.waterBase.height = h - this.horizonY;

    this.moonPath.position.set(this.moon.position.x, this.horizonY + 4);
    this.moonPath.width = w * 0.13;
    this.moonPath.height = (h - this.horizonY) * 0.96;

    this.streaks.forEach((st, i) => {
      const r = this.lcg(300 + i)();
      st.sprite.position.set(r * w, this.horizonY + 12 + r * (h - this.horizonY - 30));
      st.sprite.scale.set((30 + r * 90) / 32, (3 + r * 4) / 32);
    });

    this.mists.forEach((m, i) => {
      m.sprite.position.set((i / 6) * w, this.horizonY + 20 + (i % 3) * 26);
      const ms = Math.min(w, h) * (0.5 + (i % 3) * 0.25);
      m.sprite.scale.set(ms / 128);
    });

    this.fireflies.forEach((f) => {
      f.sprite.position.set(f.bx * w, this.horizonY * 0.35 + f.by * (h * 0.4));
    });

    // foreground rocks + Luna's perch
    const rockY = h * 0.86;
    this.rocks.clear();
    this.rocks.ellipse(w * 0.08, rockY + 40, w * 0.22, h * 0.1).fill(0x080d18);
    this.rocks.ellipse(w * 0.94, rockY + 52, w * 0.2, h * 0.12).fill(0x080d18);

    const perchW = Math.min(w, h) * 0.42;
    const perchTop = h * 0.68;
    this.perch.clear();
    this.perch
      .poly([
        w * 0.5 - perchW / 2, perchTop,
        w * 0.5 + perchW / 2, perchTop,
        w * 0.5 + perchW * 0.32, perchTop + h * 0.2,
        w * 0.5 - perchW * 0.32, perchTop + h * 0.2,
      ])
      .fill(0x131c30);
    this.perch
      .ellipse(w * 0.5, perchTop, perchW / 2, h * 0.022)
      .fill(0x1d2a45);
    // moonlit rim on the perch
    this.perch
      .ellipse(w * 0.5 - perchW * 0.08, perchTop - 2, perchW * 0.42, h * 0.012)
      .fill({ color: 0x9fc8e8, alpha: 0.35 });

    this.petAnchor = { x: w * 0.5, y: perchTop - 6 };

    this.lastStyleKey = ''; // force restyle
    this.restyleTimer = 1;
  }

  private drawRidge(g: Graphics, baseY: number, amp: number, peaks: number, rnd: () => number): void {
    const w = this.w + OVERSCAN * 2;
    const pts: number[] = [-OVERSCAN, baseY + amp * 0.4];
    for (let i = 0; i <= peaks; i++) {
      const x = -OVERSCAN + (w * i) / peaks;
      const y = baseY - rnd() * amp;
      pts.push(x, y);
      pts.push(x + w / peaks / 2, baseY - rnd() * amp * 0.4);
    }
    pts.push(this.w + OVERSCAN, baseY + amp * 0.4);
    pts.push(this.w + OVERSCAN, this.h);
    pts.push(-OVERSCAN, this.h);
    g.clear();
    g.poly(pts).fill(0x0c1428); // tinted per-frame via restyle
  }

  private drawTrees(): void {
    const g = this.trees;
    g.clear();
    const rnd = this.lcg(55);
    const baseY = this.horizonY + 8;
    for (let i = 0; i < 7; i++) {
      const x = rnd() * this.w;
      const th = this.h * (0.05 + rnd() * 0.05);
      const tw = th * 0.42;
      g.rect(x - th * 0.03, baseY - th * 0.25, th * 0.06, th * 0.25).fill(0x0a1220);
      for (let t = 0; t < 3; t++) {
        const ty = baseY - th * 0.2 - t * th * 0.26;
        const tww = tw * (1 - t * 0.24);
        g.poly([x - tww / 2, ty, x + tww / 2, ty, x, ty - th * 0.34]).fill(0x0a1220);
      }
    }
  }

  // --------------------------------------------------------------- update

  update(dt: number, cycle: DayNightCycle, px: number, py: number): void {
    this.elapsed += dt;
    const t = this.elapsed;
    const pal = cycle.palette;
    const strength = CONFIG.parallaxStrength;

    // idle drift so the scene breathes even without input
    const driftX = Math.sin(t * 0.12) * 5;

    this.skyLayer.position.set(-px * strength * 0.04 + driftX * 0.3, -py * strength * 0.02);
    this.farLayer.position.set(-px * strength * 0.1 + driftX * 0.5, -py * strength * 0.05);
    this.midLayer.position.set(-px * strength * 0.2 + driftX * 0.8, -py * strength * 0.1);
    this.waterLayer.position.set(-px * strength * 0.3 + driftX, -py * strength * 0.14);
    this.foreLayer.position.set(-px * strength * 0.45 + driftX * 1.3, -py * strength * 0.2);

    // throttled palette restyle (cheap enough at ~2.5Hz)
    this.restyleTimer += dt;
    if (this.restyleTimer > 0.4) {
      this.restyleTimer = 0;
      this.restyle(pal);
    }

    // stars twinkle
    for (const st of this.stars) {
      st.sprite.alpha = pal.starAlpha * (0.35 + 0.65 * (0.5 + 0.5 * Math.sin(t * st.speed + st.phase)));
    }
    this.moon.alpha = pal.moonAlpha;
    this.moonGlow.alpha = 0.75 + 0.25 * Math.sin(t * 0.8);

    // water shimmer
    for (const st of this.streaks) {
      const s = st.sprite;
      s.position.x += st.speed * dt;
      if (s.position.x > this.w + 40) s.position.x = -40;
      s.alpha = st.baseAlpha * (0.6 + 0.4 * Math.sin(t * 2 + st.phase)) * (0.4 + pal.ambient * 0.6);
    }
    this.moonPath.alpha = 0.5 * pal.moonAlpha * (0.75 + 0.25 * Math.sin(t * 1.7));
    this.moonPath.scale.x = 1 + 0.08 * Math.sin(t * 0.9);

    // waterfall
    this.fallStrips.forEach((s, i) => {
      s.alpha = 0.4 + 0.18 * Math.sin(t * 7 + i * 2.1);
      s.position.x += Math.sin(t * 3 + i) * dt * 6;
    });
    for (const f of this.foam) {
      f.t += dt;
      if (f.t >= f.dur) {
        f.t = 0;
        f.dur = 0.9 + Math.random() * 0.7;
        f.x0 = this.waterfallX + (Math.random() - 0.5) * 46 - this.waterLayer.position.x;
      }
      const k = f.t / f.dur;
      f.sprite.position.set(f.x0, this.waterfallBaseY - this.waterLayer.position.y - k * 26);
      const sc = (6 + k * 16) / 32;
      f.sprite.scale.set(sc);
      f.sprite.alpha = 0.55 * (1 - k);
    }

    // mist drift
    for (const m of this.mists) {
      m.sprite.position.x += m.speed * dt;
      if (m.sprite.position.x > this.w + 120) m.sprite.position.x = -120;
    }

    // fireflies
    for (const f of this.fireflies) {
      const s = f.sprite;
      s.position.x += Math.sin(t * f.speed + f.phase) * f.amp * dt;
      s.position.y += Math.cos(t * f.speed * 0.7 + f.phase * 1.3) * f.amp * 0.6 * dt;
      s.alpha = pal.fireflyAlpha * (0.25 + 0.75 * (0.5 + 0.5 * Math.sin(t * 2.4 + f.phase * 3)));
    }
  }

  private restyle(pal: Palette): void {
    const key = [pal.skyTop, pal.skyBottom, pal.waterDeep, pal.waterShallow, pal.ridgeFar, pal.ridgeNear]
      .map((c) => c.toString(16))
      .join('|');
    if (key === this.lastStyleKey) return;
    this.lastStyleKey = key;

    this.skyTex?.destroy();
    this.skyTex = gradientTexture(
      [
        { offset: 0, color: css(pal.skyTop) },
        { offset: 0.62, color: css(pal.skyBottom) },
        { offset: 1, color: css(pal.skyBottom) },
      ],
      4,
      512,
    );
    this.skySprite.texture = this.skyTex;

    this.waterTex?.destroy();
    this.waterTex = gradientTexture(
      [
        { offset: 0, color: css(pal.waterShallow) },
        { offset: 0.25, color: css(pal.waterDeep) },
        { offset: 1, color: css(0x05080f) },
      ],
      4,
      256,
    );
    this.waterBase.texture = this.waterTex;

    // ridge tints (geometry drawn once, recoloured here)
    this.farRidge.tint = pal.ridgeFar;
    this.nearRidge.tint = pal.ridgeNear;
    this.trees.tint = pal.ridgeNear;
    this.cliff.tint = pal.ridgeNear;
    this.rocks.tint = 0xffffff;
    this.perch.tint = 0xffffff;

    this.moonPath.tint = pal.waterGlow;
    for (const st of this.streaks) st.sprite.tint = pal.waterGlow;

    const dim = 0.55 + pal.ambient * 0.45;
    this.midLayer.alpha = dim;
    this.farLayer.alpha = 0.5 + pal.ambient * 0.5;
  }

  // --------------------------------------------------------------- accessors

  getPetAnchor(): { x: number; y: number } {
    return this.petAnchor;
  }

  /** Water rect in screen coords — for ambient glint particles. */
  getWaterBounds(): { x: number; y: number; w: number; h: number } {
    return { x: 0, y: this.horizonY, w: this.w, h: this.h - this.horizonY };
  }

  getWaterfallBase(): { x: number; y: number } {
    return { x: this.waterfallX, y: this.waterfallBaseY };
  }
}
