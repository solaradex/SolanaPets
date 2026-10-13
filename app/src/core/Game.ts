/**
 * Game — orchestrates the whole client.
 *
 * Owns the PixiJS Application, the habitat (MoonlitCove), the pet (Luna),
 * the day/night cycle, parallax input, the particle system, and the DOM
 * UI (HUD + CareBar). The care flow is:
 *
 *   tap button → Solana stub call → MoodEngine.applyCare →
 *   Luna.reactTo → particle burst → HUD refresh
 *
 * When the wallet adapter lands, only the stub imports change — the flow
 * stays identical.
 */
import { Application, Texture } from 'pixi.js';
import { CONFIG, CareAction, Mood } from '../config';
import { heartTexture, softDotTexture } from '../core/textures';
import { HabitatLike, SceneDef, sceneFromUrl } from '../core/scenes';
import { ParticleSystem } from '../effects/Particles';
import { DayNightCycle } from '../habitat/DayNightCycle';
import { Parallax } from '../habitat/Parallax';
import { MoodEngine } from '../pet/MoodEngine';
import { Pet } from '../pet/PetSprite';
import { cleanPet, feedPet, giveAffection, playWithPet, restPet } from '../solana/stubs';
import { CareBar } from '../ui/CareBar';
import { Hud } from '../ui/Hud';

const CARE_CALLS: Record<CareAction, (petId: number) => Promise<{ signature: string }>> = {
  feed: feedPet,
  play: playWithPet,
  clean: cleanPet,
  rest: restPet,
  love: giveAffection,
};

const clamp = (v: number, a: number, b: number) => Math.max(a, Math.min(b, v));

export class Game {
  private app!: Application;
  private scene!: SceneDef;
  private cove!: HabitatLike;
  private luna!: Pet;
  private mood = new MoodEngine();
  private cycle!: DayNightCycle;
  private parallax = new Parallax();
  private particles!: ParticleSystem;
  private careBar!: CareBar;
  private hud!: Hud;
  private uiRoot = document.createElement('div');

  private dotTex!: Texture;
  private heartTex!: Texture;

  private root: HTMLElement;
  private petId = 1; // set from the active scene in start()
  private petScale = 1;
  private petBaseX = 0;
  private petBaseY = 0;
  private elapsed = 0;
  private busy = false;
  private lastMood: Mood = 'happy';
  private hudTimer = 0;
  private glintTimer = 0;

  constructor(root: HTMLElement) {
    this.root = root;
    this.uiRoot.className = 'ui-root';
  }

  async start(): Promise<void> {
    this.app = new Application();
    await this.app.init({
      resizeTo: window,
      backgroundColor: '#0a0e1a',
      antialias: true,
      resolution: Math.min(window.devicePixelRatio || 1, 2),
    });
    this.app.canvas.classList.add('game-canvas');
    this.root.appendChild(this.app.canvas);

    this.cycle = new DayNightCycle(CONFIG.dayLengthSec);
    this.scene = sceneFromUrl();
    this.cove = this.scene.makeHabitat();
    this.app.stage.addChild(this.cove);

    this.luna = this.scene.makePet();
    this.app.stage.addChild(this.luna.container);
    this.petId = this.scene.petId;

    this.particles = new ParticleSystem(500);
    this.app.stage.addChild(this.particles.container);

    this.dotTex = softDotTexture();
    this.heartTex = heartTexture();

    this.hud = new Hud(this.scene.displayName);
    this.careBar = new CareBar((a) => void this.onCare(a));
    this.uiRoot.append(this.hud.el, this.careBar.el);
    document.body.appendChild(this.uiRoot);

    window.addEventListener('pointermove', (e) => {
      this.parallax.setTarget((e.clientX / window.innerWidth) * 2 - 1, (e.clientY / window.innerHeight) * 2 - 1);
    });
    window.addEventListener('resize', () => this.layout());
    this.layout();

    this.app.ticker.add((ticker) => {
      const dt = Math.min(ticker.deltaTime, 4) / 60;
      this.update(dt);
    });
  }

  private layout(): void {
    const w = window.innerWidth;
    const h = window.innerHeight;
    this.cove.layout(w, h);
    const anchor = this.cove.getPetAnchor();
    this.petScale = clamp(Math.min(w, h) / 700, CONFIG.petScaleMin, CONFIG.petScaleMax);
    this.luna.container.scale.set(this.petScale);
    // Luna's local origin is body-centre; feet sit ~96px below it.
    this.petBaseX = anchor.x;
    this.petBaseY = anchor.y - 96 * this.petScale;
    this.luna.container.position.set(this.petBaseX, this.petBaseY);
  }

