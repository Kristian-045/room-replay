import { test } from 'node:test';
import assert from 'node:assert/strict';
import { shouldReturnToNormal, livePosition } from './policy.ts';

test('accelerated viewing returns to normal only near the growing edge', () => {
  assert.equal(shouldReturnToNormal(2, 25, 60, true), false);
  assert.equal(shouldReturnToNormal(2, 53, 60, true), true);
  assert.equal(shouldReturnToNormal(2, 53, 60, false), false);
  assert.equal(shouldReturnToNormal(0.5, 53, 60, true), false);
  assert.equal(shouldReturnToNormal(2, 0, 0, true), false);
  assert.equal(shouldReturnToNormal(2, 53, Infinity, true), false);
  assert.equal(livePosition(5), 0);
});
