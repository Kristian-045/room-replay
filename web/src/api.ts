export type Phase = 'waiting' | 'recording' | 'retrying' | 'finished' | 'stopped' | 'failed' | 'no_stream';
export type Room = { id: string; name: string; page_url: string; stream_url: string };
export type Recording = {
  id: string;
  room_id: string;
  title: string;
  started_at: number;
  ends_at: number;
  phase: Phase;
  incomplete: boolean;
  playable: boolean;
  duration: number;
  message: string | null;
};
export type Snapshot = { rooms: Room[]; recordings: Recording[] };
export const isActive = (recording: Recording) => ['waiting', 'recording', 'retrying'].includes(recording.phase);

export async function request(path = '/api/state', body?: unknown): Promise<Snapshot> {
  const response = await fetch(path, body === undefined ? {} : {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!response.ok) {
    const text = await response.text();
    let message = text;
    try { message = JSON.parse(text).error ?? text; } catch { /* Text errors from HTTP validation. */ }
    throw new Error(message || `Request failed (${response.status})`);
  }
  return response.json();
}