  private async onCare(action: CareAction): Promise<void> {
    if (this.busy) return;
    this.busy = true;
    this.careBar.setBusy(action);
    try {
      await CARE_CALLS[action](this.petId);
      this.mood.applyCare(action);
      this.luna.reactTo(action);
      this.careBurst(action);
    } catch (err) {
      console.error('[game] care action failed', err);
    } finally {
      this.busy = false;
      this.careBar.setBusy(null);
    }
  }

  /** Particle celebration at Luna's head, themed per action. */
  private careBurst(action: CareAction): void {
    const s = this.petScale;
    const x = this.luna.container.position.x + 8 * s;
    const y = this.luna.container.position.y - 80 * s;
    const add: 'add' = 'add';
    switch (action) {
      case 'feed':
        this.particles.fountain(x, y + 30 * s, {
          count: 14, texture: this.dotTex, tint: [0xd9a45b, 0xffd98a],
          speedMin: 60, speedMax: 170, lifeMin: 0.5, lifeMax: 1,
          sizeMin: 5, sizeMax: 10, gravity: 320, blendMode: add,
        });
        break;
      case 'play':
        this.particles.burst(x, y, {
          count: 22, texture: this.dotTex, tint: [0x7fd4ff, 0xffe08a, 0xffffff],
          speedMin: 80, speedMax: 240, lifeMin: 0.6, lifeMax: 1.2,
          sizeMin: 6, sizeMax: 14, gravity: 120, blendMode: add,
        });
        this.particles.burst(x, y - 20 * s, {
          count: 6, texture: this.heartTex, tint: 0xff8ab3,
          speedMin: 40, speedMax: 110, lifeMin: 0.9, lifeMax: 1.5,
          sizeMin: 14, sizeMax: 22, gravity: -40,
        });
        break;
      case 'clean':
        this.particles.fountain(x, y, {
          count: 26, texture: this.dotTex, tint: [0x9fdcff, 0xffffff],
          speedMin: 90, speedMax: 220, lifeMin: 0.5, lifeMax: 1,
          sizeMin: 5, sizeMax: 11, gravity: 380, blendMode: add,
        });
        break;
      case 'rest':
        this.particles.spawn({
          x, y: y - 10 * s, count: 8, texture: this.dotTex, tint: 0xb9a7e6,
          speedMin: 12, speedMax: 30, angleMin: 250, angleMax: 290,
          lifeMin: 1.4, lifeMax: 2.2, sizeMin: 8, sizeMax: 14,
          gravity: -24, swayAmp: 18, swayFreq: 2.2, alpha: 0.8, blendMode: add,
        });
        break;
      case 'love':
        this.particles.burst(x, y, {
          count: 16, texture: this.heartTex, tint: [0xff8ab3, 0xffb3c9, 0xffffff],
          speedMin: 60, speedMax: 180, lifeMin: 0.9, lifeMax: 1.6,
          sizeMin: 14, sizeMax: 24, gravity: -30, drag: 0.985,
        });
        break;
    }
  }

  private update(dt: number): void {
    this.elapsed += dt;
    const t = this.elapsed;

    this.cycle.update(dt);
    this.parallax.update(dt);
    this.cove.update(dt, this.cycle, this.parallax.x, this.parallax.y);

    this.mood.update(dt);
    const m = this.mood.mood;
    if (m !== this.lastMood) {
      this.lastMood = m;
      this.luna.setMood(m);
    }
    this.luna.update(dt, t);

    // pet rides the parallax slightly, like a mid-foreground element
    this.luna.container.position.x =
      this.petBaseX - this.parallax.x * CONFIG.parallaxStrength * 0.5;
    this.luna.container.position.y =
      this.petBaseY - this.parallax.y * CONFIG.parallaxStrength * 0.22;

    // ambient water glints
    this.glintTimer += dt;
    if (this.glintTimer > 0.45) {
      this.glintTimer = 0;
      const wb = this.cove.getWaterBounds();
      this.particles.spawn({
        x: Math.random() * wb.w,
        y: wb.y + 10 + Math.random() * (wb.h - 20),
        count: 2, texture: this.dotTex, tint: 0xbfe6ff,
        speedMin: 6, speedMax: 18, angleMin: 265, angleMax: 275,
        lifeMin: 0.8, lifeMax: 1.6, sizeMin: 4, sizeMax: 9,
        alpha: 0.5, blendMode: 'add',
      });
    }

    this.particles.update(dt);

    this.hudTimer += dt;
    if (this.hudTimer > 0.25) {
      this.hudTimer = 0;
      this.hud.update(this.mood.stats, m, this.cycle.phaseName());
    }
  }
}
