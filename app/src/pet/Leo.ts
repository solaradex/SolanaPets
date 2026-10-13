/**
 * Leo — Genesis Founder #6, the Guiding Light.
 *
 * Procedural stand-in for the production sprite sheet (see PetSprite.ts).
 * A tuxedo cat: black coat with a white chest bib, white muzzle mask and
 * white paws. His legendary traits glow green — the arrowhead marking on
 * his chest and his luminous eyes.
 *
 * The Pet interface (container/update/reactTo/setMood) is identical to what
 * the real sprite-based pet will expose — swapping art changes nothing else.
 */
import { Container, Graphics, Sprite } from 'pixi.js';
import { CareAction, Mood } from '../config';
import { radialGlowTexture } from '../core/textures';
import { Pet } from './PetSprite';

const FUR_BLACK = 0x141318;
const FUR_SOFT = 0x232230;
const WHITE = 0xf2ede4;
const NOSE_PINK = 0xd98a8a;
const IRIS = 0x39ff88;
const PUPIL = 0x0a140e;
const ARROW_GLOW = 0x46ff9e;

type Reaction = 'idle' | 'hop' | 'eat' | 'settle';

export class Leo implements Pet {
  readonly container = new Container();

  /** Inner rig: reactions (hop/squash) animate here; Game owns container transform. */
  private rig = new Container();
  private torso = new Container();
  private headGroup = new Container();
  private tailGroup = new Container();
  private earL = new Container();
  private earR = new Container();
  private eyeL = new Container();
  private eyeR = new Container();
  private mouth = new Graphics();
  private arrowGlow!: Sprite;
  private eyeGlowL!: Sprite;
  private eyeGlowR!: Sprite;

