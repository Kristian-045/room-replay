import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { resolve, join, basename } from 'node:path';
import { fileURLToPath } from 'node:url';
import { once } from 'node:events';

const root = resolve(fileURLToPath(new URL('../..', import.meta.url)));
const temporary = await mkdtemp(join(tmpdir(), 'cesnet-dvr-test-'));
const source = join(temporary, 'source');
await mkdir(source);
const children = [];
let backend;
let browser;
const logs = [];
const fixture = createServer(async (request, response) => {
  try {
    const name = basename(new URL(request.url, 'http://localhost').pathname);
    const content = await readFile(join(source, name));
    response.writeHead(200, { 'Content-Type': name.endsWith('.m3u8') ? 'application/vnd.apple.mpegurl' : 'video/mp2t', 'Cache-Control': 'no-store' });
    response.end(content);
  } catch { response.writeHead(404); response.end(); }
});
fixture.listen(0, '127.0.0.1');
await once(fixture, 'listening');
const sourceUrl = `http://127.0.0.1:${fixture.address().port}/live.m3u8`;
const reservation = createServer();
reservation.listen(0, '127.0.0.1');
await once(reservation, 'listening');
const port = reservation.address().port;
await new Promise(done => reservation.close(done));
const base = `http://127.0.0.1:${port}`;

function launch(command, args, options = {}) {
  const child = spawn(command, args, { cwd: root, stdio: ['pipe', 'pipe', 'pipe'], ...options });
  child.stdout.on('data', data => logs.push(data.toString()));
  child.stderr.on('data', data => logs.push(data.toString()));
  children.push(child);
  return child;
}
async function stop(child) {
  if (!child || child.exitCode !== null || child.signalCode !== null) return;
  child.kill('SIGTERM');
  await Promise.race([once(child, 'exit'), new Promise(done => setTimeout(done, 8000))]);
  if (child.exitCode === null && child.signalCode === null) { child.kill('SIGKILL'); await once(child, 'exit'); }
}
async function waitFor(check, description, milliseconds = 30000) {
  const end = Date.now() + milliseconds;
  while (Date.now() < end) {
    try { const result = await check(); if (result) return result; } catch { /* Retry startup and publication races. */ }
    await new Promise(done => setTimeout(done, 500));
  }
  throw new Error(`Timed out: ${description}`);
}
async function api(path = '/api/state', body) {
  const response = await fetch(base + path, body === undefined ? {} : { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) });
  const value = await response.json();
  assert.equal(response.status, 200, JSON.stringify(value));
  return value;
}
async function startBackend() {
  backend = launch(join(root, 'target/debug/cesnet-dvr'), [], {
    env: { ...process.env, DVR_BIND: `127.0.0.1:${port}`, DVR_DATA_DIR: join(temporary, 'data'), DVR_WEB_DIR: join(root, 'web/dist'), DVR_ALLOW_TEST_SOURCES: '1' },
  });
  await waitFor(() => api(), 'backend startup');
}

