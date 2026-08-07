// Spike for uncompose-compare's tested sync contract (issue #66, spike #73).
// Renders the real playback-graph shape (SRC/A/B sources into per-lane gains,
// mid-render A->B crossfade) in OfflineAudioContext on all three engines and
// asserts the three contract points: zero-lag cross-correlation peak, exact
// decode sample counts for FLAC/WAV, and the ~10 ms crossfade bound with
// bit-identical output outside the window.
import { test, expect } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const dir = path.dirname(fileURLToPath(import.meta.url));
const b64 = (f) => readFileSync(path.join(dir, 'fixtures', f)).toString('base64');
const FIXTURES = {
  'a.wav': b64('a.wav'),
  'b.wav': b64('b.wav'),
  'a.flac': b64('a.flac'),
  'b.flac': b64('b.flac'),
};

const SR = 44100;
const FRAMES = 66150;
const SWITCH_SAMPLE = 30870; // 0.7 s
const FADE_SAMPLES = 441; // 10 ms

// Everything below runs in the browser. Decodes fixtures, renders the graph,
// and returns compact metrics; assertions happen back in Node.
async function harness({ fixtures, SR, FRAMES, SWITCH_SAMPLE, FADE_SAMPLES }) {
  const toArrayBuffer = (base64) => {
    const bin = atob(base64);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    return bytes.buffer;
  };

  const ctx = new OfflineAudioContext(2, FRAMES, SR);
  const decoded = {};
  const decodeInfo = {};
  for (const [name, base64] of Object.entries(fixtures)) {
    try {
      const buf = await ctx.decodeAudioData(toArrayBuffer(base64));
      decoded[name] = buf;
      decodeInfo[name] = { length: buf.length, sampleRate: buf.sampleRate, channels: buf.numberOfChannels };
    } catch (e) {
      decodeInfo[name] = { error: String(e) };
    }
  }
  if (!decoded['a.wav'] || !decoded['b.wav']) return { decodeInfo, renderSkipped: true };

  // Real playback-graph shape: three sample-locked sources (SRC muted) into
  // per-lane gains, linear crossfade A->B over FADE_SAMPLES.
  const tSwitch = SWITCH_SAMPLE / SR;
  const tEnd = (SWITCH_SAMPLE + FADE_SAMPLES) / SR;
  const lanes = [
    { buf: decoded['a.wav'], g0: 1, g1: 0 }, // A
    { buf: decoded['b.wav'], g0: 0, g1: 1 }, // B
    { buf: decoded['a.wav'], g0: 0, g1: 0 }, // SRC, muted throughout
  ];
  for (const lane of lanes) {
    const src = ctx.createBufferSource();
    src.buffer = lane.buf;
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(lane.g0, 0);
    gain.gain.setValueAtTime(lane.g0, tSwitch);
    gain.gain.linearRampToValueAtTime(lane.g1, tEnd);
    src.connect(gain).connect(ctx.destination);
    src.start(0);
  }
  const rendered = await ctx.startRendering();

  // Bit-identity outside the fade window, per channel: rendered === A before
  // the switch, rendered === B after the ramp end. Exact float compare.
  const identity = [];
  for (let ch = 0; ch < 2; ch++) {
    const out = rendered.getChannelData(ch);
    const a = decoded['a.wav'].getChannelData(ch);
    const b = decoded['b.wav'].getChannelData(ch);
    let preMismatch = 0, postMismatch = 0;
    let firstPre = -1, firstPost = -1, maxDiff = 0;
    for (let i = 0; i < SWITCH_SAMPLE; i++) {
      if (out[i] !== a[i]) {
        preMismatch++;
        if (firstPre < 0) firstPre = i;
        maxDiff = Math.max(maxDiff, Math.abs(out[i] - a[i]));
      }
    }
    for (let i = SWITCH_SAMPLE + FADE_SAMPLES; i < FRAMES; i++) {
      if (out[i] !== b[i]) {
        postMismatch++;
        if (firstPost < 0) firstPost = i;
        maxDiff = Math.max(maxDiff, Math.abs(out[i] - b[i]));
      }
    }
    identity.push({ ch, preMismatch, postMismatch, firstPre, firstPost, maxDiff });
  }

  // Cross-correlation of the post-switch region against expected candidate B,
  // lags -256..256; the contract wants the peak at exactly lag 0.
  const out = rendered.getChannelData(0);
  const b = decoded['b.wav'].getChannelData(0);
  const start = SWITCH_SAMPLE + FADE_SAMPLES + 256;
  const len = FRAMES - start - 256;
  let bestLag = null, bestVal = -Infinity, zeroVal = null, secondVal = -Infinity;
  for (let lag = -256; lag <= 256; lag++) {
    let sum = 0;
    for (let i = 0; i < len; i++) sum += out[start + i] * b[start + i + lag];
    if (lag === 0) zeroVal = sum;
    if (sum > bestVal) { secondVal = bestVal; bestVal = sum; bestLag = lag; }
    else if (sum > secondVal) { secondVal = sum; }
  }
  const correlation = { bestLag, peakRatio: bestVal / Math.max(secondVal, 1e-12), zeroVal, bestVal };

  return { decodeInfo, identity, correlation };
}

let result;
test.beforeAll(async ({ browser }) => {
  const page = await browser.newPage();
  await page.goto('about:blank');
  const t0 = Date.now();
  result = await page.evaluate(harness, { fixtures: FIXTURES, SR, FRAMES, SWITCH_SAMPLE, FADE_SAMPLES });
  result.elapsedMs = Date.now() - t0;
  await page.close();
});

test('decode-count canary: FLAC and WAV decode to exact sample counts', () => {
  console.log(JSON.stringify(result, null, 2));
  for (const name of ['a.wav', 'b.wav', 'a.flac', 'b.flac']) {
    const info = result.decodeInfo[name];
    expect(info.error, `${name} decode error`).toBeUndefined();
    expect(info.length, `${name} sample count`).toBe(FRAMES);
    expect(info.sampleRate, `${name} sample rate`).toBe(SR);
    expect(info.channels, `${name} channels`).toBe(2);
  }
});

test('zero-offset render: cross-correlation peak at lag 0', () => {
  expect(result.renderSkipped).toBeFalsy();
  expect(result.correlation.bestLag).toBe(0);
  expect(result.correlation.peakRatio).toBeGreaterThan(2);
});

test('crossfade bound: bit-identical output outside the 10 ms window', () => {
  expect(result.renderSkipped).toBeFalsy();
  for (const id of result.identity) {
    expect(id.preMismatch, `ch${id.ch} pre-switch mismatches (first at ${id.firstPre}, maxDiff ${id.maxDiff})`).toBe(0);
    expect(id.postMismatch, `ch${id.ch} post-fade mismatches (first at ${id.firstPost}, maxDiff ${id.maxDiff})`).toBe(0);
  }
});
