/**
 * Procedural texture helpers.
 *
 * The game client generates gradient/glow/dot textures at runtime from
 * <canvas> 2D drawings. This keeps the scaffold fully self-contained
 * (no binary assets) while real sprite art is produced. All helpers are
 * pure functions returning PixiJS Textures.
 */
import { Texture } from 'pixi.js';

function canvasTexture(canvas: HTMLCanvasElement): Texture {
  const tex = Texture.from(canvas);
  return tex;
}

/** Vertical linear gradient texture (stretched across the target). */
export function gradientTexture(
  stops: Array<{ offset: number; color: string }>,
  width = 4,
  height = 512,
): Texture {
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d')!;
  const g = ctx.createLinearGradient(0, 0, 0, height);
  for (const s of stops) g.addColorStop(s.offset, s.color);
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, width, height);
  return canvasTexture(canvas);
}

/**
 * Radial glow texture. `inner` should be an rgba() string so it can
 * fade to transparent at the edge.
 */
export function radialGlowTexture(inner: string, size = 256): Texture {
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d')!;
  const g = ctx.createRadialGradient(size / 2, size / 2, 0, size / 2, size / 2, size / 2);
  g.addColorStop(0, inner);
  g.addColorStop(0.35, inner);
  g.addColorStop(1, 'rgba(0,0,0,0)');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, size, size);
  return canvasTexture(canvas);
}

/** Small soft white dot — tinted per-particle at spawn time. */
export function softDotTexture(size = 64): Texture {
  return radialGlowTexture('rgba(255,255,255,1)', size);
}

/** Tiny heart shape for affection bursts. */
export function heartTexture(size = 64): Texture {
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d')!;
  const s = size / 64;
  ctx.scale(s, s);
  ctx.fillStyle = '#ffffff';
  ctx.beginPath();
  ctx.moveTo(32, 54);
  ctx.bezierCurveTo(6, 34, 4, 18, 16, 12);
  ctx.bezierCurveTo(24, 8, 30, 12, 32, 18);
  ctx.bezierCurveTo(34, 12, 40, 8, 48, 12);
  ctx.bezierCurveTo(60, 18, 58, 34, 32, 54);
  ctx.fill();
  // soften edges
  ctx.globalCompositeOperation = 'destination-in';
  const g = ctx.createRadialGradient(32, 32, 8, 32, 32, 32);
  g.addColorStop(0, 'rgba(0,0,0,1)');
  g.addColorStop(1, 'rgba(0,0,0,0.85)');
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, 64, 64);
  return canvasTexture(canvas);
}

/** Convert a hex number to a css rgb() string. */
export function css(hex: number): string {
  const r = (hex >> 16) & 255;
  const g = (hex >> 8) & 255;
  const b = hex & 255;
  return `rgb(${r},${g},${b})`;
}

/** Convert a hex number to an rgba() string with the given alpha. */
export function cssA(hex: number, alpha: number): string {
  const r = (hex >> 16) & 255;
  const g = (hex >> 8) & 255;
  const b = hex & 255;
  return `rgba(${r},${g},${b},${alpha})`;
}
