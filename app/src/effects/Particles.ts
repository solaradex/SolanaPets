/**
 * Pooled CPU particle system.
 *
 * One system instance can drive many emitters (care bursts, ambient
 * glints, splash, hearts). Particles are Sprites drawn from a fixed pool —
 * no allocation during gameplay, safe for 60fps on mobile.
 */
import { Container, Sprite, Texture } from 'pixi.js';
import { softDotTexture } from '../core/textures';

/** PixiJS v8 blend modes are string literals (BLEND_MODES is type-only). */
export type BlendMode = 'normal' | 'add' | 'multiply' | 'screen' | 'overlay';

export interface SpawnOptions {
  x: number;
  y: number;
  count?: number;
  texture?: Texture;
  /** Single tint or a list to pick from per particle. */
  tint?: number | number[];
  speedMin?: number;
  speedMax?: number;
  /** Degrees, 0 = right, 90 = down (screen coords). */
  angleMin?: number;
  angleMax?: number;
  lifeMin?: number;
  lifeMax?: number;
  sizeMin?: number;
  sizeMax?: number;
  /** Size multiplier at end of life (1 = constant). */
  endSize?: number;
  gravity?: number; // px/s^2
  drag?: number; // 0..1 per second retained
  alpha?: number;
  blendMode?: BlendMode;
  /** Horizontal sine sway while alive. */
  swayAmp?: number;
  swayFreq?: number;
  /** Initial rotation + spin for non-round textures. */
  spinMin?: number;
  spinMax?: number;
}

interface Particle {
  sprite: Sprite;
  active: boolean;
  vx: number;
  vy: number;
  life: number;
  maxLife: number;
  sizeStart: number;
  sizeEnd: number;
  alphaStart: number;
  gravity: number;
  drag: number;
  swayAmp: number;
  swayFreq: number;
  swayPhase: number;
  baseX: number;
  spin: number;
}

const rand = (a: number, b: number) => a + Math.random() * (b - a);
const pick = <T>(v: T | T[]): T => (Array.isArray(v) ? v[(Math.random() * v.length) | 0]! : v);

export class ParticleSystem {
  readonly container = new Container();
  private pool: Particle[] = [];
  private cursor = 0;
  private defaultTexture: Texture;

  constructor(max = 500) {
    this.defaultTexture = softDotTexture();
    for (let i = 0; i < max; i++) {
      const sprite = new Sprite(this.defaultTexture);
      sprite.anchor.set(0.5);
      sprite.visible = false;
      this.container.addChild(sprite);
      this.pool.push({
        sprite,
        active: false,
        vx: 0, vy: 0, life: 0, maxLife: 1,
        sizeStart: 8, sizeEnd: 8, alphaStart: 1,
        gravity: 0, drag: 1, swayAmp: 0, swayFreq: 0,
        swayPhase: 0, baseX: 0, spin: 0,
      });
    }
  }

  spawn(o: SpawnOptions): void {
    const count = o.count ?? 12;
    for (let n = 0; n < count; n++) {
      const p = this.pool[this.cursor];
      this.cursor = (this.cursor + 1) % this.pool.length;
      const s = p.sprite;
      const angle = (rand(o.angleMin ?? 0, o.angleMax ?? 360) * Math.PI) / 180;
      const speed = rand(o.speedMin ?? 20, o.speedMax ?? 90);
      p.vx = Math.cos(angle) * speed;
      p.vy = Math.sin(angle) * speed;
      p.maxLife = rand(o.lifeMin ?? 0.6, o.lifeMax ?? 1.4);
      p.life = p.maxLife;
      p.sizeStart = rand(o.sizeMin ?? 6, o.sizeMax ?? 14);
      p.sizeEnd = p.sizeStart * (o.endSize ?? 0.2);
      p.alphaStart = o.alpha ?? 1;
      p.gravity = o.gravity ?? 0;
      p.drag = o.drag ?? 1;
      p.swayAmp = o.swayAmp ?? 0;
      p.swayFreq = o.swayFreq ?? 0;
      p.swayPhase = Math.random() * Math.PI * 2;
      p.baseX = o.x;
      p.spin = rand(o.spinMin ?? 0, o.spinMax ?? 0);
      s.texture = o.texture ?? this.defaultTexture;
      s.tint = pick(o.tint ?? 0xffffff);
      s.blendMode = o.blendMode ?? 'normal';
      s.position.set(o.x, o.y);
      s.rotation = Math.random() * Math.PI * 2;
      s.visible = true;
      p.active = true;
    }
  }

  /** Convenience: radial burst. */
  burst(x: number, y: number, o: Omit<SpawnOptions, 'x' | 'y' | 'angleMin' | 'angleMax'>): void {
    this.spawn({ ...o, x, y, angleMin: 0, angleMax: 360 });
  }

  /** Convenience: upward fountain (splash, sparkles rising). */
  fountain(x: number, y: number, o: Omit<SpawnOptions, 'x' | 'y' | 'angleMin' | 'angleMax'>): void {
    this.spawn({ ...o, x, y, angleMin: 235, angleMax: 305 });
  }

  update(dt: number): void {
    for (const p of this.pool) {
      if (!p.active) continue;
      p.life -= dt;
      if (p.life <= 0) {
        p.active = false;
        p.sprite.visible = false;
        continue;
      }
      const t = 1 - p.life / p.maxLife; // 0 → 1
      // drag
      const d = Math.pow(p.drag, dt * 60);
      p.vx *= d;
      p.vy *= d;
      p.vy += p.gravity * dt;
      const s = p.sprite;
      let nx = s.position.x + p.vx * dt;
      const ny = s.position.y + p.vy * dt;
      if (p.swayAmp > 0) {
        p.swayPhase += p.swayFreq * dt;
        nx = p.baseX + (nx - p.baseX) + Math.sin(p.swayPhase) * p.swayAmp * dt * 60 * 0.016;
      }
      s.position.set(nx, ny);
      s.rotation += p.spin * dt;
      const size = p.sizeStart + (p.sizeEnd - p.sizeStart) * t;
      const texW = s.texture.width || 64;
      s.scale.set(size / texW);
      // ease-out fade
      s.alpha = p.alphaStart * (1 - t * t);
    }
  }
}
