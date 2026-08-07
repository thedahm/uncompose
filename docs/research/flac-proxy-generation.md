# FLAC proxy generation for Compare playback

Research for [#74](https://github.com/thedahm/uncompose/issues/74). Question: how should
Compare's Rust backend produce the browser-safe playback proxies decided in
[#59](https://github.com/thedahm/uncompose/issues/59) (serve FLAC, WAV fallback, never
lossy), given the [#60](https://github.com/thedahm/uncompose/issues/60) decision (Rust
backend, symphonia in-process decode, no ffmpeg runtime dependency, maturin PyPI wheels,
Linux-only v0.1)? symphonia decodes FLAC but does not encode it, so an encoder must come
from somewhere else.

## TL;DR recommendation

Use **`flacenc`** (pure Rust, Apache-2.0) as the FLAC encoder and **`hound`** for the WAV
fallback. Quantize float/32-bit sources to **24-bit integer** FLAC at the **source sample
rate** (no resampling). No C toolchain enters the maturin build, encode speed is hundreds
of times realtime so transcode-at-first-load is viable, and the XDG content-hash proxy
cache already decided in [#72](https://github.com/thedahm/uncompose/issues/72) absorbs the
repeated-CPU concern. libFLAC bindings are the fallback only if seektables, 32-bit-int
FLAC, or bit-exact reference behavior ever become requirements.

## 1. Pure-Rust FLAC encoders

### flacenc (yotarok/flacenc-rs) — the viable one

- Version 0.5.1 (2025-12-18); prior releases 0.5.0 (2025-07-18), 0.4.0 (2024-03-05);
  ~589k downloads, CI green, actively maintained.
  [crates.io](https://crates.io/crates/flacenc)
- License **Apache-2.0**; MSRV 1.83.0 with a "latest stable from one year prior" policy.
  [README](https://github.com/yotarok/flacenc-rs)
- Input is interleaved `&[i32]` plus `bits_per_sample` — **integer PCM only, no float
  input**. [docs.rs](https://docs.rs/flacenc/latest/flacenc/)
- **Bit depth capped at 8–24** (spec allows 4–32); max 8 channels.
  [src/constant.rs](https://github.com/yotarok/flacenc-rs/blob/main/src/constant.rs)
- Custom input via a `Source` trait, output via `ByteSink`; feature flags for
  multithreading (`par`), nightly SIMD, serde, mimalloc. Major-zero semver, breaking
  changes on minor bumps.
- Nothing documented beyond STREAMINFO; assume **no seektable/metadata blocks**
  (unverified, likely absent). Irrelevant for full-file `decodeAudioData` proxies.
- Benchmarks (repo's auto-generated report, 4 Wikimedia music tracks, CPU unspecified):
  compression 0.5276 of original vs libflac -8 at 0.5256 (within ~0.4%); encode speed
  ~331x realtime single-threaded, ~1309x with `par`, vs libflac -8 ~230x, -5 ~550x.
  [report.nightly.md](https://github.com/yotarok/flacenc-rs/blob/main/report/report.nightly.md)

### Others

- **claxon** 0.4.3 (2020): decode-only, confirmed. [crates.io](https://crates.io/crates/claxon)
- **flac-codec** 1.3.2 (2026-04-15, tuffy, MIT/Apache-2.0): pure-Rust encode+decode per
  RFC 9639, verified against the reference implementation. Bit-depth range and streaming
  API not verified here; worth a docs.rs read if flacenc's 24-bit cap ever matters.
  [docs.rs](https://docs.rs/flac-codec/latest/flac_codec/)
- **libflac-rs** 0.143.1 (2026-06-27): claims a bit-exact pure-Rust port of libFLAC
  1.4.3; not further verified. Smaller/WIP crates (flacshark, flac-io, flexaudio-encode,
  oxideav-flac) are not candidates.

## 2. libFLAC binding crates

- libFLAC itself is Xiph's BSD-like license (the `flac` CLI tools are GPL, not the
  library). [xiph/flac](https://github.com/xiph/flac)
- **libflac-sys** 0.3.4 (2025-10-04, BSD-3-Clause): default features build vendored
  libFLAC from a git submodule, requiring **cmake plus a C toolchain**; disabling
  `build-flac` links the system library instead. Static-vs-dynamic linking behavior is
  not documented (unverified). [repo](https://github.com/mgeier/libflac-sys)
- **flac-bound** 0.5.0 (2024-11-05, MIT): safe `FlacEncoder` wrapper; default feature
  links system libFLAC ("may require manual assistance"), `libflac` feature builds from
  source via libflac-sys. [repo](https://github.com/nabijaczleweli/flac-bound)
- **Cost for this project:** manylinux docker images ship cmake, so a vendored build is
  CI complexity rather than a blocker, and vendoring keeps the wheel auditwheel-clean.
  But it is a permanent C build dependency in every wheel build for a capability flacenc
  already provides in pure Rust at comparable ratio and better speed. Only worth it for
  seektables, 32-bit-int FLAC, or bit-exact reference output.

## 3. WAV-only alternative: the byte cost

- Stereo 16-bit PCM: 44.1 kHz = 176,400 B/s ≈ **10.6 MB/min**; 48 kHz ≈ 11.5 MB/min;
  24-bit/44.1 ≈ 15.9 MB/min (arithmetic).
- Measured FLAC ratio on music: **0.525–0.533** of the WAV size, for both libflac and
  flacenc on the same dataset
  ([flacenc benchmark report](https://github.com/yotarok/flacenc-rs/blob/main/report/report.nightly.md)).
  Xiph retired its own comparison page as "grading one's own exam"
  ([xiph.org/flac/comparison.html](https://xiph.org/flac/comparison.html)).
- Concrete case: a 4-minute mix plus 4 stems (5 stereo 16/44.1 files) is ~212 MB as WAV,
  ~112 MB as FLAC — FLAC saves ~100 MB (~47%) per comparison set, on the wire and in the
  2 GB LRU proxy cache from #72 (WAV nearly halves how many comparison sets the cache
  holds). WAV-only is functionally fine (localhost transfer, `decodeAudioData` decodes
  both) but wastes exactly this.

## 4. Sample-rate / bit-depth policy for proxies

- The browser end normalizes everything anyway: `decodeAudioData` resamples to the
  AudioContext rate
  ([MDN](https://developer.mozilla.org/en-US/docs/Web/API/BaseAudioContext/decodeAudioData))
  and `AudioBuffer` holds non-interleaved IEEE-754 float32 PCM
  ([MDN](https://developer.mozilla.org/en-US/docs/Web/API/AudioBuffer)).
- FLAC is **integer-only** ("there are no floating-point representations",
  [RFC 9639](https://www.rfc-editor.org/rfc/rfc9639.html)), 4–32 bits per sample;
  32-bit-int support arrived in FLAC 1.4.0 (2022)
  ([changelog](https://xiph.org/flac/changelog.html)); float will never be supported
  ([FAQ](https://xiph.org/flac/faq.html)).
- Separation stems are typically 16/24-bit int or 32-bit float WAV. Float and 32-bit-int
  sources must therefore be quantized for FLAC. **24-bit is the right target**: float32
  has a 24-bit significand, so 24-bit quantization of normalized float is essentially
  transparent, and the browser reduces to float32 regardless. This makes flacenc's 24-bit
  cap a non-issue.
- **Never resample server-side**: keep the source rate and let the browser's documented
  resampler do the one conversion. One fewer quality decision, and #59's sync analysis
  assumed sample counts derived from the source.
- Policy: 16-bit int sources → 16-bit FLAC (bit-exact); 24-bit int → 24-bit FLAC
  (bit-exact); 32-bit float / 32-bit int → 24-bit FLAC (transparent quantization);
  channel count and sample rate passed through.
- WAV fallback: **hound** 3.5.1 writes i8/i16/i32 (24-bit as i32 with
  `bits_per_sample: 24`) and IEEE float32 directly
  ([docs.rs](https://docs.rs/hound/latest/hound/trait.Sample.html)), so the fallback can
  even skip quantization for float sources. Same bit-depth policy applies for symmetry.

## 5. Transcode-at-load latency vs cached proxies

- flacenc encodes ~331x realtime single-threaded, ~1309x multithreaded (order of
  magnitude; benchmark CPU unspecified). A 4-minute stereo file encodes in roughly
  0.2–0.8 s; a mix plus 4 stems in ~1–4 s serially, less in parallel. Decode by symphonia
  adds little (decoding is cheaper than encoding).
- So **transcode-at-first-load is viable** — a couple of seconds before first playback,
  not tens. Caching is still right (repeat opens become instant, no repeated CPU burn),
  and #72 already decided the shape: XDG cache keyed by content hash, 2 GB LRU,
  `cache clear`. Nothing here forces eager pre-generation at import time; lazy
  encode-into-cache on first Compare load fits the numbers.

## Not verified

- flacenc seektable/metadata support beyond STREAMINFO (likely absent, undocumented).
- libflac-sys static-vs-dynamic linking specifics.
- flac-codec's exact bit-depth range and streaming API.
- CPU/environment behind the flacenc benchmark report.
