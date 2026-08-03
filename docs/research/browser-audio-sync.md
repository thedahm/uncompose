# Browser audio for synchronized A/B comparison: what the Web Audio API can promise

Research note for Uncompose Compare (synchronized A/B listening of separated stems vs originals in the browser). Date: 2026-08-03.

## Summary and recommendations

- **Use the Web Audio API (`AudioBufferSourceNode` + `GainNode`), not `<audio>` elements.** The spec guarantees sample-accurate scheduling and sub-sample-accurate loop points; `HTMLMediaElement` guarantees neither, and its `currentTime` is deliberately precision-reduced in some browsers.
- **A/B technique:** decode both versions to `AudioBuffer`s, start one `AudioBufferSourceNode` per version at the same `AudioContext` time, keep all playing, and switch by crossfading `GainNode.gain` (an a-rate param) over ~10 ms with an equal-power curve. This gives phase-locked, click-free, effectively instantaneous switching.
- **Budget memory:** decoded audio is Float32 PCM, ~10.6 MB per stereo minute at 44.1 kHz (~85 MB for a 4-minute track). Original + 4 stems ≈ 420 MB. Cap what is resident (decode only the loop region, or one stem pair at a time), especially on iOS Safari.
- **Serve WAV or FLAC to the browser, transcoded server-side.** MP3/AAC encoder delay/padding is trimmed inconsistently across browsers, which breaks gapless loops and can silently misalign A vs B by tens of samples. Uncompose already produces PCM stems, so this is cheap: FLAC for bandwidth, WAV as universal fallback.
- **Sync is testable:** render the A/B graph in an `OfflineAudioContext` and cross-correlate the output against the source buffers to prove zero-sample offset in CI; at runtime, `AudioContext.currentTime`, `getOutputTimestamp()`, and `outputLatency` let you align UI to actual audible output.
- **Server-side proxy/transcode is needed** only for ingesting user files in exotic codecs (ALAC, formats Safari rejects), for guaranteeing identical decoded sample counts across browsers, and for very long material where full decode would blow memory.

---

## 1. Scheduling: AudioBufferSourceNode vs HTMLMediaElement

