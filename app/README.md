# SolanaPets — Game Client

The visual heart of SolanaPets. Mobile-first 2.5D pet habitats built with
PixiJS v8 + TypeScript + Vite, packaged as a PWA.

> **Status:** Foundation scaffold (Step 9). Luna's moonlit cove is live with
> procedural art. Real sprite sheets slot in via `src/pet/PetSprite.ts`
> with zero game-logic changes. Solana calls are stubs (`src/solana/stubs.ts`)
> until the wallet adapter lands.

## Run it

```bash
npm install
npm run dev      # → http://localhost:5173 (use --host to test on your phone)
npm run build    # typecheck + production build
```

Open on a phone for the real feel — everything is touch-first.

## What's here

| Path | What |
|---|---|
| `src/core/Game.ts` | Orchestrator: Pixi app, main loop, care flow, parallax input |
| `src/core/textures.ts` | Runtime-generated gradients, glows, dots, hearts (no binary assets yet) |
| `src/habitat/MoonlitCove.ts` | Luna's habitat — 5 parallax layers, animated water, waterfall, fireflies, mist |
| `src/habitat/DayNightCycle.ts` | 4-minute day/night palette cycle (pure logic, habitat-agnostic) |
| `src/habitat/Parallax.ts` | Pointer-driven parallax easing |
| `src/pet/Luna.ts` | Procedural Luna: breathing, blinking, tail sway, care reactions |
| `src/pet/MoodEngine.ts` | Stats → mood logic (decay + care effects, matches on-chain semantics) |
| `src/pet/PetSprite.ts` | **The art seam** — sprite-sheet contract for production art |
| `src/effects/Particles.ts` | Pooled particle system (bursts, fountains, ambient) |
| `src/ui/CareBar.ts` | 5 thumb-friendly care buttons (DOM) |
| `src/ui/Hud.ts` | Name, mood pill, stat bars, day/night phase (DOM) |
| `src/solana/stubs.ts` | Logged stand-ins for the 5 pet-registry care instructions |

## Architecture

```
tap CareBar → Solana stub (logs real instruction shape)
            → MoodEngine.applyCare (stats + bond XP)
            → Luna.reactTo (hop / eat / settle animation)
            → ParticleSystem burst (hearts, sparkles, droplets)
            → HUD refresh (mood pill + stat bars)
```

**Habitat spec** (for the other 7 habitats): extend the layer pattern from
`MoonlitCove` — `sky → far → mid → water/ground → fore`, each a `Container`
with a depth factor, restyled from `DayNightCycle.palette` on a throttle,
animated in `update(dt, cycle, px, py)`. Pet anchor via `getPetAnchor()`.

**Pet spec** (for the other 7 pets): implement the `Pet` interface
(`container`, `update`, `reactTo`, `setMood`). Procedural now, sprite sheets
later — `PetSprite.loadPetSprite()` is the documented swap point.

## Roadmap

- [ ] Wallet adapter + real `pet-registry` care transactions (stubs ready)
- [ ] Production sprite sheets for Luna (front/side/back, expressions)
- [ ] 7 more habitats + pets (Lenny, Lucy, Loboe, Lilah, Leo, Lola, Loki)
- [ ] Breeding / marketplace / raffle UI flows
- [ ] Passkey sign-in + account IDs
- [ ] Sound design + haptics
- [ ] Lockscreen widget API
