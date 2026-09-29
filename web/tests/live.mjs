// Explicit opt-in: this records a short sample from the real A318 stream.
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

if (process.env.DVR_LIVE_TEST !== '1') throw new Error('Set DVR_LIVE_TEST=1 to record a real CESNET sample.');
const base = process.env.DVR_TEST_URL || 'http://127.0.0.1:3000';
const artifacts = await mkdtemp(join(tmpdir(), 'cesnet-live-test-'));
const results = [];
let recordingId;
let browser;

async function api(path = '/api/state', body) {
  const response = await fetch(base + path, body === undefined ? {} : {
    method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body),
  });
  const value = await response.json();
  assert.equal(response.status, 200, JSON.stringify(value));
  return value;
}
async function until(check, description, timeout = 90000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    if (await check()) return;
    await new Promise(done => setTimeout(done, 1000));
  }
  throw new Error(`Timed out: ${description}`);
}
const active = recording => ['waiting', 'recording', 'retrying'].includes(recording.phase);

try {
  const initial = await api();
  assert.ok(!initial.recordings.some(active), 'Do not interrupt an existing recording');
  const room = initial.rooms.find(room => room.page_url === 'https://live.cesnet.cz/munifia318.html');
  assert.ok(room, 'A318 must be saved');
  const started = await api('/api/recordings', { room_id: room.id, ends_at: Math.floor(Date.now() / 1000) + 240, subject_id: null });
  recordingId = started.recordings.find(active).id;
  console.log(`Recording live sample: ${recordingId}`);
  await until(async () => (await api()).recordings.find(recording => recording.id === recordingId).duration >= 45, '45 seconds of live capture');
  for (const executablePath of ['/usr/bin/chromium', '/usr/bin/brave']) {
    browser = await chromium.launch({ executablePath, headless: true, args: ['--autoplay-policy=no-user-gesture-required'] });
    const page = await browser.newPage({ viewport: { width: 1360, height: 950 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(base);
    await page.locator(`[data-recording-id="${recordingId}"]`).getByRole('button', { name: 'Watch', exact: true }).click();
    await page.waitForFunction(() => document.querySelector('video')?.readyState >= 2);
    const before = await page.evaluate(async () => {
      const video = document.querySelector('video');
      video.currentTime = 0;
      await video.play();
      return { time: video.currentTime, audio: video.webkitAudioDecodedByteCount, video: video.webkitVideoDecodedByteCount };
    });
    for (let step = 0; step < 3; step++) await page.getByRole('button', { name: 'Increase speed', exact: true }).click();
    await page.waitForTimeout(6000);
    const after = await page.evaluate(() => {
      const video = document.querySelector('video');
      return { time: video.currentTime, speed: video.playbackRate, audio: video.webkitAudioDecodedByteCount, video: video.webkitVideoDecodedByteCount, width: video.videoWidth, height: video.videoHeight };
    });
    assert.equal(after.speed, 2);
    assert.ok(after.time - before.time > 9 && after.time - before.time < 15, JSON.stringify({ before, after }));
    assert.ok(after.audio > before.audio && after.video > before.video, 'Audio and video must continue decoding at 2x');
    await page.getByLabel('Seek', { exact: true }).fill('0');
    assert.ok(await page.evaluate(() => document.querySelector('video').currentTime < 2));
    await page.getByRole('button', { name: 'Go live', exact: true }).click();
    for (let step = 0; step < 3; step++) await page.getByRole('button', { name: 'Increase speed', exact: true }).click();
    await page.waitForFunction(() => document.querySelector('video').playbackRate === 1, undefined, { timeout: 20000 });
    await page.getByText('Caught up. Playing at 1×.', { exact: true }).waitFor();
    assert.deepEqual(errors, []);
    const name = executablePath.split('/').at(-1);
    await page.screenshot({ path: join(artifacts, `${name}.png`), fullPage: true });
    results.push({ browser: name, before, after, catchUp: true, seekToBeginning: true });
    console.log(`PASS ${name}: live recording, 2x audio/video decode, seek, return to 1x`);
    await browser.close();
    browser = undefined;
  }
  await api(`/api/recordings/${recordingId}/stop`, {});
  const playlist = await (await fetch(`${base}/media/${recordingId}/index.m3u8`)).text();
  assert.ok(playlist.endsWith('#EXT-X-ENDLIST\n'));
  console.log(`PASS: sample finalized; remains in library as ${recordingId}`);
} finally {
  if (browser) await browser.close();
  if (recordingId) {
    const recording = (await api()).recordings.find(recording => recording.id === recordingId);
    if (recording && active(recording)) await api(`/api/recordings/${recordingId}/stop`, {});
  }
  await writeFile(join(artifacts, 'results.json'), JSON.stringify({ recordingId, results }, null, 2));
  console.log(`Artifacts: ${artifacts}`);
}
