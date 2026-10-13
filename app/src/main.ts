import './app.css';
import { Game } from './core/Game';

function showError(err: unknown): void {
  const div = document.createElement('div');
  div.className = 'boot-error';
  div.textContent = `SolanaPets failed to start: ${err instanceof Error ? err.message : String(err)}`;
  document.body.appendChild(div);
}

async function boot(): Promise<void> {
  const root = document.getElementById('app');
  if (!root) throw new Error('missing #app element');
  const game = new Game(root);
  await game.start();

  // PWA service worker (best-effort; failures must never break boot).
  if ('serviceWorker' in navigator) {
    try {
      await navigator.serviceWorker.register('./sw.js');
    } catch {
      /* offline support unavailable — game still runs */
    }
  }
}

boot().catch(showError);
