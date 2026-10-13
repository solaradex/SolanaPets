/**
 * Luna — Genesis Founder #1, the Moonlit Otter.
 *
 * Procedural stand-in for the production sprite sheet (see PetSprite.ts).
 * Built from layered PixiJS Graphics so every body part animates:
 * breathing torso, blinking amber eyes, swaying tail with a glowing tip,
 * and the crescent-moon chest marking that pulses with soft bioluminescence.
 *
 * The Pet interface (container/update/reactTo/setMood) is identical to what
 * the real sprite-based pet will expose — swapping art changes nothing else.
 */
import { Container, Graphics, Sprite } from 'pixi.js';
import { CareAction, Mood } from '../config';
import { radialGlowTexture } from '../core/textures';
import { Pet } from './PetSprite';

const FUR_DARK = 0x6b4a33;
const FUR_MID = 0x7d5a3e;
const BELLY = 0xd9b98c;
const MUZZLE = 0xe8d3ac;
const NOSE = 0x2e1f16;
const IRIS = 0xd99a2b;
const PUPIL = 0x140d06;
const MOON_GLOW = 0x9fdcff;

type Reaction = 'idle' | 'hop' | 'eat' | 'settle';

export class Luna implements Pet {
  readonly container = new Container();

  /** Inner rig: reactions (hop/squash) animate here; Game owns container transform. */
  private rig = new Container();
  private torso = new Container();
  private headGroup = new Container();
  private tailGroup = new Container();
  private eyeL = new Container();
  private eyeR = new Container();
  private mouth = new Graphics();
  private moonGlow!: Sprite;
  private tailTipGlow!: Sprite;

  private headBaseY = -78;
  private blinkTimer = 2.4;
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
    // tail sweeps back-left and curls upward, classic otter
    this.strip(
      g,
      [[0, 0], [-28, -6], [-52, -2], [-70, -18], [-78, -44]],
      [26, 23, 18, 13, 8],
      FUR_DARK,
    );
    // lighter underside streak
    this.strip(
      g,
      [[-6, 4], [-30, 2], [-52, 6], [-68, -8]],
      [10, 9, 7, 5],
      FUR_MID,
    );
    this.tailTipGlow = new Sprite(radialGlowTexture('rgba(127,212,255,0.85)', 128));
    this.tailTipGlow.anchor.set(0.5);
    this.tailTipGlow.blendMode = 'add';
    this.tailTipGlow.position.set(-78, -44);
    this.tailTipGlow.scale.set(0.55);
    this.tailGroup.addChild(g, this.tailTipGlow);
    this.tailGroup.position.set(-42, 42);
  }

  private buildTorso(): void {
    const g = new Graphics();
    g.ellipse(0, 12, 62, 78).fill(FUR_DARK);
    g.ellipse(-14, -4, 46, 60).fill({ color: FUR_MID, alpha: 0.55 });
    // belly
    g.ellipse(8, 30, 40, 52).fill(BELLY);
    // little arms
    g.ellipse(-52, 26, 13, 26).fill(FUR_DARK).stroke({ width: 0, color: 0 });
    g.ellipse(56, 26, 13, 26).fill(FUR_DARK);
    // feet
    g.ellipse(-26, 84, 20, 12).fill(FUR_MID);
    g.ellipse(30, 84, 20, 12).fill(FUR_MID);
    this.torso.addChild(g);

    // crescent moon chest marking — Luna's legendary trait
    const mark = new Graphics();
    mark.circle(0, 0, 15).fill(MOON_GLOW);
    mark.circle(7, -4, 12.5).fill(BELLY); // cutout → crescent
    mark.position.set(10, -4);
    this.moonGlow = new Sprite(radialGlowTexture('rgba(159,220,255,0.7)', 128));
    this.moonGlow.anchor.set(0.5);
    this.moonGlow.blendMode = 'add';
    this.moonGlow.position.set(10, -4);
    this.moonGlow.scale.set(0.6);
    this.torso.addChild(this.moonGlow, mark);
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
    g.circle(-10, -10, 38).fill({ color: FUR_MID, alpha: 0.5 });
    // muzzle
    g.ellipse(6, 20, 30, 22).fill(MUZZLE);
    // nose
    g.roundRect(-2, 8, 16, 11, 5).fill(NOSE);
    g.circle(2, 11, 2).fill({ color: 0xffffff, alpha: 0.35 });
    // whiskers
    g.stroke({ width: 1.5, color: 0xffffff, alpha: 0.55 });
    for (const side of [-1, 1]) {
      for (let i = 0; i < 3; i++) {
        const y = 18 + i * 6;
        g.moveTo(6 + side * 24, y).lineTo(6 + side * 52, y - 4 + i * 4);
      }
    }
    this.headGroup.addChild(g);

    this.eyeL = this.makeEye();
    this.eyeL.position.set(-20, -10);
    this.eyeR = this.makeEye();
    this.eyeR.position.set(32, -10);
    this.headGroup.addChild(this.eyeL, this.eyeR, this.mouth);

    this.headGroup.position.set(8, this.headBaseY);
    this.drawMouth();
  }

  private drawMouth(): void {
    const g = this.mouth;
    g.clear();
    const sad = this.mood === 'hungry' || this.mood === 'lonely' || this.mood === 'bored' || this.mood === 'dirty';
    if (this.mood === 'sleepy') {
      g.moveTo(-2, 38).lineTo(14, 38).stroke({ width: 2.5, color: NOSE, alpha: 0.8 });
    } else if (sad) {
      // small worried line
      g.moveTo(-2, 40).quadraticCurveTo(6, 36, 14, 40).stroke({ width: 2.5, color: NOSE, alpha: 0.85 });
    } else if (this.mood === 'happy') {
      g.arc(6, 32, 12, Math.PI * 0.2, Math.PI * 0.8).stroke({ width: 3, color: NOSE });
    } else {
      g.arc(6, 33, 9, Math.PI * 0.25, Math.PI * 0.75).stroke({ width: 2.5, color: NOSE, alpha: 0.9 });
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
    const br = Math.sin(time * 2.3);
    this.torso.scale.set(1 + 0.012 * br, 1 + 0.022 * br);
    this.headGroup.position.y = this.headBaseY + Math.sin(time * 2.3 + 0.5) * 2.4;

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

    // tail sway + glowing tip pulse
    this.tailGroup.rotation = -0.12 + Math.sin(time * 1.6) * 0.2;
    this.tailTipGlow.alpha = 0.55 + 0.4 * Math.sin(time * 3.1);
    this.moonGlow.alpha = 0.6 + 0.35 * Math.sin(time * 2.2 + 1);

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
