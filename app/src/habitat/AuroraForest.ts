/**
 * AuroraForest — Leo's habitat.
 *
 * "The Guiding Light." A mountain forest at night under the aurora borealis:
 * five parallax layers (sky/far/mid/water/fore, same factors as MoonlitCove),
 * flowing aurora curtains, dense pine silhouettes, and a glassy lake that
 * mirrors the lights. Leo's rocky outcrop anchors the foreground.
 *
 * This habitat is NIGHT-dominant — the aurora burns brightest at deep night
 * and fades as the shared DayNightCycle moves toward dawn/day. Aurora
 * intensity is driven by the palette's moonAlpha so it stays in sync with
 * the rest of the client's lighting.
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

interface AuroraBand {
  sprite: Sprite;
  baseX: number; // 0..1 of width
  driftSpeed: number;
  driftAmp: number;
  phase: number;
  pulseSpeed: number;
  baseAlpha: number;
  baseScaleY: number;
  tint: number;
  reflection: Sprite;
}

interface Glint {
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

const OVERSCAN = 90;

const AURORA_TINTS = [0x37f2a2, 0x2ee6c8, 0x7a5cff, 0x46ffa0, 0xb06cff, 0x2ee6c8];

export class AuroraForest extends Container {
  private skyLayer = new Container();
  private farLayer = new Container();
  private midLayer = new Container();
  private waterLayer = new Container();
  private foreLayer = new Container();

  private skySprite!: Sprite;
  private skyTex: Texture | null = null;
  private stars: Star[] = [];
  private auroraGroup = new Container();
  private bands: AuroraBand[] = [];

  private farRidge!: Graphics;
  private nearRidge!: Graphics;
  private pinesFar!: Graphics;
  private pinesNear!: Graphics;

  private lakeBase!: Sprite;
  private lakeTex: Texture | null = null;
  private glints: Glint[] = [];

  private mists: Mist[] = [];
  private fireflies: Firefly[] = [];
  private outcrop!: Graphics;
  private outcropRim!: Graphics;

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
    this.buildLake();
    this.buildAtmosphere();
    this.buildForeground();
  }

  // ---------------------------------------------------------------- build

  private buildSky(): void {
    this.skySprite = new Sprite();
    this.skyLayer.addChild(this.skySprite);

    const dot = softDotTexture(32);
    for (let i = 0; i < 140; i++) {
      const s = new Sprite(dot);
      s.anchor.set(0.5);
      s.tint = 0xffffff;
      s.blendMode = 'add';
      const sc = 1 + Math.random() * 2.2;
      s.scale.set(sc / 32);
      this.skyLayer.addChild(s);
      this.stars.push({ sprite: s, phase: Math.random() * Math.PI * 2, speed: 0.6 + Math.random() * 2.4 });
    }

    // Aurora curtains — stretched radial glows, additive, animated per-frame.
    // Band textures are shared; per-band tint gives the green→teal→violet spread.
    const bandTex = radialGlowTexture('rgba(255,255,255,1)', 256);
    const reflTex = gradientTexture(
      [
        { offset: 0, color: 'rgba(255,255,255,0.9)' },
        { offset: 1, color: 'rgba(255,255,255,0)' },
      ],
      8,
      256,
    );
    for (let i = 0; i < AURORA_TINTS.length; i++) {
      const tint = AURORA_TINTS[i]!;
      const s = new Sprite(bandTex);
      s.anchor.set(0.5);
      s.tint = tint;
      s.blendMode = 'add';
      s.rotation = (Math.random() - 0.5) * 0.24;
      this.auroraGroup.addChild(s);

      const r = new Sprite(reflTex);
      r.anchor.set(0.5, 0);
      r.tint = tint;
      r.blendMode = 'add';
      this.waterLayer.addChild(r);

      this.bands.push({
        sprite: s,
        baseX: 0.08 + (i / (AURORA_TINTS.length - 1)) * 0.84 + (Math.random() - 0.5) * 0.06,
        driftSpeed: 0.1 + Math.random() * 0.22,
        driftAmp: 0.03 + Math.random() * 0.05,
        phase: Math.random() * Math.PI * 2,
        pulseSpeed: 0.5 + Math.random() * 0.9,
        baseAlpha: 0.5 + Math.random() * 0.35,
        baseScaleY: 0.9 + Math.random() * 0.5,
        tint,
        reflection: r,
      });
    }
    this.skyLayer.addChild(this.auroraGroup);
  }

  private buildRidges(): void {
    this.farRidge = new Graphics();
    this.farLayer.addChild(this.farRidge);
    this.nearRidge = new Graphics();
    this.midLayer.addChild(this.nearRidge);
    this.pinesFar = new Graphics();
    this.farLayer.addChild(this.pinesFar);
    this.pinesNear = new Graphics();
    this.midLayer.addChild(this.pinesNear);
  }

  private buildLake(): void {
    this.lakeBase = new Sprite();
    this.waterLayer.addChild(this.lakeBase);

    const dot = softDotTexture(32);
    for (let i = 0; i < 22; i++) {
      const s = new Sprite(dot);
      s.anchor.set(0.5);
      s.blendMode = 'add';
      this.waterLayer.addChild(s);
      this.glints.push({
        sprite: s,
        speed: 8 + Math.random() * 22,
        baseAlpha: 0.1 + Math.random() * 0.28,
        phase: Math.random() * Math.PI * 2,
      });
    }
  }

  private buildAtmosphere(): void {
    const dot = softDotTexture(128);
    for (let i = 0; i < 7; i++) {
      const s = new Sprite(dot);
      s.anchor.set(0.5);
      s.tint = 0x8fd8c8;
      s.alpha = 0.06;
      this.midLayer.addChild(s);
      this.mists.push({ sprite: s, speed: 3 + Math.random() * 7 });
    }
    const glow = radialGlowTexture('rgba(140,255,190,0.9)', 64);
    for (let i = 0; i < 16; i++) {
      const s = new Sprite(glow);
      s.anchor.set(0.5);
      s.blendMode = 'add';
      const sc = 0.22 + Math.random() * 0.3;
      s.scale.set(sc);
      this.foreLayer.addChild(s);
      this.fireflies.push({
        sprite: s,
        bx: Math.random(),
        by: Math.random(),
        phase: Math.random() * Math.PI * 2,
        speed: 0.35 + Math.random() * 0.7,
        amp: 12 + Math.random() * 24,
      });
    }
  }

  private buildForeground(): void {
    this.outcrop = new Graphics();
    this.foreLayer.addChild(this.outcrop);
    this.outcropRim = new Graphics();
    this.foreLayer.addChild(this.outcropRim);
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
    this.horizonY = h * 0.52;
    const W = w + OVERSCAN * 2;
    const skyH = this.horizonY;

    this.skySprite.position.set(-OVERSCAN, 0);
    this.skySprite.width = W;
    this.skySprite.height = skyH + 4;

    for (const st of this.stars) {
      st.sprite.position.set(Math.random() * w, Math.random() * skyH * 0.92);
    }

    // aurora bands hang in the upper sky, tall enough to feel like curtains
    for (const b of this.bands) {
      const bw = Math.min(w, h) * (0.1 + Math.random() * 0.08);
      b.sprite.width = bw;
      b.sprite.height = skyH * 1.15 * b.baseScaleY;
      b.sprite.position.set(b.baseX * w, skyH * 0.42);
      b.reflection.position.set(b.baseX * w, this.horizonY + 6);
      b.reflection.width = bw * 0.5;
      b.reflection.height = (h - this.horizonY) * 0.8;
    }

    this.drawRidge(this.farRidge, this.horizonY, h * 0.12, 6, this.lcg(41));
    this.drawRidge(this.nearRidge, this.horizonY + 8, h * 0.08, 8, this.lcg(77));
    this.drawPines(this.pinesFar, this.lcg(123), 9, 0.75, -0.04);
    this.drawPines(this.pinesNear, this.lcg(201), 12, 1.15, 0.02);

    this.lakeBase.position.set(-OVERSCAN, this.horizonY);
    this.lakeBase.width = W;
    this.lakeBase.height = h - this.horizonY;

    this.glints.forEach((gl, i) => {
      const r = this.lcg(500 + i)();
      gl.sprite.position.set(r * w, this.horizonY + 14 + r * (h - this.horizonY - 34));
      gl.sprite.scale.set((26 + r * 80) / 32, (3 + r * 4) / 32);
    });

    this.mists.forEach((m, i) => {
      m.sprite.position.set((i / 7) * w, this.horizonY + 16 + (i % 3) * 24);
      const ms = Math.min(w, h) * (0.55 + (i % 3) * 0.28);
      m.sprite.scale.set(ms / 128);
    });

    this.fireflies.forEach((f) => {
      f.sprite.position.set(f.bx * w, this.horizonY * 0.3 + f.by * (h * 0.45));
    });

    // Leo's rocky outcrop — left of centre, jutting into the lake
    const cx = w * 0.38;
    const topY = h * 0.7;
    const ocw = Math.min(w, h) * 0.5;
    this.outcrop.clear();
    this.outcrop
      .poly([
        cx - ocw * 0.55, topY + h * 0.02,
        cx + ocw * 0.5, topY - h * 0.01,
        cx + ocw * 0.42, topY + h * 0.16,
        cx - ocw * 0.34, topY + h * 0.2,
      ])
      .fill(0x101828);
    this.outcrop
      .poly([
        cx - ocw * 0.4, topY + h * 0.05,
        cx + ocw * 0.34, topY + h * 0.03,
        cx + ocw * 0.3, topY + h * 0.12,
        cx - ocw * 0.28, topY + h * 0.14,
      ])
      .fill(0x16202f);
    // aurora-lit rim along the top edge
    this.outcropRim.clear();
    this.outcropRim
      .ellipse(cx - ocw * 0.03, topY - h * 0.004, ocw * 0.5, h * 0.01)
      .fill({ color: 0x7df2c0, alpha: 0.4 });
    // a lone pine on the outcrop's right shoulder
    const px = cx + ocw * 0.42;
    const ph = h * 0.11;
    this.outcrop.rect(px - 2, topY - ph * 0.3, 4, ph * 0.3).fill(0x0a1220);
    for (let t = 0; t < 3; t++) {
      const ty = topY - ph * 0.28 - t * ph * 0.24;
      const tww = ph * 0.5 * (1 - t * 0.24);
      this.outcrop.poly([px - tww / 2, ty, px + tww / 2, ty, px, ty - ph * 0.32]).fill(0x0a1220);
    }

    this.petAnchor = { x: cx, y: topY - 4 };

    this.lastStyleKey = '';
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
    g.poly(pts).fill(0xffffff); // tinted per-frame via restyle
  }

  private drawPines(g: Graphics, rnd: () => number, count: number, scale: number, yOff: number): void {
    g.clear();
    const baseY = this.horizonY + this.h * yOff;
    for (let i = 0; i < count; i++) {
      const x = rnd() * this.w;
      const th = this.h * (0.06 + rnd() * 0.07) * scale;
      const tw = th * 0.45;
      g.rect(x - th * 0.03, baseY - th * 0.25, th * 0.06, th * 0.25).fill(0xffffff);
      for (let t = 0; t < 3; t++) {
        const ty = baseY - th * 0.2 - t * th * 0.26;
        const tww = tw * (1 - t * 0.24);
        g.poly([x - tww / 2, ty, x + tww / 2, ty, x, ty - th * 0.34]).fill(0xffffff);
      }
    }
  }

  // --------------------------------------------------------------- update

  update(dt: number, cycle: DayNightCycle, px: number, py: number): void {
    this.elapsed += dt;
    const t = this.elapsed;
    const pal = cycle.palette;
    const strength = CONFIG.parallaxStrength;

    const driftX = Math.sin(t * 0.1) * 5;

    this.skyLayer.position.set(-px * strength * 0.04 + driftX * 0.3, -py * strength * 0.02);
    this.farLayer.position.set(-px * strength * 0.1 + driftX * 0.5, -py * strength * 0.05);
    this.midLayer.position.set(-px * strength * 0.2 + driftX * 0.8, -py * strength * 0.1);
    this.waterLayer.position.set(-px * strength * 0.3 + driftX, -py * strength * 0.14);
    this.foreLayer.position.set(-px * strength * 0.45 + driftX * 1.3, -py * strength * 0.2);

    this.restyleTimer += dt;
    if (this.restyleTimer > 0.4) {
      this.restyleTimer = 0;
      this.restyle(pal);
    }

    // aurora — brightest at deep night, driven by the shared palette
    const nightF = Math.min(1, Math.max(0, pal.moonAlpha));
    this.auroraGroup.alpha = 0.15 + 0.85 * nightF;
    for (const b of this.bands) {
      const s = b.sprite;
      s.position.x = b.baseX * this.w + Math.sin(t * b.driftSpeed + b.phase) * this.w * b.driftAmp;
      s.alpha = b.baseAlpha * (0.65 + 0.35 * Math.sin(t * b.pulseSpeed + b.phase * 2));
      s.scale.y = b.baseScaleY * (1 + 0.07 * Math.sin(t * 0.7 + b.phase));
      s.rotation += Math.sin(t * 0.23 + b.phase) * dt * 0.02;
      // mirrored reflection in the lake, corrected for layer parallax offset
      const r = b.reflection;
      r.position.x = s.position.x + (this.waterLayer.position.x - this.skyLayer.position.x);
      r.alpha = s.alpha * 0.4 * (0.7 + 0.3 * Math.sin(t * 2.1 + b.phase));
      r.scale.x = 1 + 0.15 * Math.sin(t * 1.3 + b.phase);
    }

    for (const st of this.stars) {
      st.sprite.alpha = pal.starAlpha * (0.35 + 0.65 * (0.5 + 0.5 * Math.sin(t * st.speed + st.phase)));
    }

    // lake shimmer
    for (const gl of this.glints) {
      const s = gl.sprite;
      s.position.x += gl.speed * dt;
      if (s.position.x > this.w + 40) s.position.x = -40;
      s.alpha = gl.baseAlpha * (0.6 + 0.4 * Math.sin(t * 2 + gl.phase)) * (0.35 + pal.ambient * 0.65);
    }

    // mist over the lake
    for (const m of this.mists) {
      m.sprite.position.x += m.speed * dt;
      if (m.sprite.position.x > this.w + 140) m.sprite.position.x = -140;
    }

    // fireflies
    for (const f of this.fireflies) {
      const s = f.sprite;
      s.position.x += Math.sin(t * f.speed + f.phase) * f.amp * dt;
      s.position.y += Math.cos(t * f.speed * 0.7 + f.phase * 1.3) * f.amp * 0.6 * dt;
      s.alpha = pal.fireflyAlpha * (0.25 + 0.75 * (0.5 + 0.5 * Math.sin(t * 2.2 + f.phase * 3)));
    }
  }

  private restyle(pal: Palette): void {
    const key = [pal.skyTop, pal.skyBottom, pal.waterDeep, pal.waterShallow, pal.ridgeFar, pal.ridgeNear]
      .map((c) => c.toString(16))
      .join('|');
    if (key === this.lastStyleKey) return;
    this.lastStyleKey = key;

    // night-dominant sky: deepen toward teal so the aurora reads even at dawn/dusk
    const deepTop = 0x04141a;
    const deepBottom = 0x0b2b33;
    this.skyTex?.destroy();
    this.skyTex = gradientTexture(
      [
        { offset: 0, color: css(deepTop) },
        { offset: 0.55, color: css(pal.skyTop) },
        { offset: 1, color: css(deepBottom) },
      ],
      4,
      512,
    );
    this.skySprite.texture = this.skyTex;

    this.lakeTex?.destroy();
    this.lakeTex = gradientTexture(
      [
        { offset: 0, color: css(pal.waterShallow) },
        { offset: 0.3, color: css(pal.waterDeep) },
        { offset: 1, color: css(0x04080d) },
      ],
      4,
      256,
    );
    this.lakeBase.texture = this.lakeTex;

    this.farRidge.tint = pal.ridgeFar;
    this.nearRidge.tint = pal.ridgeNear;
    this.pinesFar.tint = 0x0a1622;
    this.pinesNear.tint = 0x0d1b28;
    this.outcrop.tint = 0xffffff;

    for (const gl of this.glints) gl.sprite.tint = pal.waterGlow;

    const dim = 0.55 + pal.ambient * 0.45;
    this.midLayer.alpha = dim;
    this.farLayer.alpha = 0.5 + pal.ambient * 0.5;
  }

  // --------------------------------------------------------------- accessors

  getPetAnchor(): { x: number; y: number } {
    return this.petAnchor;
  }

  /** Lake rect in screen coords — for ambient glint particles. */
  getWaterBounds(): { x: number; y: number; w: number; h: number } {
    return { x: 0, y: this.horizonY, w: this.w, h: this.h - this.horizonY };
  }
}
