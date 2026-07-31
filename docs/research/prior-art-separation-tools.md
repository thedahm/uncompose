# Prior Art: Moises and Existing Open Separation Tools

Research for issue #4. Question: what do Moises and the existing open tools already do, and where is the gap Uncompose fills? Compiled 2026-07-31 from primary sources (product pages, project repos, READMEs, release notes).

## Part 1 — Moises: the bar a personal replacement must meet

Sourcing note: moises.ai hides its full pricing table behind login (studio.moises.ai/billing/pricing), so pricing comes from the US Apple App Store listing. Help-center articles (help.moises.ai) are the best primary source for limits and plan gating; several are marked "updated 1 year ago", so treat exact numbers as roughly 2025-vintage but still-published policy.

### Stem separation specifics

Stem configurations by plan (https://help.moises.ai/hc/en-us/articles/360010972019-Which-instruments-can-be-separated-on-Moises):

- **Free:** 2-stem (Vocals, Instrumental) and 4-stem (Vocals, Drums, Bass, Other) only. **5 uploads/month**, files up to **5 minutes**.
- **Premium:** custom separation, pick up to **5 instruments per upload** plus an automatic "Other" track. Available instruments: Vocals (or Lead/Background Vocals), Guitar (combined, or Acoustic+Electric, or Lead+Rhythm), Bass, Drums, Acoustic Piano, Keys, Wind, Strings. Files up to **20 min**, unlimited uploads.
- **Pro:** everything above, all uploads automatically **Hi-Fi quality**, plus Pro-exclusive stems: **Drum parts** (Kick, Snare, Toms, Hi-Hat, Cymbals, Other Drums) and **Multimedia stems** (Dialogue, Soundtrack, Effects) for podcast/film audio (https://help.moises.ai/hc/en-us/articles/10523828567196-Reasons-to-Subscribe-to-the-Moises-Pro-Plan).

**Quality tiers:** "Hi-Fi" separation is Pro-only, marketed as minimal bleed between instruments, 48kHz/24-bit processing and lossless WAV export (https://moises.ai/features/hi-fi-stem-separation/).

**Limits:** Free 5 songs/month at 5 min/file; Premium/Pro unlimited uploads at 20 min/file (https://help.moises.ai/hc/en-us/articles/360010855680, https://help.moises.ai/hc/en-us/articles/360010972039). On Free, re-running a different separation on the same song deducts from the monthly quota.

**Input formats:** audio MP3, AAC, AC3, AIFC, OGG, WMA, AIFF, FLAC, WAV; video with audio extraction MP4, M4V, MOV, MKV, M4R, M4A, FLV, MPEG, WEBM; import from public URL (https://help.moises.ai/hc/en-us/articles/360013289060-Accepted-file-formats).

**Export:** MP3 or M4A on mobile; MP3, M4A, or **WAV on web/desktop only, paid plans only**; Pro gets WAV at 48kHz/24-bit automatically. Export modes: "Separate Tracks" (individual stems, without playback edits baked in) or "Audio Mix" (single file including speed/pitch changes) (https://help.moises.ai/hc/en-us/articles/360013691720-How-do-I-export-my-file).

**Batch:** "Bulk Upload" on web/desktop only, up to **20 files** with the same separation settings for Premium/Pro; not on mobile (https://help.moises.ai/hc/en-us/articles/17950941743004).

### Workflow around separation

(mostly https://help.moises.ai/hc/en-us/articles/8583454469276-How-to-Upload-and-Edit-your-track-using-Moises)

- Upload via +/New (device files, URL, camera roll), pick separation type, submit, cloud processing, song lands in a persistent **library** list. Premium/Pro get priority processing queues.
- **Player UI:** one row per stem with volume slider, **mute and solo** per stem, per-track pan, transport, master volume.
- A processed song can be switched to a different separation config after the fact.
- **Platforms:** iOS/iPadOS (iPad App of the Year 2024), Android, web app, desktop app (Mac/Windows; Microsoft Store Best Music App 2025), Apple Vision, Apple TV (https://moises.ai/, https://apps.apple.com/us/app/moises-the-musicians-app/id1515796612).
- **Stems VST plugin** (Pro-only): AU/VST3/AAX for major DAWs; drag-drop or URL input, batch, 7–8 stems per track, stems drag into the DAW timeline or download as ZIP. Cloud-processed, not real-time (https://moises.ai/features/stems-vst-plugin/).

### Adjacent bundled features (the moat around separation)

- Smart metronome / exportable click track (Free: first minute only).
- Chord detection with easy/medium/advanced views and guitar tabs (Free: first minute).
- Lyrics transcription, editable on web/desktop (Free: first minute).
- Pitch/key change (Free: 2 semitones) and speed change (Free: plus/minus 10).
- AI-detected song Sections with looping.
- AI Mastering (Auto/Advanced modes Pro-only), Voice Studio (Pro-only), AI Studio generative stems (credit-metered), key/BPM detection, music cutter, capo mode, collaborative setlists.

### Pricing (US, mid-2026)

From the US Apple App Store listing (https://apps.apple.com/us/app/moises-the-musicians-app/id1515796612), the most reliable public source since web pricing needs login:

- **Free:** $0. 5 uploads/mo, 2/4-stem only, 5-min files, 1-minute caps on metronome/chords/lyrics.
- **Premium:** **$5.99/mo or $39.99/yr**.
- **Pro:** **$29.99/mo**.

Third-party 2026 reviews quote conflicting numbers and look unreliable; regional and web-checkout pricing may differ.

### Separation tech claims

- Proprietary models from parent company **Music AI**; published research includes **Moises-Light** (band-split U-Net with rotary-embedding transformer blocks, claimed SDR comparable to BS-RoFormer at up to 13x fewer parameters) and diffusion-based vocal separation (https://music.ai/blog/research/Moises-Research-Innovations-2025/).
- Claims models are trained only on licensed material.
- **All separation is cloud-processed, including the VST plugin.** Internet required; no real-time separation.

## Part 2 — Open-source tool survey (as of 2026-07-31)

### Ultimate Vocal Remover (UVR5)

- **Repo:** https://github.com/Anjok07/ultimatevocalremovergui (25.6k stars, MIT)
- **What it is:** the de facto standard free desktop GUI for source separation. Tkinter GUI, Windows/macOS installers plus source install for Linux.
- **Interface:** desktop GUI only. No official CLI or library (that role got filled by python-audio-separator).
- **Models:** the community reference model ecosystem: VR Architecture (their own trained models), MDX-Net, MDX23C, Demucs v1–v4, plus an **Ensemble mode** combining multiple models. A separate **Roformer beta patch** (Nov/Dec 2024 builds on SourceForge) added BS-RoFormer, SCNet, and Bandit model support with DirectML for AMD/Intel GPUs ([SourceForge mirror](https://sourceforge.net/projects/ult-vocal-remover-uvr.mirror/files/v5.6/)). In-app Download Center fetches models.
- **Workflow:** pick files/folder, pick model, convert. Batch supported. FFmpeg needed for non-WAV input. No built-in playback/audition, no job history. Output WAV/FLAC/MP3.
- **Install/GPU:** Windows 10+ installer (plus DirectML variant), macOS Big Sur+ arm64/x86_64 dmg, Linux from source (Python 3.9–3.10). CUDA GPU recommended (GTX 1060 6GB min), CPU works but slow ([README](https://github.com/Anjok07/ultimatevocalremovergui)).
- **Maintenance:** effectively stalled. Last stable release v5.6, **Sep 26, 2023** ([releases](https://github.com/Anjok07/ultimatevocalremovergui/releases)); Roformer support only ever shipped as a beta patch; last repo push Mar 2025; ~1,500 open issues.
- **Strengths:** best model breadth + ensembles, huge community mindshare, free.
- **Gaps:** dated Tkinter UI, no playback or stem audition, no job/library management, confusing model naming, stable release three years old, Roformer stuck in beta, no Linux packaging.

### Demucs (+ Demucs-GUI)

- **Repos:** https://github.com/facebookresearch/demucs (10.4k stars, MIT, **archived**, last push Apr 2024) → maintained continuation at https://github.com/adefossez/demucs (3k stars, push Jul 2026). Author (now at Kyutai) says he is "not actively working on Demucs anymore, so expect slow replies and no new feature" ([fork README](https://github.com/adefossez/demucs)).
- **What it is:** the research model + reference CLI that underpins most other tools. Hybrid Transformer Demucs (htdemucs) v4 models: `htdemucs` (default), `htdemucs_ft` (fine-tuned, 4x slower), `htdemucs_6s` (6 stems incl. piano/guitar), `hdemucs_mmi`, `mdx`/`mdx_extra` (+quantized).
- **Interface:** CLI (`pip install demucs; demucs MY_TRACK.mp3`) and Python API. Flags: `--two-stems=vocals`, `--mp3`, `--flac`, `--float32`, `--segment`, `-d cpu`.
- **Workflow:** file in, stem WAVs out (44.1kHz int16 default). Batch = pass multiple files. No playback.
- **GPU/CPU:** CUDA (3–7GB VRAM), CPU fallback (~1.5x track length).
- **Demucs-GUI:** https://github.com/CarlGao4/Demucs-Gui (GPL-3.0, 1.2k stars). Prebuilt binaries for Win/macOS/Linux with CUDA, MPS, Intel Arc/Xe, ROCm variants; mixer options; 2.0a1 (Jun 20, 2025) added Apollo restoration model support ([releases](https://github.com/CarlGao4/Demucs-Gui/releases)). Last release Jun 2025, low-intensity maintenance. Demucs-only (no MDX/RoFormer), no playback.
- **Strengths:** trivially scriptable, permissive license, still the best-known 4/6-stem model family.
- **Gaps:** model development frozen since ~2022 (RoFormers now beat it on SDR), no first-party GUI, maintenance is caretaker-mode.

### python-audio-separator

- **Repo:** https://github.com/nomadkaraoke/python-audio-separator (MIT, 1.3k stars). Latest release **v0.44.5, Jul 20, 2026**; very active ([releases](https://github.com/nomadkaraoke/python-audio-separator/releases)).
- **What it is:** CLI + Python library exposing the whole UVR model zoo without the GUI. Maintained by beveradb (Nomad Karaoke), built to power their automated karaoke pipeline.
- **Models:** broadest programmatic coverage anywhere: MDX-Net (ONNX), VR arch (.pth), Demucs (htdemucs/6s), MDX23C, **BS-Roformer and Mel-Band Roformer checkpoints** (.ckpt). `--list_models`, auto-download on first use, `--ensemble_preset` curated multi-model ensembles.
- **Workflow:** `audio-separator file.wav --model_filename X.ckpt`. Batch, chunked processing for long files, custom output naming, single-stem mode, output WAV/FLAC/MP3/M4A with bitrate control. Also documents a modal.com remote-API deployment.
- **Install:** pip variants for CPU, CUDA 11.8/12.2, CoreML (Apple Silicon), DirectML; conda; Docker images (CPU + GPU). Python ≥3.10, FFmpeg dependency.
- **Strengths:** the "engine" many wrappers and HuggingFace spaces embed; up to date with RoFormer SOTA; genuinely maintained in 2026.
- **Gaps:** developer tool. No GUI, no playback, model choice still requires reading community lore about which checkpoint is best.

### StemRoller

- **Repo:** https://github.com/stemrollerapp/stemroller (3.1k stars). License: **Unlicense or MIT-No-Attribution**, user's choice.
- **What it is:** one-click Electron desktop app: "separate vocal and instrumental stems from any song with a single click." Signature feature: **built-in YouTube search**; type a song name, hit Split, it downloads via yt-dlp and separates. Local files also supported.
- **Model:** bundles Demucs (htdemucs family) + ffmpeg + yt-dlp in the installer ([README](https://github.com/stemrollerapp/stemroller)).
- **Workflow:** search or drop file → wait minutes → "Open" folder of stems (vocals/drums/bass/other). No in-app playback/mixer, no real queue management.
- **Install:** prebuilt Windows (CUDA build) and macOS downloads via site/GitHub (binaries hosted on Hugging Face). Linux unofficial, manual deps.
- **Maintenance:** alive. v3.1.1 **Jun 30, 2026**, v3.1.0 Feb 2026 ([releases](https://github.com/stemrollerapp/stemroller/releases)); Discord for support.
- **Strengths:** easiest onboarding of any open tool; YouTube-in workflow matches how musicians actually start ("I want stems of this song").
- **Gaps:** Demucs-only (behind SOTA), no model choice, no playback/audition, no history/library, YouTube ripping sits in a legal gray zone.

### OpenStems

- **Org:** https://github.com/OpenStems (site: https://openstems.github.io)
- **What it actually is:** new (2026) project positioning as a **real-time, stem-aware music player**, not a batch splitter. It plugs stem separation and per-stem control into two open-source players: **Pear** (YouTube Music desktop client, macOS/Win) and **LX Music**. Three modes: Realtime (5 stems, live), Karaoke HQ (2 stems), 8 Stems HQ. Routes audio to **OBS** for streaming and to **DAWs** as AU (macOS) / VST3 (Windows) plugins. Claims no GPU and no virtual audio driver needed. v1.0.1 .pkg/.exe installers via GitHub releases ([site](https://openstems.github.io)).
- **Open-source reality check:** the main `OpenStems/OpenStems` repo is **empty (0 KB, 1 README commit, 0 stars, no license)**; the org only otherwise hosts the website repo. The "open source" framing largely leans on the player foundations (Pear is MIT, 32.9k stars: https://github.com/pear-devs/pear-desktop). As of mid-2026 the separation engine's source does not appear published. Treat as binary-distributed, near-zero traction, unproven maintenance.
- **Why it matters for positioning:** it is the one project in this list oriented around **live playback/manipulation of stems** rather than export, which is exactly the usability gap everything else has. Confirms the kickoff brief's read: OpenStems owns live-player integration; Uncompose's offline processing + stem management center of gravity does not collide with it.

### Other notable tools and the model landscape

- **Spleeter** (https://github.com/deezer/spleeter, MIT, 28.3k stars): the 2019 pioneer (2/4/5-stem TensorFlow models). Effectively dormant: latest GitHub release v2.3.0, PyPI 2.4.0 (~2023), no real development. Quality long surpassed; relevant only as prior art and name recognition.
- **Music-Source-Separation-Training (ZFTurbo)** (https://github.com/ZFTurbo/Music-Source-Separation-Training, MIT, 1.5k stars, pushed Jul 2026): the community's training/inference framework where most modern checkpoints (BS-RoFormer, Mel-Band RoFormer, SCNet, MDX23C variants) come from. CLI inference, researcher-grade UX. ZFTurbo also runs MVSEP.
- **BS-RoFormer / Mel-Band RoFormer:** current SOTA architecture (ByteDance paper); open implementation at https://github.com/lucidrains/BS-RoFormer (MIT, pushed Jun 2026). Community checkpoints (ZFTurbo, viperx, unwa, etc.) circulate via MSST/MVSEP/UVR beta and python-audio-separator. This is what quality-focused users actually run in 2026.
- **freemusicdemixer** (https://github.com/sevagh/free-music-demixer, MIT, 357 stars): client-side Demucs via WebAssembly in the browser. Repo **archived** (last push Apr 2025); hosted product went freemium. Proof that local in-browser separation works, but slow and discontinued as open source.
- **KaraFan** (https://github.com/Captain-FLAM/KaraFan, MIT, 150 stars): Colab/Python ensemble pipeline tuned for karaoke instrumentals. Last push Jun 2024, effectively inactive.
- **drumsep** (https://github.com/inagoy/drumsep, MIT, 145 stars, push Nov 2025): fine-tuned Demucs that splits a drum stem into kick/snare/toms/cymbals. Niche second-stage tool, CLI/Colab only.
- **Small GUI wrappers:** MISST (https://github.com/Frikallo/MISST, Tkinter+Demucs "stem player") and StemDeck (https://github.com/stemdeckapp/stemdeck, htdemucs_6s drag-and-drop app) exist but have little traction/maintenance.
- **Closed but relevant:** Moises/Music.AI has no meaningful open-source separation code; ByteDance's Ripple was a closed mobile app. Their research (BS-RoFormer) is what the open community reimplemented.
- **MVSEP.com** (context, not open source): ZFTurbo's web service; free tier 50 separations/day (320kbps MP3), paid tiers for lossless and priority. 100+ algorithms incl. BS/Mel RoFormer, MDX23C, SCNet, Demucs4, instrument-specific models, ensembles; hosts the community SDR quality leaderboard that effectively defines "best model" discourse ([mvsep.com](https://mvsep.com/en)). It is the benchmark a local-first product gets compared against on quality and model breadth.

## Part 3 — Positioning

### Cross-cutting gaps in the open ecosystem

1. **Nobody open-source combines separation with playback/audition.** UVR, Demucs, python-audio-separator, and StemRoller all end at "files in a folder." The Moises player (per-stem mute/solo/volume) is the daily-use surface, and no open tool has it. OpenStems targets live stem playback but its separation engine source is unpublished and the project has near-zero traction.
2. **No job history or library.** Moises keeps a persistent library of processed songs; every open tool makes re-running, comparing models, and finding old results manual file juggling. Nothing records which model/version/settings produced a result (reproducibility gap Uncompose explicitly targets).
3. **SOTA access is hostile.** The best open quality (BS/Mel-RoFormer community checkpoints) lives in a UVR beta patch, CLI tools (python-audio-separator, MSST), and community folklore. The only polished open GUI (UVR) has had no stable release since Sep 2023.
4. **Install friction everywhere.** Python versions, FFmpeg, CUDA-variant wheels. Only StemRoller truly bundles everything, and it is locked to Demucs with zero model choice.
5. **Maintenance risk.** UVR stalled, Demucs archived/caretaker-mode, Spleeter dormant, freemusicdemixer archived. The actively maintained pieces (python-audio-separator, MSST, StemRoller) are engines or single-purpose apps, not a product.

### The minimal local Moises-replacement bar

For the kickoff's stated success measure ("Dominic can use Uncompose instead of Moises for a meaningful portion of his own separation needs"):

**Must have (v1):**
- 2-stem and 4-stem separation (vocals/drums/bass/other) at RoFormer-class quality, i.e. at least matching Moises Free and approaching Hi-Fi. Practically: python-audio-separator-style model access, not Demucs-only.
- Local processing, CPU and one GPU path.
- Audition: per-stem playback with mute/solo/volume. This is the single biggest differentiator versus every open tool and the reason people stay on Moises.
- Organized output + export (WAV and MP3 at minimum), and a job record: input, model, version, parameters.
- Common input formats via FFmpeg (Moises accepts 9 audio + 9 video formats; matching audio formats is enough for v1).

**Should have (fast follows):** 5–6 stem presets (guitar, piano), model choice/comparison, batch, a persistent library of past jobs, 20-min+ files without quota anxiety (a local tool gets "unlimited" for free, which is itself the pitch versus $5.99–$29.99/mo).

**Explicitly not the bar (per kickoff non-goals):** drum-part splits, dialogue/FX stems, the practice bundle (chords, lyrics, metronome, pitch/speed), AI mastering/voice, real-time separation, DAW plugin. The practice bundle is Moises's real moat, but it sits outside separation and is deferred by design (and pitch/speed/metronome are the most plausible later additions).

### Where Uncompose fits

The open ecosystem has excellent engines and no product. The straight line for Uncompose: own the workflow layer (job orchestration, reproducibility, audition, library, export) on top of interchangeable engines, with python-audio-separator or MSST-style model access rather than a single bundled model. That position collides with no active project: UVR is a stalled GUI without playback, StemRoller is a one-click Demucs app, python-audio-separator is a library, OpenStems is a live player with closed separation code, MVSEP is a cloud service.

**Facts that bear on other open decisions:**
- *First model:* Demucs is the safe integration (MIT, pip, known) but is frozen and no longer SOTA; community RoFormer checkpoints are what quality-focused users run in 2026. Wrapping python-audio-separator (MIT, actively released Jul 2026, handles model download + all major architectures) may beat integrating Demucs directly, and would make Uncompose model-agnostic from day one.
- *First interface:* the unserved gap is audition + library, which points at a local web UI (or desktop shell) over a CLI core. A bare CLI would land in space python-audio-separator already covers well.
- *OpenStems coexistence:* confirmed non-overlapping; it is live-playback oriented, and its separation engine is not actually open.
