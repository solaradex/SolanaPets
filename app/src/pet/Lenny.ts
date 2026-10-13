/**
 * Lenny — Genesis Founder #2, the River Explorer.
 *
 * Procedural stand-in for the production sprite sheet (see PetSprite.ts).
 * A brave, playful otter in a blue explorer bandana with a compass emblem.
 * His legendary traits: the luminescent Flow Mark striping his tail and
 * his Tidecaller Eyes — ocean blue, softly glowing.
 *
 * Same Pet interface as Luna (container/update/reactTo/setMood) — the
 * rig, reactions and mood faces are identical so game logic never cares
 * which pet is on screen.
 */
import { Container, Graphics, Sprite } from 'pixi.js';
import { CareAction, Mood } from '../config';
import { radialGlowTexture } from '../core/textures';
import { Pet } from './PetSprite';

const FUR_DARK = 0x5d4128; // A - River Brown
const FUR_MID = 0x6e5133;
const BELLY = 0xd9c096; // B - Sand
const MUZZLE = 0xe6d0a8;
const NOSE = 0x2a1e14;
const IRIS = 0x2b9fd8; // E-B Ocean Blue (Tidecaller — glowing)
const PUPIL = 0x0d1a24;
const FLOW_GLOW = 0x7fd4ff; // Flow Mark luminescence
const BANDANA = 0x2b6cb0; // explorer blue
const BANDANA_DARK = 0x1e4e80;
const COMPASS = 0xf6d47c;

type Reaction = 'idle' | 'hop' | 'eat' | 'settle';

export class Lenny implements Pet {
  readonly container = new Container();

  /** Inner rig: reactions (hop/squash) animate here; Game owns container transform. */
  private rig = new Container();
  private torso = new Container();
  private headGroup = new Container();
  private tailGroup = new Container();
  private eyeL = new Container();
  private eyeR = new Container();
  private eyeGlowL!: Sprite;
  private eyeGlowR!: Sprite;
  private mouth = new Graphics();
  private flowGlow!: Sprite;
  private flowStripe!: Graphics;

  private headBaseY = -78;
  private blinkTimer = 2.1;
  private blinkT = 0;
  private reaction: Reaction = 'idle';
  private reactionT = 0;
  private eyeOpen = 1;
  private eyeOpenTarget = 1;
  private mood: Mood = 'happy';
  private squash = 0;

  constructor() {
    this.buildTail();
    this.buildTorso();
    this.buildHead();
    this.rig.addChild(this.tailGroup, this.torso, this.headGroup);
    this.container.addChild(this.rig);
  }

  // ------------------------------------------------------------------ build

  /** Tapered strip polygon along a spine — used for the tail. */
  private strip(g: Graphics, pts: Array<[number, number]>, widths: number[], color: number): void {
    const left: number[] = [];
    const right: number[] = [];
    for (let i = 0; i < pts.length; i++) {
      const [x, y] = pts[i]!;
      const [nx, ny] = pts[Math.min(i + 1, pts.length - 1)]!;
      const [px, py] = pts[Math.max(i - 1, 0)]!;
      let dx = nx - px;
      let dy = ny - py;
      const len = Math.hypot(dx, dy) || 1;
      dx /= len;
      dy /= len;
      const w = widths[i]! / 2;
      left.push(x - dy * w, y + dx * w);
      right.push(x + dy * w, y - dx * w);
    }
    const poly: number[] = [];
    for (let i = 0; i < left.length; i += 2) poly.push(left[i]!, left[i + 1]!);
    for (let i = right.length - 2; i >= 0; i -= 2) poly.push(right[i]!, right[i + 1]!);
    g.poly(poly).fill(color);
  }

  private buildTail(): void {
    const g = new Graphics();
    // tail sweeps back-right and curls upward — mirrored from Luna for character
    this.strip(
      g,
      [[0, 0], [28, -6], [52, -2], [70, -18], [78, -44]],
      [26, 23, 18, 13, 8],
      FUR_DARK,
    );
    // lighter underside streak
    this.strip(
      g,
      [[6, 4], [30, 2], [52, 6], [68, -8]],
      [10, 9, 7, 5],
      FUR_MID,
    );
    this.tailGroup.addChild(g);

    // Flow Mark — luminescent stripe along the tail (legendary trait)
    this.flowStripe = new Graphics();
    this.strip(
      this.flowStripe,
      [[14, -2], [38, -4], [58, -12], [70, -28]],
      [7, 6, 5, 4],
      FLOW_GLOW,
    );
    this.flowGlow = new Sprite(radialGlowTexture('rgba(127,212,255,0.85)', 128));
    this.flowGlow.anchor.set(0.5);
    this.flowGlow.blendMode = 'add';
    this.flowGlow.position.set(48, -12);
    this.flowGlow.scale.set(0.5);
    this.tailGroup.addChild(this.flowStripe, this.flowGlow);
    this.tailGroup.position.set(42, 42);
  }

