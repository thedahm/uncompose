// Generate deterministic stereo test fixtures: seeded-noise WAVs, then FLAC via ffmpeg.
// Noise (not sine) so cross-correlation has a single sharp zero-lag peak.
import { writeFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';

const SAMPLE_RATE = 44100;
const FRAMES = 66150; // 1.5 s exactly
const CHANNELS = 2;

function lcg(seed) {
  let s = seed >>> 0;
  return () => {
    s = (Math.imul(s, 1664525) + 1013904223) >>> 0;
    return s / 4294967296;
  };
}

function makeNoise(seed) {
  const rand = lcg(seed);
  const data = new Int16Array(FRAMES * CHANNELS);
  for (let i = 0; i < data.length; i++) {
    data[i] = Math.round((rand() * 2 - 1) * 0.5 * 32767);
  }
  return data;
}

function wavBytes(pcm) {
  const dataLen = pcm.length * 2;
  const buf = Buffer.alloc(44 + dataLen);
  buf.write('RIFF', 0);
  buf.writeUInt32LE(36 + dataLen, 4);
  buf.write('WAVE', 8);
  buf.write('fmt ', 12);
  buf.writeUInt32LE(16, 16);
  buf.writeUInt16LE(1, 20); // PCM
  buf.writeUInt16LE(CHANNELS, 22);
  buf.writeUInt32LE(SAMPLE_RATE, 24);
  buf.writeUInt32LE(SAMPLE_RATE * CHANNELS * 2, 28);
  buf.writeUInt16LE(CHANNELS * 2, 32);
  buf.writeUInt16LE(16, 34);
  buf.write('data', 36);
  buf.writeUInt32LE(dataLen, 40);
  Buffer.from(pcm.buffer).copy(buf, 44);
  return buf;
}

for (const [name, seed] of [['a', 1], ['b', 2]]) {
  const wav = wavBytes(makeNoise(seed));
  writeFileSync(`fixtures/${name}.wav`, wav);
  execFileSync('ffmpeg', ['-y', '-loglevel', 'error', '-i', `fixtures/${name}.wav`,
    '-c:a', 'flac', `fixtures/${name}.flac`]);
}
console.log(`wrote fixtures: ${FRAMES} frames @ ${SAMPLE_RATE} Hz, ${CHANNELS} ch`);
