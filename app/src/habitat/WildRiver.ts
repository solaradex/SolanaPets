/**
 * WildRiver — Lenny's habitat.
 *
 * "The River Explorer" lives where the water runs fast: a wild river
 * cutting through pine forest, opening at dawn. Same five-layer 2.5D
 * parallax architecture as MoonlitCove (the visual benchmark):
 *
 *   sky   (0.04) — dawn gradient, rising sun, god rays, fading stars
 *   far   (0.10) — distant mountain ridge
 *   mid   (0.20) — pine forests on both banks, rocky outcrops
 *   water (0.30) — rushing river, sunlit path, rapids foam, drift leaves
 *   fore  (0.45) — riverbank boulders (Lenny's perch)
 *
 * Adventure-toned: the shared day/night palette is lifted warmer and
 * brighter here than the cove's deep night, so even midnight on the
 * river feels like an explorer's camp, not a moonlit grotto.
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

interface Leaf {
  sprite: Sprite;
  speed: number;
  phase: number;
  spin: number;
  baseY: number;
}

interface Mote {
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

interface RapidDot {
  sprite: Sprite;
  t: number;
  dur: number;
  dx: number;
  dy: number;
}

interface Rapid {
  fx: number;
  fy: number;
  x: number;
  y: number;
  dots: RapidDot[];
}

const OVERSCAN = 90; // extra draw width so parallax never reveals edges

function lerpChannel(a: number, b: number, t: number): number {
  return Math.round(a + (b - a) * t);
}

/** Lift a color toward a warm tone — the river's adventure grade. */
function liftColor(c: number, t: number): number {
  const r = lerpChannel((c >> 16) & 255, 0x7a, t);
  const g = lerpChannel((c >> 8) & 255, 0x6a, t);
  const b = lerpChannel(c & 255, 0x58, t);
  return (r << 16) | (g << 8) | b;
}

export class WildRiver extends Container {
  private skyLayer = new Container();
  private farLayer = new Container();
  private midLayer = new Container();
  private waterLayer = new Container();
  private foreLayer = new Container();

  private skySprite!: Sprite;
  private skyTex: Texture | null = null;
  private stars: Star[] = [];
  private sun!: Container;
  private sunGlow!: Sprite;
  private sunDisc!: Graphics;
  private rays: Sprite[] = [];

  private farRidge!: Graphics;
  private nearRidge!: Graphics;
  private pines!: Graphics;
  private outcrops!: Graphics;

  private waterBase!: Sprite;
  private waterTex: Texture | null = null;
  private sunPath!: Sprite;
  private streaks: Streak[] = [];
  private rapids: Rapid[] = [];