The Web Audio API lists "Sample-accurate scheduled sound playback with low latency" as a core feature, and the spec's `start(when)` definition is explicit: "the exact value of `when` is always used without rounding to the nearest sample frame" — i.e. scheduling is not merely sample-accurate but sub-sample-exact in the spec's playback model ([Web Audio API spec](https://webaudio.github.io/web-audio-api/)). `AudioBufferSourceNode` is the node designed for "audio assets which require a high degree of scheduling flexibility and accuracy" (same spec, §1.9). MDN frames it the same way: it is for audio with "particularly stringent timing accuracy requirements... that must match a specific rhythm and can be kept in memory" ([MDN: AudioBufferSourceNode](https://developer.mozilla.org/en-US/docs/Web/API/AudioBufferSourceNode)).

`HTMLMediaElement` offers none of this:

- `currentTime` is a double in seconds, but browsers deliberately reduce its precision for anti-fingerprinting. Firefox rounds it to 2 ms by default and to 100 ms with `privacy.resistFingerprinting` ([MDN: HTMLMediaElement.currentTime](https://developer.mozilla.org/en-US/docs/Web/API/HTMLMediaElement/currentTime)). At 44.1 kHz, 2 ms is ~88 samples of ambiguity.
- Setting `currentTime` seeks, but there is no spec guarantee about seek granularity or how long the seek takes; seeking is asynchronous and codec/keyframe dependent. Two media elements cannot be started or seeked with a defined mutual offset.
- Gapless transitions through media elements require Media Source Extensions plus manual trimming of codec padding; Chrome's own engineering guidance on gapless audio is an MSE tutorial precisely because plain media elements cannot do it ([Chrome Developers: Media Source Extensions for Audio](https://developer.chrome.com/blog/media-source-extensions-for-audio)).

Conclusion: two `<audio>` elements toggled by `muted` or volume can drift by tens of milliseconds and cannot be verified or corrected to sample accuracy. Not usable for phase-coherent A/B.

## 2. Sample-accurate A/B switching technique

The reliable pattern:

1. `decodeAudioData` both versions (original and separated stem / stem mix) into `AudioBuffer`s.
2. Create one `AudioBufferSourceNode` per version, each feeding its own `GainNode` into the destination.
3. Call `start(t0)` on all sources with the **same** `t0` (a small offset into the future, e.g. `ctx.currentTime + 0.05`). Per the spec's start-time language above, both begin at the identical sample frame. All versions play continuously; "switching" is purely a gain operation, so A and B stay phase-locked forever.
4. Switch by automating gains: ramp the outgoing gain to 0 and the incoming to 1 over a short window.

Why this is sample-accurate: `GainNode.gain` is an **a-rate** `AudioParam`, meaning "the current audio parameter value [is taken] for each sample frame of the audio signal", vs k-rate params which hold one value per 128-frame render quantum ([MDN: AudioParam](https://developer.mozilla.org/en-US/docs/Web/API/AudioParam)). Automation events (`linearRampToValueAtTime`, `setTargetAtTime`, `setValueCurveAtTime`) are computed in the audio rendering thread against the context's sample clock, so the ramp shape is deterministic per sample.

Crossfade details:

- Use ~5–20 ms. Long enough to avoid a click from a waveform discontinuity, short enough to feel instant. A plain linear crossfade of correlated signals is usually fine, but an **equal-power** curve (gains `cos(θ)` / `sin(θ)`, θ from 0 to π/2, e.g. via `setValueCurveAtTime` with precomputed curves) avoids any mid-fade level dip and behaves well if stems and original decorrelate locally.
- `linearRampToValueAtTime` ramps from the *previous* automation event; anchor with `setValueAtTime(currentGain, now)` first or the ramp start time is not "now" ([MDN: AudioParam](https://developer.mozilla.org/en-US/docs/Web/API/AudioParam)).
- `setTargetAtTime` is an exponential approach that never exactly reaches the target; usable with `timeConstant ≈ fadeTime/5`, but fixed-length ramps/curves are easier to reason about ([MDN: AudioParam](https://developer.mozilla.org/en-US/docs/Web/API/AudioParam)).
- `AudioBufferSourceNode`s are one-shot: a node "can only be played once; after each call to `start()`, you have to create a new node" — but nodes are cheap and `AudioBuffer`s are reusable ([MDN: AudioBufferSourceNode](https://developer.mozilla.org/en-US/docs/Web/API/AudioBufferSourceNode)). Every transport action (play, seek, region restart) means building fresh sources started at a common time with a common buffer offset.

## 3. Memory cost of full-song AudioBuffers

`decodeAudioData` decodes the *entire* file ("only works with complete file data", [MDN: decodeAudioData](https://developer.mozilla.org/en-US/docs/Web/API/BaseAudioContext/decodeAudioData)) into linear PCM, resampled to the context sample rate (spec decode algorithm: "resample it to the sample-rate of the BaseAudioContext" — [spec](https://webaudio.github.io/web-audio-api/)). `AudioBuffer` channel data is `Float32Array` per channel ([spec, §1.4 AudioBuffer](https://webaudio.github.io/web-audio-api/)), i.e. 4 bytes/sample/channel regardless of source bit depth or codec.

Cost: `4 bytes × channels × sampleRate × seconds`.

| Material | Decoded size |
|---|---|
| 1 min stereo @ 44.1 kHz | ~21 MB |
| 4 min stereo @ 44.1 kHz | ~85 MB |
| 4 min original + 4 stereo stems | ~423 MB |
| Same, context running at 48 kHz output | ~460 MB |

Note the last row: the context runs at the hardware rate, so decoding on a 48 kHz (or 96 kHz) output device inflates buffers beyond the file's nominal size.

Browsers do not publish per-tab audio memory quotas, but iOS Safari is well known to terminate tabs under memory pressure at far lower thresholds than desktop. MDN's own guidance is that `AudioBufferSourceNode` is for audio that "can be kept in memory", while assets that must stream should be played via `AudioWorkletNode` ([MDN: AudioBufferSourceNode](https://developer.mozilla.org/en-US/docs/Web/API/AudioBufferSourceNode)); the spec says the same ([§1.9](https://webaudio.github.io/web-audio-api/)). Mitigations, in order of effort:

1. **Decode lazily and cap residency**: keep only the stem set currently being compared; drop references on switch so buffers can be collected.
2. **Decode only the loop region**: fetch byte ranges of WAV (trivially seekable, fixed frame size) or server-sliced segments; a 10-second stereo loop costs ~3.5 MB instead of the whole song.
3. **Stream via AudioWorklet**: pull PCM chunks into a ring buffer inside an `AudioWorkletProcessor` (WebCodecs `AudioDecoder` can feed it). Sample-accurate with constant memory, but you own buffering, seeking, and decode. Escape hatch for long material, not the v1 path.

## 4. Loop regions

`AudioBufferSourceNode` has `loop`, `loopStart`, `loopEnd` (both in **seconds**, defaults 0; `loopEnd` is exclusive) ([MDN: AudioBufferSourceNode](https://developer.mozilla.org/en-US/docs/Web/API/AudioBufferSourceNode)). The spec's normative playback algorithm (§1.9.6) states loop points "can be expressed with sub-sample precision and can vary dynamically during playback", with the playhead interpolated between sample frames (linear interpolation depicted; UAs may use other interpolation) ([spec](https://webaudio.github.io/web-audio-api/)). Practical consequences:

- Looping is sample-accurate (sub-sample, in fact) by spec; the loop wrap happens in the audio thread with no gap.
- `loopStart`/`loopEnd` are plain attributes, not AudioParams: you can change them live and the algorithm honors the change at the next wrap, but the change is not schedulable — it lands whenever the main thread sets it. For a synced A/B pair, set identical loop values on all sources before `start()`, and treat live edits of the region as a rebuild-and-restart.
- Because all sources share identical loop points and a common start time, they wrap on the same sample frame and stay phase-locked across loop iterations indefinitely.

**Gapless caveat — codec padding.** A loop (or an A/B pair) is only sample-exact if the decoded buffers contain exactly the original samples. Lossy codecs prepend encoder delay and append padding: LAME MP3 pads in 576-sample blocks (Chrome's demo file had "exactly 576 padding samples at the end"), with gapless info carried in the Xing/LAME header or the iTunes `iTunSMPB` tag ([Chrome Developers: MSE for Audio](https://developer.chrome.com/blog/media-source-extensions-for-audio)). Apple's AAC encoder uses 2112 priming frames vs ffmpeg's 1024, and a Web Audio spec maintainer confirms browsers "don't consistently trim these values during decoding", characterizing the situation as browser bugs outside the spec's control ([web-audio-api discussion #2505](https://github.com/WebAudio/web-audio-api/discussions/2505)). For Compare this means:

- An MP3-decoded stem and a WAV-decoded original can be offset by tens of samples relative to each other, differently per browser.
- Loop boundaries can land on silent padding, audible as a gap or thump at the wrap.
- **WAV and FLAC have no encoder delay/padding**; decoded sample counts are faithful. Use them.

## 5. Format support for decodeAudioData (Chrome / Firefox / Safari)

`decodeAudioData` delegates to the browser's media stack ("attempt to decode the encoded audioData into linear PCM", after MIME sniffing — [spec decode algorithm](https://webaudio.github.io/web-audio-api/)), so media-element codec support is the right first-order proxy, with the caveat that decodeAudioData has its own bug surface.

| Format | Chrome | Firefox | Safari | Notes |
|---|---|---|---|---|
| WAV (PCM) | yes | yes | yes | universal, patent-free ([MDN codec guide](https://developer.mozilla.org/en-US/docs/Web/Media/Guides/Formats/Audio_codecs)) |
| MP3 | yes | yes (22+, platform decoder) | yes (3.1+) | patents expired 2017 ([MDN codec guide](https://developer.mozilla.org/en-US/docs/Web/Media/Guides/Formats/Audio_codecs)) |
| AAC | yes (MP4 container; Main Profile caveats, absent in pure Chromium builds) | platform-dependent (OS decoder, patent reasons) | yes | ([MDN codec guide](https://developer.mozilla.org/en-US/docs/Web/Media/Guides/Formats/Audio_codecs)) |
| FLAC | 56+ | 51+ | 11 partial, 13+ full | ([caniuse: FLAC](https://caniuse.com/flac), [MDN codec guide](https://developer.mozilla.org/en-US/docs/Web/Media/Guides/Formats/Audio_codecs)) |
| ALAC | no | no | yes (native) | Safari-only ([MDN codec guide](https://developer.mozilla.org/en-US/docs/Web/Media/Guides/Formats/Audio_codecs)) |

Safari-specific decodeAudioData hazards, from WebKit's tracker:

- decodeAudioData regressions have shipped: MP3 decode failing entirely in Safari 15 on macOS Catalina ([WebKit bug 231449](https://bugs.webkit.org/show_bug.cgi?id=231449)); WebM/Opus Web Audio content broken by Safari 15 ([WebKit bug 226922](https://bugs.webkit.org/show_bug.cgi?id=226922)).
- FLAC-in-MP4 via MSE regressed in Safari 17 beta ([WebKit bug 260491](https://bugs.webkit.org/show_bug.cgi?id=260491)). Native `.flac` container is the safer Safari target than FLAC-in-MP4. Media-element support does not guarantee decodeAudioData support version-for-version, so **probe FLAC through decodeAudioData on real Safari** and keep a WAV fallback.

Cross-cutting decodeAudioData caveats: it strips all metadata (an `AudioBuffer` carries only PCM plus sampleRate/length/channels, [spec §1.4](https://webaudio.github.io/web-audio-api/)); it decodes only the first audio track ([spec decode algorithm](https://webaudio.github.io/web-audio-api/)); it resamples to the context rate; and, per §4, it handles MP3/AAC priming inconsistently, so **decoded sample counts for the same lossy file differ by browser** ([discussion #2505](https://github.com/WebAudio/web-audio-api/discussions/2505)).

## 6. Achievable and testable sync accuracy

Two clock domains matter:

- **`AudioContext.currentTime`** — the audio hardware clock: "the time of the sample frame immediately following the last sample-frame in the block of audio most recently processed" ([spec](https://webaudio.github.io/web-audio-api/)). Double precision, sample-addressable, advances in 128-frame render quanta on the audio thread, immune to main-thread jank. All scheduling (`start(when)`, param automation) lives in this domain.
- **`performance.now()` / JS timers** — main-thread time. Timer callbacks skew by "tens of milliseconds or more" under layout/GC load; the canonical pattern is lookahead scheduling: a ~25 ms timer tick that schedules audio events ~100 ms ahead in `AudioContext` time ([web.dev: Audio scheduling — A Tale of Two Clocks](https://web.dev/articles/audio-scheduling)).

Bridging and latency introspection:

- `AudioContext.getOutputTimestamp()` returns a paired `{contextTime, performanceTime}` snapshot for mapping audio time onto the performance timeline; Baseline widely available since April 2021 ([MDN: getOutputTimestamp](https://developer.mozilla.org/en-US/docs/Web/API/AudioContext/getOutputTimestamp)). Use it to draw a playhead that tracks what is actually audible.
- `baseLatency`: graph-to-audio-subsystem latency (the spec's example: ~5.8 ms for a 128-frame quantum at 44.1 kHz); widely available since April 2021 ([MDN: baseLatency](https://developer.mozilla.org/en-US/docs/Web/API/AudioContext/baseLatency), [spec](https://webaudio.github.io/web-audio-api/)).
- `outputLatency`: subsystem-to-output-device estimate; Baseline only "newly available" as of **March 2025**, so treat it as best-effort on older Safari ([MDN: outputLatency](https://developer.mozilla.org/en-US/docs/Web/API/AudioContext/outputLatency)).

What accuracy to claim: **A-vs-B relative sync is exact (0 samples)** — both sources render in the same graph off the same clock, so there is nothing to drift. Absolute output timing (when a sample reaches the speaker) is only known to within the `baseLatency + outputLatency` estimate; irrelevant for the comparison itself, relevant only for playhead/visual alignment.

How to verify:

1. **OfflineAudioContext rendering** — `OfflineAudioContext` renders the identical node graph deterministically, as fast as possible, into an `AudioBuffer` ([MDN: OfflineAudioContext](https://developer.mozilla.org/en-US/docs/Web/API/OfflineAudioContext)). In CI (Playwright: Chromium/Firefox/WebKit) build the A/B graph offline and assert: (a) cross-correlation of rendered output vs the source buffer peaks at lag 0; (b) "A solo" and "B solo" renders align known markers (an impulse in test fixtures) at identical sample indices; (c) a rendered crossfade equals `a·gainA + b·gainB` within float tolerance.
2. **Decode-count interop check** — decode a fixture file in each browser and assert `audioBuffer.length` equals the known sample count. This directly detects the §4/§5 priming-trim inconsistencies: passes for WAV/FLAC, fails (differently per browser) for MP3/AAC, which is exactly the signal wanted.
3. **Physical loopback** (manual, optional) — play a click, record via `getUserMedia` or an interface loopback, measure offset. Only needed if absolute latency ever matters (e.g. syncing to video); not needed for A/B correctness.

## 7. When server-side playback proxies / transcoding are needed

Uncompose's separation pipeline already emits PCM stems server-side, so transcoding is cheap and solves every interop problem at once. Transcode/proxy when:

1. **Guaranteeing identical decoded PCM across browsers.** Serving WAV or FLAC sidesteps lossy priming/padding trim differences entirely ([#2505](https://github.com/WebAudio/web-audio-api/discussions/2505), [Chrome MSE article](https://developer.chrome.com/blog/media-source-extensions-for-audio)). This is the default posture, not an exception: never hand the browser MP3/AAC for comparison playback even when the user uploaded MP3. Decode server-side once and compare stems against *that* decode, so A and B share provenance.
2. **Exotic ingest formats.** ALAC plays nowhere but Safari; AAC in pure Chromium builds and on some Firefox platforms depends on OS decoders ([MDN codec guide](https://developer.mozilla.org/en-US/docs/Web/Media/Guides/Formats/Audio_codecs)). The server transcodes these to FLAC/WAV at upload time.
3. **Very long material.** A 60-minute set at stereo 44.1 kHz is ~1.27 GB decoded per version; full decode is off the table, so the server slices region-sized WAV/FLAC segments (byte-ranging into WAV is trivial given its fixed frame size), or streams raw PCM to an AudioWorklet.
4. **Bandwidth.** FLAC is roughly half of WAV size, gapless-safe, and supported by all three engines (Chrome 56+ / Firefox 51+ / Safari 13+, [caniuse](https://caniuse.com/flac)); fall back to WAV wherever a startup decodeAudioData FLAC probe fails.

Not needed: DRM, MSE, or HLS-style streaming proxies. Compare's material is song-length, user-owned, and precision-critical; plain files plus Web Audio is the right layer.

## 8. Recommendation for Uncompose Compare

1. **Playback engine**: one `AudioContext`; per comparison, N `AudioBufferSourceNode → GainNode` chains started at a shared future `start(t0, offset)`; switching is a ~10 ms equal-power gain crossfade via `setValueCurveAtTime`; seek and region edits tear down sources and restart all of them at a common time.
2. **Delivery**: server transcodes everything to FLAC (WAV fallback) at 44.1/48 kHz matching the source; feature-probe FLAC through decodeAudioData at startup with a 1-second fixture.
3. **Loops**: identical `loop`/`loopStart`/`loopEnd` on all sources; region editing is a restart; add a "decode region only" mode backed by server range-slicing for long files.
4. **Memory policy**: decode on demand, keep at most the active comparison set resident, estimate footprint as `4 × channels × rate × duration`; default iOS to region-only decode.
5. **Testing**: CI renders the graph in `OfflineAudioContext` under Playwright (Chromium/Firefox/WebKit), asserting zero-lag cross-correlation and fixture decode counts; the decode-count test doubles as a canary for browser codec regressions like [WebKit 231449](https://bugs.webkit.org/show_bug.cgi?id=231449).
6. **UI playhead**: drive from `getOutputTimestamp()`; use `outputLatency` when present and degrade gracefully where absent (pre-2025 Safari) ([MDN: outputLatency](https://developer.mozilla.org/en-US/docs/Web/API/AudioContext/outputLatency)).
