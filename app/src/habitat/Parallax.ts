/**
 * Pointer-driven parallax.
 * Normalised pointer position (-1..1) eases toward a target so layers
 * glide instead of snapping. Habitats apply per-layer depth factors.
 */
export class Parallax {
  x = 0;
  y = 0;
  private tx = 0;
  private ty = 0;

  /** nx, ny in -1..1 (pointer offset from screen centre). */
  setTarget(nx: number, ny: number): void {
    this.tx = Math.max(-1, Math.min(1, nx));
    this.ty = Math.max(-1, Math.min(1, ny));
  }

  update(dt: number): void {
    const k = 1 - Math.exp(-dt * 3.2);
    this.x += (this.tx - this.x) * k;
    this.y += (this.ty - this.y) * k;
  }
}