try {
  launch('ffmpeg', ['-hide_banner', '-loglevel', 'error', '-re', '-f', 'lavfi', '-i', 'testsrc2=size=640x360:rate=25', '-f', 'lavfi', '-i', 'sine=frequency=440:sample_rate=48000', '-c:v', 'libx264', '-preset', 'ultrafast', '-g', '50', '-keyint_min', '50', '-sc_threshold', '0', '-pix_fmt', 'yuv420p', '-c:a', 'aac', '-f', 'hls', '-hls_time', '2', '-hls_list_size', '0', '-hls_flags', 'temp_file', '-hls_segment_filename', join(source, 'source%06d.ts'), join(source, 'live.m3u8')]);
  await startBackend();
  browser = await chromium.launch({ executablePath: process.env.DVR_TEST_BROWSER || '/usr/bin/chromium', headless: true, args: ['--autoplay-policy=no-user-gesture-required'] });
  const page = await browser.newPage({ viewport: { width: 1360, height: 950 } });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto(base);
  assert.equal((await api()).rooms.length, 5, 'FI defaults must be preloaded');
  await page.getByRole('button', { name: '+ Add', exact: true }).click();
  await page.getByLabel('Room name', { exact: true }).fill('Test lecture');
  await page.getByLabel('CESNET page or playlist URL').fill(sourceUrl);
  await page.getByRole('button', { name: 'Save room', exact: true }).click();
  await waitFor(async () => (await api()).rooms.some(room => room.name === 'Test lecture'), 'room persistence');
  await page.getByRole('button', { name: 'Record now' }).click();
  let state = await waitFor(async () => { const state = await api(); return state.recordings.length && state; }, 'recording start');
  const id = state.recordings[0].id;
  const duplicate = await fetch(base + '/api/recordings', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ room_id: state.rooms.find(room => room.name === 'Test lecture').id, ends_at: Math.floor(Date.now() / 1000) + 120 }) });
  assert.equal(duplicate.status, 400, 'Concurrent start must be rejected');
  state = await waitFor(async () => { const state = await api(); return state.recordings[0].duration >= 24 && state; }, '24 seconds of captured video', 60000);
  console.log('PASS: room saved, recording starts, concurrent start rejected, playlist grows');
  await page.getByRole('button', { name: 'Watch', exact: true }).click();
  await page.waitForFunction(() => document.querySelector('video')?.readyState >= 2);
  await page.evaluate(async () => { const video = document.querySelector('video'); video.currentTime = 0; await video.play(); });
  await page.getByLabel('Playback speed').selectOption('2');
  await page.waitForTimeout(4500);
  const accelerated = await page.evaluate(() => { const v = document.querySelector('video'); return { time: v.currentTime, speed: v.playbackRate, audioBytes: v.webkitAudioDecodedByteCount, videoBytes: v.webkitVideoDecodedByteCount }; });
  assert.equal(accelerated.speed, 2);
  assert.ok(accelerated.time > 6 && accelerated.time < 13, JSON.stringify(accelerated));
  assert.ok(accelerated.audioBytes > 0 && accelerated.videoBytes > 0, 'Both audio and video must decode at 2x');
  await page.getByRole('button', { name: 'Start over' }).click();
  assert.ok(await page.evaluate(() => document.querySelector('video').currentTime < 2), 'Seeking to the beginning must work');
  await page.getByRole('button', { name: 'Go live', exact: true }).click();
  await page.getByLabel('Playback speed').selectOption('2');
  await page.waitForFunction(() => document.querySelector('video').playbackRate === 1, undefined, { timeout: 15000 });
  await page.getByText('Caught up. Playing at 1×.', { exact: true }).waitFor();
  console.log('PASS: growing playback, seeking, 2x audio/video decode, catch-up to 1x');
  await page.screenshot({ path: join(temporary, 'desktop.png'), fullPage: true });
  await stop(backend);
  const shutdownPlaylist = await readFile(join(temporary, 'data/recordings', id, 'index.m3u8'), 'utf8');
  const beforeRestart = [...shutdownPlaylist.matchAll(/#EXTINF:([\d.]+)/g)].reduce((total, match) => total + Number(match[1]), 0);
  await startBackend();
  await waitFor(async () => (await api()).recordings[0].duration >= beforeRestart + 8, 'capture resumes after restart', 40000);
  const resumed = (await api()).recordings[0];
  assert.equal(resumed.id, id);
  assert.equal(resumed.incomplete, true);
  const playlist = await (await fetch(`${base}/media/${id}/index.m3u8`)).text();
  assert.ok(playlist.includes('#EXT-X-DISCONTINUITY'), 'Restarted capture needs a discontinuity');
  console.log('PASS: restart resumes the same recording and publishes a discontinuity');
  await api(`/api/recordings/${id}/stop`, {});
  const finished = await (await fetch(`${base}/media/${id}/index.m3u8`)).text();
  assert.ok(finished.endsWith('#EXT-X-ENDLIST\n'));
  await stop(backend);
  await startBackend();
  assert.equal((await api()).recordings[0].phase, 'stopped');
  await page.reload();
  await page.getByRole('button', { name: 'Watch', exact: true }).click();
  await page.waitForFunction(() => document.querySelector('video')?.readyState >= 2);
  await page.evaluate(async (boundary) => { const v = document.querySelector('video'); v.currentTime = Math.max(0, boundary - 2); await v.play(); }, beforeRestart);
  await page.getByLabel('Playback speed').selectOption('2');
  await page.waitForFunction(() => document.querySelector('video')?.ended, undefined, { timeout: 20000 });
  assert.equal(await page.evaluate(() => document.querySelector('video').playbackRate), 2, 'Finished recording must retain 2x near its end');
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({ path: join(temporary, 'mobile.png'), fullPage: true });
  assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'Mobile layout must not overflow');
  assert.deepEqual(errors, []);
  console.log('PASS: stop finalizes playlist, persists across restart, finished playback crosses attempts, mobile layout fits');
  console.log(`Artifacts: ${temporary}`);
} catch (error) {
  console.error(logs.slice(-25).join(''));
  throw error;
} finally {
  if (browser) await browser.close();
  for (const child of children.reverse()) await stop(child);
  fixture.closeAllConnections();
  await new Promise(done => fixture.close(done));
  await writeFile(join(temporary, 'process.log'), logs.join(''));
}