  private mists: Mist[] = [];
  private leaves: Leaf[] = [];
  private motes: Mote[] = [];
  private banks!: Graphics;
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
    this.buildRapids();
    this.buildAtmosphere();
    this.buildForeground();
  }

  // ---------------------------------------------------------------- build

  private buildSky(): void {
    this.skySprite = new Sprite();
    this.skyLayer.addChild(this.skySprite);

    // a few faint stars — mostly a dawn/day sky
    const dot = softDotTexture(32);
    for (let i = 0; i < 30; i++) {
      const s = new Sprite(dot);
      s.anchor.set(0.5);
      s.tint = 0xffffff;
      s.blendMode = 'add';
      const sc = 1 + Math.random() * 2;
      s.scale.set(sc / 32);
      this.skyLayer.addChild(s);
      this.stars.push({ sprite: s, phase: Math.random() * Math.PI * 2, speed: 0.6 + Math.random() * 2 });
    }

    // rising sun
    this.sun = new Container();
    this.sunGlow = new Sprite(radialGlowTexture('rgba(255,214,150,0.6)'));
    this.sunGlow.anchor.set(0.5);
    this.sunGlow.blendMode = 'add';
    this.sunDisc = new Graphics();
    this.sun.addChild(this.sunGlow, this.sunDisc);
    this.skyLayer.addChild(this.sun);

    // god rays through the pines
    const rayTex = gradientTexture(
      [
        { offset: 0, color: 'rgba(255,230,170,0.5)' },
        { offset: 1, color: 'rgba(255,230,170,0)' },
      ],
      24,
      256,
    );
    for (let i = 0; i < 2; i++) {
      const ray = new Sprite(rayTex);
      ray.anchor.set(0.5, 0);
      ray.blendMode = 'add';
      ray.rotation = 0.35 + i * 0.22;
      this.skyLayer.addChild(ray);
      this.rays.push(ray);
    }
  }

  private buildRidges(): void {
    this.farRidge = new Graphics();
    this.farLayer.addChild(this.farRidge);
    this.nearRidge = new Graphics();
    this.midLayer.addChild(this.nearRidge);
    this.pines = new Graphics();
    this.midLayer.addChild(this.pines);
    this.outcrops = new Graphics();
    this.midLayer.addChild(this.outcrops);
  }

  private buildWater(): void {
    this.waterBase = new Sprite();
    this.waterLayer.addChild(this.waterBase);

    // sunlit path on rushing water
    this.sunPath = new Sprite(
      gradientTexture(
        [
          { offset: 0, color: 'rgba(255,255,255,0.9)' },
          { offset: 1, color: 'rgba(255,255,255,0)' },
        ],
        8,
        256,
      ),
    );
    this.sunPath.anchor.set(0.5, 0);
    this.sunPath.blendMode = 'add';
    this.waterLayer.addChild(this.sunPath);

    // fast current streaks — the river runs quicker than the cove
    const dot = softDotTexture(32);
    for (let i = 0; i < 34; i++) {
      const s = new Sprite(dot);
      s.anchor.set(0.5);
      s.blendMode = 'add';
      this.waterLayer.addChild(s);
      this.streaks.push({
        sprite: s,
        speed: 46 + Math.random() * 70,
        baseAlpha: 0.14 + Math.random() * 0.3,
        phase: Math.random() * Math.PI * 2,
      });
    }
  }

  private buildRapids(): void {
    const dot = softDotTexture(32);
    const spots: Array<[number, number]> = [
      [0.3, 0.62],
      [0.62, 0.74],
      [0.45, 0.86],
    ];
    for (const [fx, fy] of spots) {
      const dots: RapidDot[] = [];
      for (let i = 0; i < 8; i++) {
        const s = new Sprite(dot);
        s.anchor.set(0.5);
        s.tint = 0xeaf6ff;
        s.blendMode = 'add';
        this.waterLayer.addChild(s);
        dots.push({
          sprite: s,
          t: Math.random(),
          dur: 0.6 + Math.random() * 0.6,
          dx: (Math.random() - 0.5) * 70,
          dy: (Math.random() - 0.5) * 26,
        });
      }
      this.rapids.push({ fx, fy, x: 0, y: 0, dots });
    }
  }

  private buildAtmosphere(): void {
    // morning mist — thicker than the cove's
    const dot = softDotTexture(128);
    for (let i = 0; i < 8; i++) {
      const s = new Sprite(dot);
      s.anchor.set(0.5);
      s.tint = 0xcfd8e6;
      s.alpha = 0.1;
      this.midLayer.addChild(s);
      this.mists.push({ sprite: s, speed: 6 + Math.random() * 10 });
    }
    // drifting leaves on the current
    const leafDot = softDotTexture(32);
    for (let i = 0; i < 14; i++) {
      const s = new Sprite(leafDot);
      s.anchor.set(0.5);
      s.tint = [0x7fae5a, 0xa8bf6a, 0xc9a86a][i % 3]!;
      s.scale.set((9 + Math.random() * 7) / 32, (5 + Math.random() * 4) / 32);
      s.rotation = Math.random() * Math.PI;
      this.waterLayer.addChild(s);
      this.leaves.push({
        sprite: s,
        speed: 14 + Math.random() * 26,
        phase: Math.random() * Math.PI * 2,
        spin: (Math.random() - 0.5) * 1.4,
        baseY: 0,
      });
    }
    // light motes in the sunbeams
    const glow = radialGlowTexture('rgba(255,236,190,0.9)', 64);
    for (let i = 0; i < 10; i++) {
      const s = new Sprite(glow);
      s.anchor.set(0.5);
      s.blendMode = 'add';
      s.scale.set(0.2 + Math.random() * 0.25);
      this.foreLayer.addChild(s);
      this.motes.push({
        sprite: s,
        bx: Math.random(),
        by: Math.random(),
        phase: Math.random() * Math.PI * 2,
        speed: 0.3 + Math.random() * 0.6,
        amp: 12 + Math.random() * 22,
      });
    }
  }

  private buildForeground(): void {
    this.banks = new Graphics();
    this.foreLayer.addChild(this.banks);
    this.perch = new Graphics();
    this.foreLayer.addChild(this.perch);
  }

  // --------------------------------------------------------------- layout

  /** Deterministic pseudo-random for stable shapes across resizes. */
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
    this.horizonY = h * 0.46;
    const W = w + OVERSCAN * 2;

    this.skySprite.position.set(-OVERSCAN, 0);
    this.skySprite.width = W;
    this.skySprite.height = h;

    const sunR = Math.min(w, h) * 0.06;
    this.sun.position.set(w * 0.68, h * 0.22);
    this.sunGlow.scale.set((sunR * 8) / 256);
    this.sunDisc.clear();
    this.sunDisc.circle(0, 0, sunR).fill(0xffe9b8);

    for (const st of this.stars) {
      st.sprite.position.set(Math.random() * w, Math.random() * this.horizonY * 0.9);
    }

    this.rays.forEach((r, i) => {
      r.position.set(w * (0.6 + i * 0.12), h * 0.16);
      r.width = w * 0.05;
      r.height = h * 0.5;
    });

    this.drawRidge(this.farRidge, this.horizonY, h * 0.09, 5, this.lcg(31));
    this.drawRidge(this.nearRidge, this.horizonY + 8, h * 0.06, 6, this.lcg(77));
    this.drawPines();
    this.drawOutcrops();

    this.waterBase.position.set(-OVERSCAN, this.horizonY);
    this.waterBase.width = W;
    this.waterBase.height = h - this.horizonY;

    this.sunPath.position.set(this.sun.position.x, this.horizonY + 4);
    this.sunPath.width = w * 0.16;
    this.sunPath.height = (h - this.horizonY) * 0.96;

    this.streaks.forEach((st, i) => {
      const r = this.lcg(400 + i)();
      st.sprite.position.set(r * w, this.horizonY + 10 + r * (h - this.horizonY - 24));
      st.sprite.scale.set((40 + r * 110) / 32, (3 + r * 4) / 32);
    });

    this.rapids.forEach((rp) => {
      rp.x = rp.fx * w;
      rp.y = rp.fy * h;
    });

    this.mists.forEach((m, i) => {
      m.sprite.position.set((i / 8) * w, this.horizonY + 16 + (i % 4) * 22);
      const ms = Math.min(w, h) * (0.55 + (i % 4) * 0.22);
      m.sprite.scale.set(ms / 128);
    });

    this.leaves.forEach((l, i) => {
      const r = this.lcg(900 + i)();
      l.sprite.position.set(r * w, this.horizonY + 14 + r * (h - this.horizonY - 30));
      l.baseY = l.sprite.position.y;
    });

    this.motes.forEach((mo) => {
      mo.sprite.position.set(mo.bx * w, this.horizonY * 0.3 + mo.by * h * 0.45);
    });

    // riverbank boulders left & right, river runs through the middle
    const bankY = h * 0.88;
    this.banks.clear();
    this.banks.ellipse(w * 0.06, bankY + 46, w * 0.24, h * 0.11).fill(0x101a12);
    this.banks.ellipse(w * 0.96, bankY + 58, w * 0.22, h * 0.13).fill(0x101a12);
    this.banks.ellipse(w * 0.2, bankY + 70, w * 0.12, h * 0.07).fill(0x0b130d);

    // Lenny's perch — a flat-topped boulder at the river's edge
    const perchW = Math.min(w, h) * 0.4;
    const perchTop = h * 0.7;
    const px = w * 0.42;
    this.perch.clear();
    this.perch
      .poly([
        px - perchW / 2, perchTop,
        px + perchW / 2, perchTop,
        px + perchW * 0.3, perchTop + h * 0.18,
        px - perchW * 0.3, perchTop + h * 0.18,
      ])
      .fill(0x1a241d);
    this.perch.ellipse(px, perchTop, perchW / 2, h * 0.02).fill(0x2a3a2e);
    // sunlit rim on the perch
    this.perch
      .ellipse(px - perchW * 0.06, perchTop - 2, perchW * 0.42, h * 0.011)
      .fill({ color: 0xffe2a8, alpha: 0.4 });

    this.petAnchor = { x: px, y: perchTop - 6 };

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

  private drawPines(): void {
    const g = this.pines;
    g.clear();
    const rnd = this.lcg(123);
    const baseY = this.horizonY + 10;
    // two forest clusters — riverbanks left and right
    for (let c = 0; c < 2; c++) {
      const cx = c === 0 ? this.w * 0.16 : this.w * 0.84;
      for (let i = 0; i < 6; i++) {
        const x = cx + (rnd() - 0.5) * this.w * 0.24;
        const th = this.h * (0.06 + rnd() * 0.06);
        const tw = th * 0.42;
        g.rect(x - th * 0.03, baseY - th * 0.25, th * 0.06, th * 0.25).fill(0x0a1220);
        for (let t = 0; t < 3; t++) {
          const ty = baseY - th * 0.2 - t * th * 0.26;
          const tww = tw * (1 - t * 0.24);
          g.poly([x - tww / 2, ty, x + tww / 2, ty, x, ty - th * 0.34]).fill(0x0a1220);
        }
      }
    }
  }

  private drawOutcrops(): void {
    const g = this.outcrops;
    g.clear();
    const rnd = this.lcg(200);
    for (let i = 0; i < 4; i++) {
      const x = rnd() * this.w;
      const ow = this.w * (0.05 + rnd() * 0.05);
      const oh = this.h * (0.02 + rnd() * 0.025);
      g.ellipse(x, this.horizonY + 12, ow, oh).fill(0x0a1220);
    }
  }

  // --------------------------------------------------------------- update

  /** Adventure grade: lift the shared palette warmer/brighter. */
  private tone(pal: Palette): Palette {
    return {
      skyTop: liftColor(pal.skyTop, 0.25),
      skyBottom: liftColor(pal.skyBottom, 0.25),
      starAlpha: pal.starAlpha * 0.45,
      moonAlpha: pal.moonAlpha * 0.6,
      waterDeep: liftColor(pal.waterDeep, 0.25),
      waterShallow: liftColor(pal.waterShallow, 0.25),
      waterGlow: pal.waterGlow,
      ridgeFar: liftColor(pal.ridgeFar, 0.25),
      ridgeNear: liftColor(pal.ridgeNear, 0.25),
      ambient: 0.72 + pal.ambient * 0.28,
      fireflyAlpha: pal.fireflyAlpha * 0.8,
    };
  }

  update(dt: number, cycle: DayNightCycle, px: number, py: number): void {
    this.elapsed += dt;
    const t = this.elapsed;
    const pal = this.tone(cycle.palette);
    const strength = CONFIG.parallaxStrength;

    // idle drift so the scene breathes even without input
    const driftX = Math.sin(t * 0.12) * 5;

    this.skyLayer.position.set(-px * strength * 0.04 + driftX * 0.3, -py * strength * 0.02);
    this.farLayer.position.set(-px * strength * 0.1 + driftX * 0.5, -py * strength * 0.05);
    this.midLayer.position.set(-px * strength * 0.2 + driftX * 0.8, -py * strength * 0.1);
    this.waterLayer.position.set(-px * strength * 0.3 + driftX, -py * strength * 0.14);
    this.foreLayer.position.set(-px * strength * 0.45 + driftX * 1.3, -py * strength * 0.2);

    // throttled palette restyle
    this.restyleTimer += dt;
    if (this.restyleTimer > 0.4) {
      this.restyleTimer = 0;
      this.restyle(pal);
    }

    // stars twinkle (faint — a dawn sky)
    for (const st of this.stars) {
      st.sprite.alpha = pal.starAlpha * (0.35 + 0.65 * (0.5 + 0.5 * Math.sin(t * st.speed + st.phase)));
    }
    // sun strength follows ambient light
    const sunA = Math.min(1, Math.max(0.06, (pal.ambient - 0.55) * 2.4));
    this.sun.alpha = sunA;
    this.sunGlow.alpha = (0.7 + 0.3 * Math.sin(t * 0.7)) * sunA;
    for (const r of this.rays) {
      r.alpha = sunA * 0.5 * (0.7 + 0.3 * Math.sin(t * 0.9 + r.rotation * 4));
    }

    // rushing water — streaks race downstream (leftward)
    for (const st of this.streaks) {
      const s = st.sprite;
      s.position.x -= st.speed * dt;
      if (s.position.x < -40) s.position.x = this.w + 40;
      s.alpha = st.baseAlpha * (0.6 + 0.4 * Math.sin(t * 2.6 + st.phase)) * (0.4 + pal.ambient * 0.6);
    }
    this.sunPath.alpha = 0.55 * sunA * (0.75 + 0.25 * Math.sin(t * 1.9));
    this.sunPath.scale.x = 1 + 0.1 * Math.sin(t * 1.1);

    // rapids churn
    for (const rp of this.rapids) {
      for (const d of rp.dots) {
        d.t += dt;
        if (d.t >= d.dur) {
          d.t = 0;
          d.dur = 0.6 + Math.random() * 0.6;
          d.dx = (Math.random() - 0.5) * 70;
          d.dy = (Math.random() - 0.5) * 26;
        }
        const k = d.t / d.dur;
        d.sprite.position.set(
          rp.x + d.dx - this.waterLayer.position.x,
          rp.y + d.dy - this.waterLayer.position.y - k * 14,
        );
        const sc = (5 + k * 15) / 32;
        d.sprite.scale.set(sc);
        d.sprite.alpha = 0.6 * (1 - k) * (0.5 + pal.ambient * 0.5);
      }
    }

    // leaves ride the current, bobbing
    for (const l of this.leaves) {
      const s = l.sprite;
      s.position.x -= l.speed * dt;
      if (s.position.x < -30) {
        s.position.x = this.w + 30;
        s.position.y = this.horizonY + 14 + Math.random() * (this.h - this.horizonY - 30);
        l.baseY = s.position.y;
      }
      s.position.y = l.baseY + Math.sin(t * 2 + l.phase) * 6;
      s.rotation += l.spin * dt;
      s.alpha = 0.35 + 0.65 * pal.ambient;
    }

    // mist drift
    for (const m of this.mists) {
      m.sprite.position.x += m.speed * dt;
      if (m.sprite.position.x > this.w + 120) m.sprite.position.x = -120;
    }

    // light motes in the sunbeams
    for (const mo of this.motes) {
      const s = mo.sprite;
      s.position.x += Math.sin(t * mo.speed + mo.phase) * mo.amp * dt;
      s.position.y += Math.cos(t * mo.speed * 0.7 + mo.phase * 1.3) * mo.amp * 0.6 * dt;
      s.alpha = pal.ambient * (0.2 + 0.6 * (0.5 + 0.5 * Math.sin(t * 2 + mo.phase * 3)));
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
        { offset: 1, color: css(0x0a0f14) },
      ],
      4,
      256,
    );
    this.waterBase.texture = this.waterTex;

    this.farRidge.tint = pal.ridgeFar;
    this.nearRidge.tint = pal.ridgeNear;
    this.pines.tint = pal.ridgeNear;
    this.outcrops.tint = pal.ridgeNear;
    this.banks.tint = 0xffffff;
    this.perch.tint = 0xffffff;

    this.sunPath.tint = pal.waterGlow;
    for (const st of this.streaks) st.sprite.tint = pal.waterGlow;

    const dim = 0.6 + pal.ambient * 0.4;
    this.midLayer.alpha = dim;
    this.farLayer.alpha = 0.55 + pal.ambient * 0.45;
  }

  // --------------------------------------------------------------- accessors

  getPetAnchor(): { x: number; y: number } {
    return this.petAnchor;
  }

  /** Water rect in screen coords — for ambient glint particles. */
  getWaterBounds(): { x: number; y: number; w: number; h: number } {
    return { x: 0, y: this.horizonY, w: this.w, h: this.h - this.horizonY };
  }
}