  private headBaseY = -84;
  private blinkTimer = 2.8;
  private blinkT = 0;
  private twitchTimer = 3.5;
  private twitchT = 0;
  private twitchEar: 'L' | 'R' = 'L';
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
    // tail sweeps right and curls up around the body, classic sitting cat
    this.strip(
      g,
      [[0, 0], [34, 4], [58, 14], [70, 2], [66, -22]],
      [20, 18, 15, 12, 9],
      FUR_BLACK,
    );
    this.strip(
      g,
      [[6, -2], [34, 0], [56, 8]],
      [8, 7, 6],
      FUR_SOFT,
    );
    this.tailGroup.addChild(g);
    this.tailGroup.position.set(44, 58);
  }

  private buildTorso(): void {
    const g = new Graphics();
    // sitting pear shape
    g.ellipse(0, 18, 56, 74).fill(FUR_BLACK);
    g.ellipse(-12, 2, 40, 56).fill({ color: FUR_SOFT, alpha: 0.55 });
    // white chest bib — the tuxedo front
    g.ellipse(10, 28, 30, 46).fill(WHITE);
    g.ellipse(10, -6, 18, 20).fill(WHITE);
    // front paws (white socks)
    g.roundRect(-24, 74, 20, 26, 9).fill(WHITE);
    g.roundRect(12, 74, 20, 26, 9).fill(WHITE);
    // haunches
    g.ellipse(-44, 52, 24, 34).fill(FUR_BLACK);
    g.ellipse(48, 52, 24, 34).fill(FUR_BLACK);
    this.torso.addChild(g);

    // glowing arrowhead — Leo's legendary trait ("the guiding light")
    const mark = new Graphics();
    const ax = 10;
    const ay = 18;
    mark
      .poly([
        ax, ay - 20, // tip (points up)
        ax - 11, ay - 2,
        ax - 4, ay - 2,
        ax - 4, ay + 14,
        ax + 4, ay + 14,
        ax + 4, ay - 2,
        ax + 11, ay - 2,
      ])
      .fill(ARROW_GLOW);
    this.arrowGlow = new Sprite(radialGlowTexture('rgba(70,255,158,0.7)', 128));
    this.arrowGlow.anchor.set(0.5);
    this.arrowGlow.blendMode = 'add';
    this.arrowGlow.position.set(ax, ay - 2);
    this.arrowGlow.scale.set(0.5);
    this.torso.addChild(this.arrowGlow, mark);
  }

  private makeEar(side: -1 | 1): Container {
    const c = new Container();
    const g = new Graphics();
    // triangle ear, black outside, soft inner
    g.poly([side * -22, 0, side * 22, 0, side * 6, -44]).fill(FUR_BLACK);
    g.poly([side * -13, -4, side * 13, -4, side * 5, -32]).fill(0x8a7f74);
    c.addChild(g);
    return c;
  }

  private makeEye(): { c: Container; glow: Sprite } {
    const c = new Container();
    const glow = new Sprite(radialGlowTexture('rgba(57,255,136,0.55)', 64));
    glow.anchor.set(0.5);
    glow.blendMode = 'add';
    glow.scale.set(0.85);
    const g = new Graphics();
    // almond feline eye
    g.ellipse(0, 0, 14, 11).fill(0xffffff);
    g.ellipse(0, 1, 10, 8).fill(IRIS);
    // vertical slit pupil
    g.ellipse(0, 1, 2.6, 6.4).fill(PUPIL);
    g.circle(-3.5, -2.5, 2.4).fill(0xffffff);
    c.addChild(glow, g);
    return { c, glow };
  }

  private buildHead(): void {
    const g = new Graphics();
    // round head
    g.circle(0, 0, 50).fill(FUR_BLACK);
    g.circle(-10, -12, 36).fill({ color: FUR_SOFT, alpha: 0.5 });
    // white muzzle mask — tuxedo marking
    g.ellipse(4, 22, 30, 20).fill(WHITE);
    // nose
    g.poly([-6, 12, 6, 12, 0, 19]).fill(NOSE_PINK);
    // whiskers
    g.stroke({ width: 1.5, color: 0xffffff, alpha: 0.55 });
    for (const side of [-1, 1]) {
      for (let i = 0; i < 3; i++) {
        const y = 20 + i * 6;
        g.moveTo(4 + side * 26, y).lineTo(4 + side * 56, y - 4 + i * 4);
      }
    }
    this.headGroup.addChild(g);

    this.earL = this.makeEar(-1);
    this.earL.position.set(-34, -38);
    this.earR = this.makeEar(1);
    this.earR.position.set(34, -38);
    this.headGroup.addChild(this.earL, this.earR);

    const eL = this.makeEye();
    this.eyeL = eL.c;
    this.eyeGlowL = eL.glow;
    this.eyeL.position.set(-19, -8);
    const eR = this.makeEye();
    this.eyeR = eR.c;
    this.eyeGlowR = eR.glow;
    this.eyeR.position.set(27, -8);
    this.headGroup.addChild(this.eyeL, this.eyeR, this.mouth);

    this.headGroup.position.set(4, this.headBaseY);
    this.drawMouth();
  }

  private drawMouth(): void {
    const g = this.mouth;
    g.clear();
    const sad = this.mood === 'hungry' || this.mood === 'lonely' || this.mood === 'bored' || this.mood === 'dirty';
    if (this.mood === 'sleepy') {
      g.moveTo(-4, 40).lineTo(12, 40).stroke({ width: 2.5, color: 0x2a2a30, alpha: 0.8 });
    } else if (sad) {
      g.moveTo(-4, 42).quadraticCurveTo(4, 38, 12, 42).stroke({ width: 2.5, color: 0x2a2a30, alpha: 0.85 });
    } else if (this.mood === 'happy') {
      // cat smile: two arcs from the nose
      g.arc(-2, 34, 8, Math.PI * 0.15, Math.PI * 0.85).stroke({ width: 2.5, color: 0x2a2a30 });
      g.arc(10, 34, 8, Math.PI * 0.15, Math.PI * 0.85).stroke({ width: 2.5, color: 0x2a2a30 });
    } else {
      g.arc(4, 36, 8, Math.PI * 0.2, Math.PI * 0.8).stroke({ width: 2.5, color: 0x2a2a30, alpha: 0.9 });
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
    const br = Math.sin(time * 2.1);
    this.torso.scale.set(1 + 0.01 * br, 1 + 0.02 * br);
    this.headGroup.position.y = this.headBaseY + Math.sin(time * 2.1 + 0.5) * 2.2;

    // blinking
    this.blinkTimer -= dt;
    if (this.blinkTimer <= 0) {
      this.blinkT = 0.12;
      this.blinkTimer = 2.4 + Math.random() * 3;
    }
    let eyeScale = this.eyeOpen;
    if (this.blinkT > 0) {
      this.blinkT -= dt;
      eyeScale = Math.max(0.08, eyeScale * (this.blinkT / 0.12));
    }
    this.eyeOpen += (this.eyeOpenTarget - this.eyeOpen) * Math.min(1, dt * 8);
    this.eyeL.scale.y = eyeScale;
    this.eyeR.scale.y = eyeScale;

    // ear twitches — feline signature
    this.twitchTimer -= dt;
    if (this.twitchTimer <= 0) {
      this.twitchT = 0.22;
      this.twitchEar = Math.random() < 0.5 ? 'L' : 'R';
      this.twitchTimer = 2.5 + Math.random() * 4;
    }
    if (this.twitchT > 0) {
      this.twitchT -= dt;
      const k = Math.sin((this.twitchT / 0.22) * Math.PI) * 0.28;
      if (this.twitchEar === 'L') this.earL.rotation = -k;
      else this.earR.rotation = k;
    } else {
      this.earL.rotation *= Math.max(0, 1 - dt * 10);
      this.earR.rotation *= Math.max(0, 1 - dt * 10);
    }

    // tail sway + glowing accents pulse
    this.tailGroup.rotation = -0.1 + Math.sin(time * 1.4) * 0.22;
    this.arrowGlow.alpha = 0.55 + 0.4 * Math.sin(time * 2.6);
    this.eyeGlowL.alpha = 0.5 + 0.35 * Math.sin(time * 3.3);
    this.eyeGlowR.alpha = 0.5 + 0.35 * Math.sin(time * 3.3 + 0.7);

    // reactions
    if (this.reaction !== 'idle') {
      this.reactionT += dt;
      const rt = this.reactionT;
      if (this.reaction === 'hop') {
        if (rt >= 0.55) {
          this.reaction = 'idle';
          this.squash = 0.12;
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
          // curl up: tuck head slightly, compress torso
          this.torso.scale.set(0.97, 0.94);
          this.headGroup.position.y = this.headBaseY + 8;
          this.tailGroup.rotation = -0.32;
        }
      }
    }
    if (this.reaction !== 'settle') {
      this.headGroup.rotation *= Math.max(0, 1 - dt * 8);
    }
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