  private buildTorso(): void {
    const g = new Graphics();
    g.ellipse(0, 12, 62, 78).fill(FUR_DARK);
    g.ellipse(14, -4, 46, 60).fill({ color: FUR_MID, alpha: 0.55 });
    // belly
    g.ellipse(-8, 30, 40, 52).fill(BELLY);
    // little arms
    g.ellipse(-56, 26, 13, 26).fill(FUR_DARK);
    g.ellipse(52, 26, 13, 26).fill(FUR_DARK);
    // feet
    g.ellipse(-30, 84, 20, 12).fill(FUR_MID);
    g.ellipse(26, 84, 20, 12).fill(FUR_MID);
    this.torso.addChild(g);

    // explorer bandana around the neck
    const b = new Graphics();
    b.roundRect(-46, -38, 92, 26, 10).fill(BANDANA);
    b.roundRect(-46, -38, 92, 10, 6).fill({ color: BANDANA_DARK, alpha: 0.45 });
    // knot + tails on the left side
    b.circle(-38, -24, 10).fill(BANDANA_DARK);
    b.poly([-44, -18, -58, 6, -46, 8, -34, -14]).fill(BANDANA);
    b.poly([-36, -16, -40, 14, -30, 12, -26, -14]).fill(BANDANA_DARK);
    this.torso.addChild(b);

    // compass emblem on the bandana
    const c = new Graphics();
    c.circle(18, -25, 11).fill(COMPASS);
    c.circle(18, -25, 8.5).fill(BANDANA_DARK);
    c.stroke({ width: 2, color: COMPASS });
    c.moveTo(18, -33).lineTo(18, -17);
    c.moveTo(10, -25).lineTo(26, -25);
    // needle pointing north-east — the explorer's way
    c.poly([18, -25, 22, -21, 18, -31, 14, -21]).fill(0xff6b5e);
    this.torso.addChild(c);
  }

  private makeEye(): Container {
    const c = new Container();
    const g = new Graphics();
    g.circle(0, 0, 13).fill(0xffffff);
    g.circle(0, 1, 8.5).fill(IRIS);
    g.circle(0, 1, 4.2).fill(PUPIL);
    g.circle(-2.5, -2, 2.6).fill(0xffffff);
    c.addChild(g);
    return c;
  }

  private buildHead(): void {
    const g = new Graphics();
    // ears
    g.poly([-40, -36, -54, -66, -24, -50]).fill(FUR_DARK);
    g.poly([-36, -40, -46, -58, -28, -48]).fill(BELLY);
    g.poly([32, -38, 46, -68, 18, -52]).fill(FUR_DARK);
    g.poly([30, -42, 38, -60, 22, -50]).fill(BELLY);
    // head
    g.circle(0, 0, 52).fill(FUR_DARK);
    g.circle(10, -10, 38).fill({ color: FUR_MID, alpha: 0.5 });
    // muzzle
    g.ellipse(-6, 20, 30, 22).fill(MUZZLE);
    // nose
    g.roundRect(-14, 8, 16, 11, 5).fill(NOSE);
    g.circle(-10, 11, 2).fill({ color: 0xffffff, alpha: 0.35 });
    // whiskers
    g.stroke({ width: 1.5, color: 0xffffff, alpha: 0.55 });
    for (const side of [-1, 1]) {
      for (let i = 0; i < 3; i++) {
        const y = 18 + i * 6;
        g.moveTo(-6 + side * 24, y).lineTo(-6 + side * 52, y - 4 + i * 4);
      }
    }
    this.headGroup.addChild(g);

    this.eyeL = this.makeEye();
    this.eyeL.position.set(-32, -10);
    this.eyeR = this.makeEye();
    this.eyeR.position.set(20, -10);
    // Tidecaller Eyes — soft ocean glow behind each eye (legendary trait)
    const glowTex = radialGlowTexture('rgba(79,195,255,0.55)', 64);
    this.eyeGlowL = new Sprite(glowTex);
    this.eyeGlowL.anchor.set(0.5);
    this.eyeGlowL.blendMode = 'add';
    this.eyeGlowL.position.copyFrom(this.eyeL.position);
    this.eyeGlowL.scale.set(0.9);
    this.eyeGlowR = new Sprite(glowTex);
    this.eyeGlowR.anchor.set(0.5);
    this.eyeGlowR.blendMode = 'add';
    this.eyeGlowR.position.copyFrom(this.eyeR.position);
    this.eyeGlowR.scale.set(0.9);
    this.headGroup.addChild(this.eyeGlowL, this.eyeGlowR, this.eyeL, this.eyeR, this.mouth);

    this.headGroup.position.set(-8, this.headBaseY);
    this.drawMouth();
  }

