# Research: Distribution channels for Python/ML audio apps

- **Ticket**: [#5 Distribution channels for Python/ML audio apps](https://github.com/thedahm/uncompose/issues/5)
- **Date**: 2026-07-31
- **Question**: What are the realistic distribution channels for a local Python/PyTorch-based audio application? Evaluate pip/pipx/uv, conda, container images, Flatpak, AppImage, and distro repos given heavyweight ML dependencies and large model files; the common patterns for model-file download and caching; and what comparable projects actually ship.

All claims below were verified against primary sources (official docs, package indexes, project source code) on 2026-07-31. Baseline facts used throughout: current stable PyTorch is **2.13.0** (2026-07-08, `Requires-Python >=3.10`, supports 3.10-3.14) ([PyPI](https://pypi.org/project/torch/)). A default `pip install torch` on Linux downloads **~2.7 GB** (526.6 MB torch wheel + ~2.2 GB of `nvidia-*` CUDA wheels); the macOS arm64 wheel is 111 MB (MPS, no CUDA); the Windows PyPI wheel is ~122 MB CPU-only ([PyPI JSON](https://pypi.org/pypi/torch/2.13.0/json)).

## TL;DR

1. **PyPI + uv/pipx is the mandatory first channel.** It is what every comparable separation tool ships (demucs, spleeter, audio-separator), it is the contributor path, and demucs's own README now leads with `uvx demucs`. Cost: on Linux, plain `pip install` pulls the full ~2.7 GB CUDA stack even on GPU-less machines, because torch backend selection is an index-URL choice that package metadata cannot express.
2. **conda-forge is the only channel where "depend on pytorch + ffmpeg and let CPU/GPU resolve itself" actually works on all three OSes** (via the `__cuda` virtual package), and it solves the ffmpeg system-dependency problem. Direct precedent: audio-separator is on conda-forge. But conda-forge packages of fast-moving projects rot (audio-separator lags 12 versions there), so treat it as a later, secondary channel.
3. **Nobody in this niche ships Linux desktop binaries, and no stem-separation app exists on Flathub.** Flatpak with CUDA is proven possible (Speech Note's NVIDIA addon pattern) and would be first-of-kind for this niche, but it is a meaningful maintenance investment. AppImage is cheaper to build (~3 GB artifact) but weak on updates at that size.
4. **The musician-grade channel in this niche is a fat installer** (UVR's 1.7 GB Windows setup.exe has 3.18M downloads) or a bootstrapper that resolves torch on first run (ComfyUI Desktop uses uv for this). That is a later milestone, not release one.
5. **Models never ship inside the package.** The universal pattern is first-run download with checksum verification into a user cache dir, overridable by env var and flag. Demucs itself migrated from a self-hosted CDN + torch.hub to Hugging Face Hub + safetensors in 2026. HF Hub gives resume, offline mode, dry-run sizing, and revision pinning for free.
6. **Debian main and Homebrew core are effectively closed** (Debian's ML policy classifies weights trained on unlicensed music as "ToxicCandy", Homebrew's pytorch is CPU-only). AUR and nixpkgs are cheap, community-driven, and can come later.

---

## 1. Channel-by-channel evaluation

### 1.1 pip / pipx / uv

**How torch ships on PyPI today.** The default PyPI Linux wheel is a CUDA build (currently targeting CUDA 13.0); the CUDA runtime arrives as ~12 separate `nvidia-*` pip wheels (cublas 423 MB, cudnn 366 MB, cufft 214 MB, nccl 206 MB, ...) ([PyPI JSON](https://pypi.org/pypi/torch/2.13.0/json)). CPU-only, older-CUDA, ROCm, and XPU builds live on a separate index: `pip install torch --index-url https://download.pytorch.org/whl/cpu` (variants `cpu`, `cu126`...`cu132`, `rocm6.x/7.x`, `xpu`) ([pytorch.org/get-started](https://pytorch.org/get-started/locally/)). Windows CUDA wheels are only on download.pytorch.org, not PyPI.

**The structural gap.** Backend selection is an index choice, not package metadata. Standard `Requires-Dist` cannot express "torch, CPU build". So an app on PyPI can only declare `torch>=X`; Linux users without a GPU download ~2.7 GB of CUDA they cannot use, unless they pass index flags themselves. Tools like [light-the-torch](https://github.com/Slicer/light-the-torch) exist solely to work around this.

**PyPI size limits** are 100 MB/file and 10 GB/project by default ([docs.pypi.org](https://docs.pypi.org/project-management/storage-limits/)); PyTorch operates on granted exceptions (160 GiB quota by 2024, [pypi/support#3836](https://github.com/pypi/support/issues/3836)). Practical rule: never bundle torch or models; a pure-Python Uncompose wheel will be tiny.

**pipx**: one isolated venv per app, so each torch-dependent tool duplicates the full stack; index override is possible via `--pip-args` but cannot be defaulted from metadata ([pipx CLI ref](https://pipx.pypa.io/latest/reference/cli.html)).

**uv**: has a dedicated [PyTorch integration guide](https://docs.astral.sh/uv/guides/integration/pytorch/) covering named indexes with `explicit = true`, per-platform routing via `[tool.uv.sources]` + markers, user-selectable backend extras with `[tool.uv] conflicts`, and `--torch-backend=auto` / `UV_TORCH_BACKEND`. Two caveats: `--torch-backend` exists only in the `uv pip` interface (not `uv tool install`), and `[tool.uv.*]` is project config, not published wheel metadata, so `uv tool install uncompose` from PyPI still resolves the default (CUDA) torch on Linux. uv hardlinks from a global cache, so multiple tools share the torch download on disk ([uv cache docs](https://docs.astral.sh/uv/concepts/cache/)).

**Python version window**: torch 2.13 supports 3.10-3.14, with historical lag on new Python releases (3.14 was "preview" in torch 2.9, full in 2.10, [pytorch#169929](https://github.com/pytorch/pytorch/issues/169929)). pipx/uv default to the newest Python, so ship `requires-python = ">=3.10,<3.15"` and document `--python 3.12` as the escape hatch.

**Precedent**: demucs 4.1.0 declares plain `torch>=2.1` plus Intel-macOS environment-marker caps ([pyproject](https://raw.githubusercontent.com/adefossez/demucs/main/pyproject.toml)); audio-separator's `[cpu]`/`[gpu]`/`[dml]` extras only switch the onnxruntime flavor because extras cannot select a torch build ([pyproject](https://raw.githubusercontent.com/nomadkaraoke/python-audio-separator/main/pyproject.toml)).

**Demands on the codebase**: standard `[project.scripts]` entry point, `requires-python` bounds, extras for non-torch backend deps, a README install matrix (pip/uv commands per OS/accelerator) that must be re-verified when torch bumps its default CUDA (roughly every 6-9 months: cu126 → cu128 → cu130 within ~18 months). ffmpeg must be documented as a separate OS-level install; PyPI has no real ffmpeg distribution (even demucs points pip users to conda-forge for it).

### 1.2 conda / conda-forge

**PyTorch's official Anaconda channel is dead**: deprecation announced Oct 2024, PyTorch 2.5 was the last release there; rationale was that conda builds consumed over half of packaging maintenance for under 5% of downloads ([pytorch#138506](https://github.com/pytorch/pytorch/issues/138506)). conda-forge took over and is current: pytorch **2.13.0** updated 2026-07-25, all five platforms, with the historical Windows-CUDA gap closed in Jan 2025 ([anaconda.org/conda-forge/pytorch](https://anaconda.org/conda-forge/pytorch), [feedstock PR #231](https://github.com/conda-forge/pytorch-cpu-feedstock/pull/231)).

**The unique win**: the `__cuda` virtual package detects the host driver and the solver automatically prefers CUDA builds on GPU machines and CPU builds elsewhere ([conda virtual packages](https://docs.conda.io/projects/conda/en/latest/user-guide/tasks/manage-virtual.html)). No other channel does per-machine accelerator resolution from declared metadata. And **ffmpeg is a normal conda-forge run dep** (currently 8.1.2, all platforms), which erases the biggest pip-channel UX wart.

**Anaconda licensing caveat**: the `defaults` channel requires a paid plan for orgs >200 employees ([Anaconda ToS](https://www.anaconda.com/legal/terms/terms-of-service)); point users at **Miniforge** (conda-forge-only) ([miniforge](https://github.com/conda-forge/miniforge)).

**Burden**: submit a recipe to staged-recipes once, then maintain the feedstock (autotick bot files version-bump PRs). A pure-Python app builds as cheap `noarch: python`. The observed failure mode is neglect: audio-separator's conda-forge package is 12 versions behind PyPI; spleeter's is frozen at 1.5.3 since 2020.

**Bonus path**: conda's [constructor](https://conda.github.io/constructor/) builds double-clickable .sh/.pkg/.exe installers bundling app + pytorch + ffmpeg with no preexisting conda, plus desktop shortcuts via menuinst. Caveat: fixed package set, so CPU and GPU need separate installers. This is a plausible route to a musician-grade installer without leaving the Python packaging world.

**Demands on the codebase**: none beyond clean PyPI packaging (grayskull generates the recipe from PyPI); all deps must exist on conda-forge (torch, torchaudio, ffmpeg all do).

### 1.3 Container images

Official `pytorch/pytorch:2.13.0-cuda13.0-cudnn9-runtime` is **2.8-3.6 GB compressed** depending on CUDA line; devel variants 11-12 GB; NVIDIA NGC's image is 10.5 GB ([Docker Hub](https://hub.docker.com/r/pytorch/pytorch/tags), [NGC](https://catalog.ngc.nvidia.com/orgs/nvidia/containers/pytorch)). GPU passthrough needs the NVIDIA Container Toolkit on Linux or WSL2 + a paravirtualization Windows driver ([toolkit guide](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/latest/install-guide.html), [Docker GPU docs](https://docs.docker.com/desktop/features/gpu/)). **macOS is a hard stop**: containers run in a Linux VM with no GPU/MPS passthrough, so torch in Docker on Apple Silicon is CPU-only.

Suitability: fine for batch file-in/file-out via bind mounts (with UID/SELinux annoyances); wrong as a primary desktop channel (Docker + driver + toolkit prerequisite chain, no macOS GPU). Host on **GHCR** (free for public images) rather than Docker Hub (anonymous pulls limited to 100/6h/IP) ([GitHub Packages billing](https://docs.github.com/en/billing/concepts/product-billing/github-packages), [Docker Hub limits](https://docs.docker.com/docker-hub/usage/)).

Precedent: audio-separator maintains current CPU and `:gpu` images; spleeter's official images have been unmaintained since 2021.

**Demands on the codebase**: Dockerfile(s) + CI tag matrix (cpu/cuda), a CLI that behaves headlessly with mounted paths, cache dir env vars honored so model caches can be volume-mounted.

### 1.4 Flatpak / Flathub

**Feasible, precedented outside this niche, nonzero ongoing cost.** Flathub builds are offline, so all pip deps must be pinned as manifest modules (via `flatpak-pip-generator` or `req2flatpak`), including exact torch wheel URLs ([Flathub requirements](https://docs.flathub.org/docs/for-app-authors/requirements), [flatpak-builder-tools](https://github.com/flatpak/flatpak-builder-tools)). Runtime network is a normal permission, so **first-run model download is allowed** (Speech Note does exactly this, [manifest](https://github.com/flathub/net.mkiol.SpeechNote/blob/master/net.mkiol.SpeechNote.yaml)).

**CUDA works** via the `org.freedesktop.Platform.GL.nvidia` extension (which installs `libcuda.so.1`) plus bundling the CUDA toolkit runtime in the app or an addon. Proven pattern: Speech Note ships a CPU base app (~1 GiB) plus an optional NVIDIA addon flatpak bundling CUDA 12.9 + cuDNN + torch CUDA wheels stripped to ~1.8 GB ([addon manifest](https://github.com/mkiol/dsnote-nvidia-flatpak/blob/beta/net.mkiol.SpeechNote.Addon.nvidia.yaml)). Buzz (Whisper GUI) is on Flathub with pinned torch `+cu129` wheels ([flathub repo](https://github.com/flathub/io.github.chidiwilliams.Buzz)).

**No stem-separation or vocal-remover app exists on Flathub** (searched 2026-07-31); Uncompose would be first-of-kind, which is real distribution upside on Linux desktop.

**Demands on the codebase**: XDG-compliant cache/config paths (models under `$XDG_CACHE_HOME`/`$XDG_DATA_HOME` work inside the sandbox), file access through portals (matters for a GUI; a CLI in flatpak is awkward), AppStream metadata + .desktop file, and the recurring cost of regenerating the pinned wheel manifest on every torch/app bump.

### 1.5 AppImage

De facto tool is [python-appimage](https://github.com/niess/python-appimage) (relocatable manylinux Pythons + recipe dir). Torch sets the glibc floor: PyTorch wheels are **manylinux_2_28** since the 2.6 cycle ([announcement](https://dev-discuss.pytorch.org/t/pytorch-linux-wheels-switching-to-new-wheel-build-platform-manylinux-2-28-on-november-12-2024/2581)), so Ubuntu 20.04+/Debian 10+/RHEL 8+ hosts. Expect a **~2.5-3.5 GB artifact** with CUDA bundled (CUDA binaries barely compress). No sandbox is actually an advantage here: torch wheels bundle the CUDA runtime, so only the host kernel driver is needed, zero plumbing.

Weaknesses: FUSE fallback (`--appimage-extract-and-run`) fully extracts ~3 GB to tmp per run; zsync delta updates perform poorly at multi-GB scale ([zsync2#69](https://github.com/AppImageCommunity/zsync2/issues/69)); desktop integration is manual (AppImageLauncher is shaky with the new static runtime). Linux x86_64/aarch64 only.

**Demands on the codebase**: minimal (recipe dir + CI job + multi-GB release hosting). No app code changes if paths are already XDG-clean.

### 1.6 Distro repos

- **AUR / Arch**: best system-torch story anywhere. Arch `extra` ships 8 torch variants at 2.13.0 including prebuilt `python-pytorch-cuda` (1.3 GB installed) ([archlinux.org](https://archlinux.org/packages/?q=python-pytorch)); an AUR PKGBUILD just declares `depends=(python-pytorch)`, no venv duplication. **No demucs/spleeter/audio-separator AUR packages exist today** (AUR RPC queried 2026-07-31), so Uncompose would be creating the niche. Cost: a PKGBUILD, near zero.
- **nixpkgs**: Python demucs never packaged (torchaudio's nix expression literally notes this); but the Rust reimplementation **demucs-rs merged in April 2026 with runtime HF model download accepted by reviewers** ([package.nix](https://github.com/NixOS/nixpkgs/blob/master/pkgs/by-name/de/demucs-rs/package.nix)), and a `fetchFromHuggingFace` fixed-output fetcher now exists for pinning weights. Caveat: CUDA torch is unfree + uncached on cache.nixos.org, so NixOS GPU users compile or use the community CUDA cache ([nixpkgs manual](https://nixos.org/manual/nixpkgs/unstable/#cuda)).
- **Homebrew**: core `pytorch` formula is alive at 2.13.0 but **CPU/Accelerate only, no CUDA ever**; torch-dependent apps are accepted (openai-whisper is in core, downloads models at runtime) ([formulae.brew.sh](https://formulae.brew.sh/formula/pytorch)). macOS users would get MPS only via pip/conda, not brew. A custom tap avoids core review.
- **Debian/Ubuntu**: `python3-torch` in main is CPU-only and frozen per release (trixie: 2.6.0); CUDA torch sits in contrib. The Debian Deep Learning team's **ML-Policy** classifies free-licensed models trained on unlicensed data as "ToxicCandy", excluded from main ([ML-Policy](https://salsa.debian.org/deeplearning-team/ml-policy/-/raw/master/ML-Policy.rst)); separation weights trained on unlicensed music are the textbook case. Best case is contrib with runtime weight download against a years-old torch. Not worth pursuing.

The structural lesson from the survey: **only projects that escaped the Python/torch stack achieved wide distro packaging** (whisper.cpp is in Debian/Ubuntu/Alpine/BSDs; Ollama is in Arch official/Fedora/nixpkgs). Every torch-based app tops out at AUR/MacPorts.

---

## 2. Model download and caching patterns

### 2.1 The reference implementations

- **torch.hub** (`load_state_dict_from_url`): flat-file cache at `~/.cache/torch/hub/checkpoints` (`TORCH_HOME` / `XDG_CACHE_HOME` respected), checksum-in-filename convention (`name-<sha256prefix>.ext`, verified during download, temp file + atomic rename), tqdm progress, implicit offline once cached, no resume, no manifest ([hub docs](https://docs.pytorch.org/docs/2.13/hub.html), [hub.py](https://github.com/pytorch/pytorch/blob/main/torch/hub.py)).
- **Hugging Face Hub** (`huggingface_hub`): content-addressed cache (`blobs/` + `snapshots/<commit>/` symlinks) under `HF_HOME` (default `~/.cache/huggingface`), revision pinning by commit hash, `HF_HUB_OFFLINE=1` / `local_files_only=True`, chunk-based Xet transfer with `.incomplete` resume handling, `dry_run=True` reports exact bytes before fetching, server-side license gating available, `CACHEDIR.TAG` so backup tools skip the cache ([cache guide](https://huggingface.co/docs/huggingface_hub/guides/manage-cache), [env vars](https://huggingface.co/docs/huggingface_hub/package_reference/environment_variables), [download guide](https://huggingface.co/docs/huggingface_hub/guides/download)).
- **Demucs** (the direct ancestor): ships an in-repo manifest (`files.txt` + per-model YAML "bags" mapping stable names like `htdemucs_ft` to constituent model signatures and mixing weights), historically delegated download to torch.hub against `dl.fbaipublicfiles.com`, and **in 2026 migrated to HF-first**: `get_model()` now tries `hf_hub_download` of `{name}.yaml` + `{sig}.safetensors` from `adefossez/*` repos, falling back to the legacy CDN ([pretrained.py](https://github.com/adefossez/demucs/blob/main/demucs/pretrained.py), [hf.py](https://github.com/adefossez/demucs/blob/main/demucs/hf.py), [repo.py](https://github.com/adefossez/demucs/blob/main/demucs/repo.py)). Also supports `--repo <local dir>` for offline/pre-seeded model dirs with the same checksum-named files.
- **openai/whisper**: SHA256 as a URL path segment, verify-on-every-load into `~/.cache/whisper` ([\_\_init\_\_.py](https://github.com/openai/whisper/blob/main/whisper/__init__.py)). Per-load rehash of multi-GB files is slow; do not copy this.
- **faster-whisper**: writes zero download code; everything (cache, resume, offline, progress) comes from `huggingface_hub.snapshot_download` ([utils.py](https://github.com/SYSTRAN/faster-whisper/blob/master/faster_whisper/utils.py)).
- **python-audio-separator**: cautionary tale. Defaults to `/tmp/audio-separator-models/` (vanishes on reboot), fetches its model manifest from a third-party GitHub raw URL at runtime, and "verifies" by MD5-hashing the last 10 MB of the file for identity lookup, not integrity ([separator.py](https://github.com/nomadkaraoke/python-audio-separator/blob/main/audio_separator/separator/separator.py)).

### 2.2 Cache location

Per XDG spec, cache is for "non-essential" re-fetchable data; a 2 GB checkpoint technically qualifies but deletion costs a multi-GB re-download. torch, HF, and whisper all chose `~/.cache` anyway. Use [platformdirs](https://platformdirs.readthedocs.io/en/latest/api.html) for per-OS defaults (Linux `~/.cache/uncompose`, macOS `~/Library/Caches/Uncompose`, Windows `%LOCALAPPDATA%`), tolerate cache deletion gracefully, and write a `CACHEDIR.TAG`.

### 2.3 The pattern to adopt

Resolution order **CLI flag > env var > platformdirs default**, plus:

1. First-run download with progress bar and pre-download size disclosure (HF `dry_run` gives exact bytes).
2. SHA256 verification once at download time, temp file + atomic rename (torch.hub style), not per-load rehash.
3. A **manifest that versions models separately from code**: ship it in the package (Demucs style), mapping stable preset names to pinned artifacts (HF repo + commit hash, or URL + sha256). This is also what makes jobs reproducible (kickoff requirement).
4. **Offline/pre-seed support**: a local model-dir mode (Demucs `--repo` style) plus `HF_HUB_OFFLINE` passthrough. Required for air-gapped users and helps Flatpak.
5. If models live on HF Hub (as Demucs's now do), `huggingface_hub` provides cache layout, resume, offline mode, and gating for free; self-hosting means reimplementing resume or living without it.

---

## 3. What comparable projects actually ship

| Project | Channels | Torch handling | Models | Status |
| --- | --- | --- | --- | --- |
| **Demucs** ([adefossez/demucs](https://github.com/adefossez/demucs)) | PyPI only (4.1.0, 2026-07, after 3-yr gap); README leads with `uvx demucs`; MacPorts is its only distro presence | `torch>=2.1`, CUDA is user's problem | Runtime download, HF-first + Meta CDN fallback | Original repo archived; fork maintained but "no new features"; ~382k downloads/mo |
| **Spleeter** ([deezer/spleeter](https://github.com/deezer/spleeter)) | PyPI, stale conda-forge, unmaintained Docker (2021) | Hard pin `tensorflow==2.12.1`, Python <3.12: rot | Runtime download from own GitHub release assets (2stems: 1.94M downloads) | Dormant since 2021 |
| **UVR** ([Anjok07/ultimatevocalremovergui](https://github.com/Anjok07/ultimatevocalremovergui)) | GitHub-release installers only: Windows setup.exe ~1.7-2.1 GB (**3.18M downloads**), macOS dmg ~0.5 GB; **Linux = clone + pip** | Frozen bundle | Downloaded in-app | The musician channel, proven at scale |
| **python-audio-separator** ([nomadkaraoke](https://github.com/nomadkaraoke/python-audio-separator)) | PyPI (~342k dl/mo), stale conda-forge, current Docker cpu/gpu | `torch>=2.3` unconditional; extras switch onnxruntime only | Runtime download from UVR-ecosystem GitHub releases | Active; the pip layer over UVR's models |
| **StemRoller** ([stemrollerapp](https://github.com/stemrollerapp/stemroller)) | Win .exe 2.5 GB / mac .zip 1.05 GB hosted on HF (not GitHub releases); no Linux build | Electron + cx_Freeze-frozen demucs binary | **Bundled in installer** (offline-capable, costs GB) | Active |
| **OpenAI Whisper** | PyPI; Homebrew core (depends on CPU-only brewed pytorch); nixpkgs | torch unpinned | Runtime download, Azure CDN, sha256-verified | The "plain pip + runtime download" archetype |
| **faster-whisper / whisper.cpp** | pip vs native C++ | faster-whisper dropped torch entirely (CTranslate2 + pip cuBLAS wheels); whisper.cpp is C++ with 5-9 MB binaries | HF Hub / download script | whisper.cpp has the widest distro reach of anything surveyed (Debian, Ubuntu, Alpine, BSDs, AUR) |
| **ComfyUI** | Three tiers: git clone (devs), Windows portable 7z per GPU 1.7-2.1 GB, comfy-cli on PyPI, Electron Desktop that **resolves deps on first run with uv** | Per-tier: manual index URLs / baked / uv-resolved | User-supplied files | The modern layered model |
| **A1111 webui** | No PyPI ever; git clone + launcher script that bootstraps venv and installs pinned torch; 52 MB Windows zip bootstrapper | Launcher-managed | User-supplied | AUR only; low-activity |
| **Ollama** (counterexample) | Native Go binary: dmg, setup.exe, install.sh, Docker (159M pulls), brew, Arch official, Fedora, nixpkgs | Ships per-accelerator bundles itself; install.sh detects hardware | Own registry with `ollama pull` | What escaping Python buys |

**Observed patterns:**

1. **Two-audience split is universal**: pip package for tinkerers/devs, giant self-contained installer for musicians. Nobody in music separation serves both from one artifact; the ecosystem layered instead (UVR ships the GUI, audio-separator repackages its models for pip/Docker).
2. **Nobody ships Linux desktop binaries** (UVR, StemRoller, ComfyUI Desktop, OpenStems: Windows + macOS only). Zero stem-separation apps on Flathub. Linux users are assumed pip-capable. This is an open niche.
3. **Four torch strategies observed**: (a) unpinned floor, CUDA is your problem (whisper, demucs); (b) hard pin and rot (spleeter); (c) bake per-accelerator torch into fat artifacts (UVR, StemRoller, ComfyUI portable); (d) escape torch (faster-whisper, whisper.cpp, Ollama). The modern middle path is a small bootstrapper that defers the heavy resolve to first run (ComfyUI Desktop + uv, A1111's 52 MB zip).
4. **Models are never in the pip package**; runtime download from HF Hub, GitHub releases, or a CDN is standard. Only StemRoller bundles weights (offline-capable, multi-GB artifacts) and only Ollama built a real registry.
5. **conda is where packages go stale** in this niche; Docker is a reproducibility escape hatch, not a consumer channel.
6. **OpenStems** (named in the kickoff brief as adjacent) turns out to be open-source in name only: binary-only installers, README-only repo, no license ([repo](https://github.com/OpenStems/OpenStems)).

---

## 4. Implications for Uncompose

### Recommended sequencing

1. **Release 1: PyPI, uv-first.** `pip install uncompose` / `uvx uncompose`. Document the install matrix per OS/accelerator (copy uv's PyTorch-guide patterns). Declare `torch>=X` with a `requires-python` upper bound; consider a `uncompose[cpu]`-style documented path even though metadata can't enforce the index. This serves the initial primary user (the creator) and contributors, and matches what a "technically competent user installing from docs alone" (kickoff success measure 2) expects in 2026.
2. **Early follow-on, cheap**: GHCR container images (cpu + cuda tags) for headless/batch users; an AUR PKGBUILD depending on Arch's prebuilt `python-pytorch-cuda`.
3. **When a GUI exists**: Flatpak (CPU base + NVIDIA addon, Speech Note pattern) as the first-of-kind Linux desktop presence, and/or a constructor-built or uv-bootstrapping installer for Windows/macOS musicians. This is the channel that actually replaces Moises for non-developers, and it is a milestone of its own.
4. **conda-forge**: only once release cadence stabilizes, and only if willing to keep the feedstock current; a stale conda package is worse than none.
5. **Skip**: Debian main (ML policy), Homebrew core (CPU-only torch); revisit nixpkgs opportunistically (demucs-rs shows reviewers accept runtime model downloads).

### What this demands of the codebase from day one

- Clean `pyproject.toml` with `[project.scripts]` entry point, bounded `requires-python`, no bundled binaries or weights.
- **platformdirs-based cache/data paths overridable by env var and CLI flag.** This single decision is what keeps Docker, Flatpak, AppImage, and air-gapped modes viable later.
- A **model manifest** (name → artifact + hash/revision) shipped in the package, versioned separately from code; downloads via `huggingface_hub` if models live on HF (they can: Demucs's weights already do), with sha256 verification, progress, size disclosure, and an offline/local-model-dir mode.
- ffmpeg treated as an explicit, checked, well-error-messaged system dependency.
- Headless-friendly CLI (no GUI assumptions in core) so the same core drops into containers and future GUI shells.

### Facts that touch other open decisions

- **Model choice**: Demucs upstream is archived/maintenance-mode, but its weights are now on HF Hub in safetensors under `adefossez/*`, and demucs 4.1.0 was re-released to PyPI in July 2026 requiring only `torch>=2.1`. Integration remains viable but Uncompose should plan to own its model-fetch layer rather than lean on demucs's long-term maintenance. Debian's "ToxicCandy" framing is also a useful signal for the licensing story around separation weights generally.
- **Contributor setup**: `uv sync` with the uv PyTorch-guide index configuration is the simplest reliable contributor path (per-platform torch routing, lockfile, global cache dedupe).
