// Read-only check against an app with a recording of at least 40 seconds.
import { chromium } from 'playwright';
import assert from 'node:assert/strict';
import { mkdtemp } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const base = process.env.DVR_TEST_URL || 'http://127.0.0.1:3000';
const state = await (await fetch(`${base}/api/state`)).json();
const recording = state.recordings.find(item => item.playable && item.duration >= 40);
assert.ok(recording, 'Need a playable recording of at least 40 seconds');
const artifacts = await mkdtemp(join(tmpdir(), 'room-replay-mobile-'));
for (const executablePath of ['/usr/bin/chromium', '/usr/bin/brave']) {
  const browser = await chromium.launch({ executablePath, headless: true });
  try {
    const context = await browser.newContext({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true, colorScheme: 'dark' });
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(base);
    assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark', 'System preference should default to dark');
    await page.getByLabel('Color theme').selectOption('light');
    assert.equal(await page.locator('html').getAttribute('data-theme'), 'light');
    await page.getByLabel('Color theme').selectOption('dark');
    await page.locator(`[data-recording-id="${recording.id}"]`).getByRole('button', { name: 'Watch', exact: true }).click();
    await page.waitForFunction(() => document.querySelector('video')?.readyState >= 2);
    const video = page.locator('video');
    const frame = page.locator('[data-player-frame]');
    const controls = page.locator('[data-player-controls]');
    const seek = controls.getByLabel('Seek', { exact: true });
    await seek.fill('20');
    await page.waitForFunction(() => Math.abs(document.querySelector('video').currentTime - 20) < 0.3);
    await frame.scrollIntoViewIfNeeded();
    const box = await frame.boundingBox();
    const tap = side => page.touchscreen.tap(box.x + box.width * (side === 'left' ? 0.15 : 0.85), box.y + box.height * 0.2);
    await tap('right'); await tap('right');
    await page.waitForFunction(() => Math.abs(document.querySelector('video').currentTime - 30) < 0.3);
    assert.equal(await video.evaluate(v => v.paused), true, 'Double tap must not toggle playback');
    await tap('right');
    await page.waitForFunction(() => Math.abs(document.querySelector('video').currentTime - 40) < 0.3);
    assert.match(await page.locator('[data-seek-feedback]').innerText(), /20 seconds/);
    await page.waitForTimeout(850);
    await tap('left'); await tap('left');
    await page.waitForFunction(() => Math.abs(document.querySelector('video').currentTime - 30) < 0.3);
    await seek.fill('2');
    await page.waitForTimeout(850);
    await tap('left'); await tap('left');
    await page.waitForFunction(() => document.querySelector('video').currentTime < 0.3);
    await page.waitForTimeout(850);
    await seek.fill(String(Math.floor(recording.duration - 2)));
    await tap('right'); await tap('right');
    await page.waitForFunction(() => Math.abs(document.querySelector('video').currentTime - document.querySelector('video').duration) < 0.3);
    await controls.getByRole('button', { name: 'Adjust brightness' }).click();
    await page.getByLabel('Video brightness', { exact: true }).fill('5');
    assert.equal(await video.evaluate(v => getComputedStyle(v).filter), 'brightness(0.05)');
    assert.equal(await controls.evaluate(el => getComputedStyle(el).filter), 'none', 'Controls must remain readable');
    await controls.getByRole('button', { name: 'Fullscreen', exact: true }).click();
    await page.waitForFunction(() => !!document.fullscreenElement);
    assert.equal(await page.locator(':fullscreen video').evaluate(v => getComputedStyle(v).filter), 'brightness(0.05)');
    await controls.getByRole('button', { name: 'Exit fullscreen', exact: true }).click();
    for (const width of [320, 390, 768]) {
      await page.setViewportSize({ width, height: 844 });
      await frame.scrollIntoViewIfNeeded();
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth));
      assert.ok(await frame.evaluate(el => {
        const frame = el.getBoundingClientRect();
        return [...el.querySelectorAll('button,input')].filter(item => getComputedStyle(item).display !== 'none').every(item => {
          const box = item.getBoundingClientRect();
          return box.left >= frame.left - 1 && box.right <= frame.right + 1 && box.top >= frame.top - 1 && box.bottom <= frame.bottom + 1;
        });
      }), `Controls must fit at ${width}px`);
      await page.screenshot({ path: join(artifacts, `${executablePath.split('/').at(-1)}-${width}.png`), fullPage: true });
    }
    await page.reload();
    await page.waitForFunction(() => document.querySelector('video')?.readyState >= 2);
    assert.equal(await video.evaluate(v => getComputedStyle(v).filter), 'brightness(0.05)', 'Brightness survives reload');
    assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark', 'Theme survives reload');
    // Real touchscreen single taps should reveal/hide controls during playback.
    await seek.fill('10');
    await controls.getByRole('button', { name: 'Play', exact: true }).click();
    await page.waitForFunction(() => !document.querySelector('video').paused);
    const updated = await frame.boundingBox();
    await page.touchscreen.tap(updated.x + updated.width / 2, updated.y + updated.height * 0.2);
    await page.waitForFunction(() => getComputedStyle(document.querySelector('[data-player-controls]')).opacity === '0');
    await page.touchscreen.tap(updated.x + updated.width / 2, updated.y + updated.height * 0.2);
    await page.waitForFunction(() => getComputedStyle(document.querySelector('[data-player-controls]')).opacity === '1');
    assert.equal(await video.evaluate(v => v.paused), false, 'Single tap reveals controls without pausing');
    assert.deepEqual(errors, []);
    console.log(`PASS ${executablePath}: touch seek/repeated taps/bounds, fullscreen dimming, theme and brightness persistence, 320/390/768px controls, single taps`);
  } finally { await browser.close(); }
}
console.log(`Artifacts: ${artifacts}`);
