export function shouldReturnToNormal(speed: number, position: number, edge: number, live: boolean): boolean {
  return live && speed > 1 && Number.isFinite(edge) && edge > 0 && position >= edge - 8;
}

export function livePosition(edge: number): number {
  return Math.max(0, edge - 8);
}
