const prefix = 'room-replay:';
export function readSaved(key: string): string | null {
  try { return localStorage.getItem(prefix + key); } catch { return null; }
}
export function save(key: string, value: string) {
  try { localStorage.setItem(prefix + key, value); } catch { /* Playback still works when storage is unavailable. */ }
}
export function savedPosition(id: string): number {
  const value = Number(readSaved('position:' + id));
  return Number.isFinite(value) && value > 0 ? value : 0;
}
