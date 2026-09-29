export type Phase = 'waiting' | 'recording' | 'retrying' | 'finished' | 'stopped' | 'failed' | 'no_stream';
export type Room = { id: string; name: string; page_url: string; stream_url: string };
export type Subject = {
  id: string;
  name: string;
  weekly_slot: { day: string; starts_at: string; ends_at: string; room: string; room_page_url: string | null };
};
export type Recording = {
  id: string;
  room_id: string;
  title: string;
  room_name: string;
  subject_id: string | null;
  started_at: number;
  ends_at: number;
  phase: Phase;
  incomplete: boolean;
  playable: boolean;
  duration: number;
  message: string | null;
};
export type Snapshot = { rooms: Room[]; subjects: Subject[]; recordings: Recording[] };
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