  private drawMouth(): void {
    const g = this.mouth;
    g.clear();
    const sad = this.mood === 'hungry' || this.mood === 'lonely' || this.mood === 'bored' || this.mood === 'dirty';
    if (this.mood === 'sleepy') {
      g.moveTo(-14, 38).lineTo(2, 38).stroke({ width: 2.5, color: NOSE, alpha: 0.8 });
    } else if (sad) {
      // small worried line
      g.moveTo(-14, 40).quadraticCurveTo(-6, 36, 2, 40).stroke({ width: 2.5, color: NOSE, alpha: 0.85 });
    } else if (this.mood === 'happy') {
      // big explorer grin
      g.arc(-6, 32, 12, Math.PI * 0.2, Math.PI * 0.8).stroke({ width: 3, color: NOSE });
    } else {
      g.arc(-6, 33, 9, Math.PI * 0.25, Math.PI * 0.75).stroke({ width: 2.5, color: NOSE, alpha: 0.9 });
    }
  }

  // ------------------------------------------------------------------ api

  setMood(mood: Mood): void {
    if (mood === this.mood) return;
    this.mood = mood;
    this.eyeOpenTarget = mood === 'sleepy' ? 0.45 : 1;
    this.drawMouth();
  }

  reactTo(action: CareAction): void {
    this.reaction = action === 'feed' ? 'eat' : action === 'rest' ? 'settle' : 'hop';
    this.reactionT = 0;
    if (this.reaction === 'settle') this.eyeOpenTarget = 0.12;
  }

  // ---------------------------------------------------------------- update

  update(dt: number, time: number): void {
    // breathing
    const br = Math.sin(time * 2.5);
    this.torso.scale.set(1 + 0.012 * br, 1 + 0.022 * br);
    this.headGroup.position.y = this.headBaseY + Math.sin(time * 2.5 + 0.5) * 2.4;

    // blinking
    this.blinkTimer -= dt;
    if (this.blinkTimer <= 0) {
      this.blinkT = 0.13;
      this.blinkTimer = 2.2 + Math.random() * 2.6;
    }
    let eyeScale = this.eyeOpen;
    if (this.blinkT > 0) {
      this.blinkT -= dt;
      eyeScale = Math.max(0.08, eyeScale * (this.blinkT / 0.13));
    }
    this.eyeOpen += (this.eyeOpenTarget - this.eyeOpen) * Math.min(1, dt * 8);
    this.eyeL.scale.y = eyeScale;
    this.eyeR.scale.y = eyeScale;
    this.eyeGlowL.scale.y = 0.9 * Math.max(0.15, eyeScale);
    this.eyeGlowR.scale.y = 0.9 * Math.max(0.15, eyeScale);

    // tail sway + Flow Mark pulse
    this.tailGroup.rotation = 0.12 + Math.sin(time * 1.7) * 0.2;
    this.flowGlow.alpha = 0.55 + 0.4 * Math.sin(time * 3.3);
    this.flowStripe.alpha = 0.75 + 0.25 * Math.sin(time * 3.3 + 0.6);
    // Tidecaller eye shimmer
    const shimmer = 0.5 + 0.35 * Math.sin(time * 2.6 + 1.2);
    this.eyeGlowL.alpha = shimmer;
    this.eyeGlowR.alpha = shimmer;

    // reactions
    if (this.reaction !== 'idle') {
      this.reactionT += dt;
      const rt = this.reactionT;
      if (this.reaction === 'hop') {
        if (rt >= 0.55) {
          this.reaction = 'idle';
          this.squash = 0.12; // landing squash, recovers below
        }
      } else if (this.reaction === 'eat') {
        const dur = 0.95;
        if (rt >= dur) {
          this.reaction = 'idle';
        } else {
          this.headGroup.position.y += Math.abs(Math.sin(rt * 16)) * -9;
          this.headGroup.rotation = Math.sin(rt * 16) * 0.06;
        }
      } else if (this.reaction === 'settle') {
        const dur = 2.4;
        if (rt >= dur) {
          this.reaction = 'idle';
          this.eyeOpenTarget = this.mood === 'sleepy' ? 0.45 : 1;
        } else {
          this.torso.scale.set(0.97, 0.95);
        }
      }
    }
    if (this.reaction !== 'settle') {
      this.headGroup.rotation *= Math.max(0, 1 - dt * 8);
    }
    // squash recovery (landing)
    if (this.squash > 0) {
      this.squash = Math.max(0, this.squash - dt * 0.45);
    }
    const hopK = this.reaction === 'hop' && this.reactionT < 0.55
      ? -Math.sin((this.reactionT / 0.55) * Math.PI) * 52
      : 0;
    const sq = 1 - this.squash;
    this.rig.scale.set(1 + this.squash * 0.6, sq);
    this.rig.position.y = hopK;
  }
}
