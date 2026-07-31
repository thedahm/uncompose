# Model Landscape: Open-Source Music Source Separation (for Uncompose)

Research date: 2026-07-31. All claims cited inline; full URL list in [Sources](#sources).

## Summary and shortlist

Uncompose needs: local-first, 4-stem (vocals/drums/bass/other), CPU as a floor with GPU as an upgrade, license-clean for an open-source product, and a maintainable Python dependency.

**Shortlist (in order):**

1. **Demucs v4 / HTDemucs (`htdemucs`, `htdemucs_ft`, `htdemucs_6s`)** — the first integration.
   - The only high-quality model family that is genuinely CPU-feasible: separation on CPU runs at "roughly equal to 1.5 times the duration of the track" ([README](https://github.com/adefossez/demucs)). GPU needs 3 GB VRAM minimum, 7 GB recommended, tunable via `--segment`.
   - MIT code, weights distributed with the repo with no separate restriction stated, trained on MUSDB HQ + 800 in-house songs ([facebookresearch/demucs](https://github.com/facebookresearch/demucs), [adefossez/demucs](https://github.com/adefossez/demucs)). Cleanest license story of any 4-stem model at this quality level.
   - True native 4-stem, plus a 6-stem variant (adds guitar, piano). ~9.0 dB average SDR on MUSDB18-HQ (9.2 dB fine-tuned/sparse) — still the reference 4-stem baseline.
   - Risk: dormant maintenance (original repo archived Jan 2025; the author's fork is stable but "no new feature for now"). Mitigation: `pip install demucs` works, code is small and frozen, and audio-separator (below) also bundles it.

2. **`audio-separator` (nomadkaraoke/python-audio-separator) as the integration layer** — either the actual first dependency, or the fast-follow.
   - MIT, on PyPI, v0.44.5 released 2026-07-20, Python 3.10–3.14, CPU/CUDA/CoreML/DirectML, auto-downloads models ([PyPI](https://pypi.org/project/audio-separator/), [repo](https://github.com/nomadkaraoke/python-audio-separator)).
   - One API over Demucs, MDX-Net, VR-arch, MDX23C, and the RoFormer models from the UVR ecosystem. This is the most actively maintained inference package in the space as of mid-2026.
   - Risk: pulls in both torch and onnxruntime (heavy install); some of the UVR-hosted community checkpoints it downloads have unclear weight licenses (see risks section).

3. **Mel-Band RoFormer, Kim/KimberleyJSN checkpoint** — the quality upgrade for vocals, GPU-first.
   - Best-in-class vocal separation (SDR 10.98 vocals on ZFTurbo's multisong benchmark vs 8.24 for HTDemucs vocals) ([MSST model zoo](https://github.com/ZFTurbo/Music-Source-Separation-Training/blob/main/docs/pretrained_models.md)).
   - The Kim checkpoint is explicitly MIT on Hugging Face ([KimberleyJSN/melbandroformer](https://huggingface.co/KimberleyJSN/melbandroformer)) — the license-clean choice among community RoFormer weights. The popular "viperx" BS-RoFormer checkpoint has **no explicit license** and should be avoided in a shipped product.
   - 2-stem (vocals/instrumental) only; combine with Demucs for drums/bass/other ("RoFormer vocals + Demucs for the rest" is the standard community pipeline).
   - Risk: transformer inference is far slower than realtime on CPU; treat as GPU-tier feature.

**Recommended path:** integrate Demucs first (direct `demucs` package or via audio-separator), design the model layer so audio-separator's catalog can slot in, then add Kim Mel-Band RoFormer as the "high quality vocals (GPU recommended)" option. Skip Spleeter, Open-Unmix, and raw kuielab/mdx-net entirely (reasons below).

---

## Comparison table

SDR figures are dB on MUSDB18-HQ test or ZFTurbo's "multisong" benchmark as noted; higher is better.

| Model / family | Quality (SDR) | Stems | CPU floor | GPU | Code license | Weights license | Packaging | Maintenance (mid-2026) |
|---|---|---|---|---|---|---|---|---|
| **Demucs v4 (htdemucs)** | 9.0 avg MUSDB-HQ; 9.2 ft+sparse; multisong avg 9.16 (voc 8.24, dr 10.88, ba 11.76, oth 5.74) | 4; 6 (`htdemucs_6s`); ft variant | Yes, ~1.5× track duration | 3–7 GB VRAM; seconds per track on RTX 3060 class | MIT | No separate license; distributed with MIT repo | `pip install demucs`, Py ≥3.10, PyTorch | Original archived Jan 2025; author fork stable but feature-frozen |
| **BS-RoFormer (paper/lucidrains)** | 9.80 avg MUSDB-HQ (paper, ByteDance); 1st place SDX23 MSS | 4 (paper); community ckpts mostly 2-stem vocals | Poor (well below realtime) | Needed in practice; VRAM scales with chunk size (~8 s default) | MIT (lucidrains) | ByteDance weights **never released**; community ckpts vary | `pip install BS-RoFormer` (arch only, no weights) | lucidrains repo active-ish; weights via community |
| **Mel-Band RoFormer (Kim ckpt)** | 10.98 vocals (multisong) | 2 (vocals/inst) | Poor | Same as BS-RoFormer | MIT (lucidrains/MSST) | **MIT** (HF metadata) | ckpt from HF; run via MSST or audio-separator | Checkpoint static; runners active |
| **BS-RoFormer viperx ckpt** | 10.87 vocals (multisong) | 2 | Poor | Same | — | **None stated** (hosted in UVR's TRvlvr/model_repo) | via UVR/audio-separator | Static |
| **SCNet / SCNet-XL** | SCNet-XL IHF 10.08 avg MUSDB test, 9.92 multisong | 4 | Untested/likely slow | Moderate | MIT | Google Drive (SCNet repo) / MSST GitHub releases, no separate license stated | No PyPI; run via MSST | Research repo, low activity; MSST carries it |
| **MDX23C (ZFTurbo)** | 10.17 vocals | 2 (vocals) | Slow | Yes | MIT (MSST) | Hosted in MSST GitHub releases (MIT repo), no separate statement | via MSST or audio-separator | MSST active (v1.0.21 Apr 2025) |
| **MDX-Net (kuielab)** | 2nd place MDX21 Leaderboard A | 2-stem models in practice | ONNX models OK-ish on CPU | Light VRAM | MIT | UVR-trained models: "honor the MIT license by providing credit to UVR" | Not pip-installable as research repo; consumed via UVR/audio-separator | Research repo inactive; lives on inside UVR |
| **Open-Unmix (umxl)** | umxl median: voc 7.21, dr 7.15, ba 6.02, oth 4.89 | 4 | Yes (lightweight) | Light | MIT | **umxl: CC BY-NC-SA 4.0 (non-commercial)** | `pip install openunmix`, PyTorch | Light maintenance (torch 2.0 compat Apr 2024) |
| **Spleeter** | Well below all above (2019-era) | 2/4/5 | Yes, fast | Yes | MIT | Not separately stated | PyPI but TensorFlow dep, old Python pins, M1 issues | Last GitHub release v2.3.0 (2021); effectively dormant |
| **audio-separator (wrapper)** | n/a (hosts the above) | 2/4/6 depending on model | Yes (model-dependent) | CUDA/CoreML/DirectML | MIT | Inherits per-model | PyPI 0.44.5 (2026-07-20), Py 3.10–3.14, torch+onnxruntime | **Very active** |
| **UVR5 (GUI)** | n/a (hosts models) | 2/4/6 | Yes | GTX 1060 6 GB min | MIT | UVR-trained models MIT-with-credit | Desktop GUI, not a library | v5.6 + Nov 2024 RoFormer beta patches; slow cadence |

---

## Per-model detail

### Demucs v4 / HTDemucs (facebookresearch → adefossez)

- **Repos:** [facebookresearch/demucs](https://github.com/facebookresearch/demucs) — **archived Jan 1, 2025, read-only**; README says "As I am no longer working at Meta, this repository is not maintained anymore. I've created a fork at github.com/adefossez/demucs." The fork [adefossez/demucs](https://github.com/adefossez/demucs) is the canonical home; its author states "I'm not actively working on Demucs anymore, so expect slow replies and no new feature for now." Last PyPI release: v4 (Dec 2022).
- **Quality:** Hybrid Transformer Demucs v4 achieves 9.00 dB SDR on the MUSDB-HQ test set, 9.20 dB with sparse attention and per-source fine-tuning ([repo README](https://github.com/facebookresearch/demucs)). On ZFTurbo's multisong benchmark: HTDemucs4 average 9.16 (bass 11.76, drums 10.88, vocals 8.24, other 5.74) ([MSST pretrained models](https://github.com/ZFTurbo/Music-Source-Separation-Training/blob/main/docs/pretrained_models.md)). `htdemucs_ft` (fine-tuned, bag of 4 models) "will take 4 times more time but might be a bit better." Perceptually, Demucs remains the drums/bass reference; its vocals are now clearly behind RoFormers.
- **Stems:** `htdemucs` (4), `htdemucs_ft` (4, fine-tuned), `htdemucs_6s` (6: +guitar, +piano; piano noted as weak), `hdemucs_mmi` (v3 retrain), plus legacy `mdx`/`mdx_extra` and quantized variants ([README](https://github.com/adefossez/demucs)).
- **Hardware:** CPU separation time "should be roughly equal to 1.5 times the duration of the track" ([README](https://raw.githubusercontent.com/adefossez/demucs/main/README.md)). GPU: minimum 3 GB VRAM, 7 GB recommended for defaults; `--segment` reduces memory (HT models cap at 7.8 s segments). Community benchmarks: ~3 min CPU-only for a 6:24 track on an i5-12400F ([LinuxLinks](https://www.linuxlinks.com/machine-learning-linux-demucs-music-source-separation/2/)); ~7–10 s per track on an RTX 3060 ([Stemuc paper, SBrT](https://biblioteca.sbrt.org.br/articlefile/4978.pdf)).
- **Licenses:** Code MIT. Weights: no separate license stated anywhere in either repo; checkpoints served from `dl.fbaipublicfiles.com` and treated by the ecosystem as falling under the repo's MIT license. Trained on MUSDB HQ plus 800 songs of internal data (README).
- **Packaging:** `pip install demucs`, Python ≥3.10 (Intel Mac capped at 3.12 by PyTorch), PyTorch + torchaudio only, models auto-download on first use.
- **Maintenance:** frozen but stable; the wrapper ecosystem (audio-separator, MSST, UVR) all vendored or wrapped it, which de-risks upstream dormancy.

### BS-RoFormer family (ByteDance paper; lucidrains; community weights)

- **Paper:** "Music Source Separation with Band-Split RoPE Transformer," Lu, Wang, Kong, Hung (SAMI-ByteDance), Sep 2023, [arXiv:2309.02612](https://arxiv.org/abs/2309.02612). 9.80 dB average SDR on MUSDB18-HQ without extra training data; **1st place in the SDX23 (Sound Demixing Challenge) MSS track**. ByteDance never released official weights.
- **Mel-Band RoFormer:** follow-up paper [arXiv:2310.01809](https://arxiv.org/abs/2310.01809) (Wang, Lu, Won, Oct 2023): replaces heuristic band-splits with overlapping mel-scale bands; "outperforms BS-RoFormer in the separation tasks of vocals, drums, and other stems."
- **Reference implementation:** [lucidrains/BS-RoFormer](https://github.com/lucidrains/BS-RoFormer), MIT, `pip install BS-RoFormer`, includes both `BSRoformer` and `MelBandRoformer` classes; architecture only, no weights.
- **Community weights (the ones that matter):** per [ZFTurbo's model zoo](https://github.com/ZFTurbo/Music-Source-Separation-Training/blob/main/docs/pretrained_models.md):
  - BS-RoFormer "viperx" (`model_bs_roformer_ep_317_sdr_12.9755.ckpt`): vocals SDR 10.87 multisong; hosted in [TRvlvr/model_repo releases](https://github.com/TRvlvr/model_repo/releases) (UVR's model host, "all_public_uvr_models" tag) with **no explicit license declaration** visible.
  - Mel-Band RoFormer "Kim": vocals SDR 10.98; [Hugging Face KimberleyJSN/melbandroformer](https://huggingface.co/KimberleyJSN/melbandroformer) with **MIT** license metadata (README itself is empty).
  - BS PolarFormer (ZFTurbo): vocals SDR 11.00, hosted in MSST GitHub releases.
- **Leaderboard:** BS-RoFormer variants dominate the [MVSEP quality-checker leaderboard](https://mvsep.com/quality_checker/leaderboard2.php?sort=insts) — top single-model entries around vocals SDR 11.3–11.4 on the multisong set, positions 5–20 nearly all RoFormer variants; ensembles reach ~11.9 vocals / 18.2 instrumental. Neither Demucs nor Spleeter appears near the top.
- **Stems:** the strong public checkpoints are 2-stem (vocals/instrumental). 4-stem RoFormer checkpoints exist (e.g. MVSep's "BS Roformer SW" 6-stem on the leaderboard) but the best ones are MVSep-internal, not freely downloadable.
- **Hardware:** transformer attention over long spectrogram sequences; much slower than realtime on CPU (community consensus; whole C++/GGML quantized ports exist specifically to make CPU viable — [BSRoformer.cpp](https://github.com/chenmozhijin/BSRoformer.cpp)). VRAM scales with chunk size (default ~352,800 samples ≈ 8 s @ 44.1 kHz); comfortable on 8 GB+ GPUs.
- **Packaging:** no single blessed inference package. Options: ZFTurbo's MSST (`inference.py` + config + ckpt), audio-separator (wraps UVR RoFormer models), lucidrains classes + manual ckpt loading, or [BSRoformer.cpp](https://github.com/chenmozhijin/BSRoformer.cpp) for CPU-quantized.
- **Maintenance:** lucidrains repo has ongoing commits and issue activity; checkpoints are static artifacts; MSST and audio-separator actively maintained.

### MDX-Net (kuielab / KUIELab-MDX-Net)

- **Repo:** [kuielab/mdx-net](https://github.com/kuielab/mdx-net), MIT. Took "2nd place on Leaderboard A and 3rd place on Leaderboard B in the MDX-Challenge ISMIR 2021."
- **Reality check:** it is competition training code (PyTorch Lightning + Hydra, wants 4× ≥2080Ti GPUs and 1.5 TB disk for augmentation). Not an inference product. The usable MDX-Net models are the ONNX checkpoints trained by the UVR team and consumed through UVR/audio-separator.
- **Licensing:** UVR states its self-trained models are covered by MIT: "For all third-party application developers who wish to use our models, please honor the MIT license by providing credit to UVR and its developers" ([UVR repo](https://github.com/Anjok07/ultimatevocalremovergui)).
- **Quality:** superseded — MDX23C (the v3 successor arch, trained by ZFTurbo) hits vocals SDR 10.17 vs ~11 for RoFormers ([MSST model zoo](https://github.com/ZFTurbo/Music-Source-Separation-Training/blob/main/docs/pretrained_models.md)).
- **Verdict for Uncompose:** never integrate the kuielab repo directly; MDX-Net models arrive for free if audio-separator is used.

### Open-Unmix (sigsep)

- **Repo:** [sigsep/open-unmix-pytorch](https://github.com/sigsep/open-unmix-pytorch), MIT code, `pip install openunmix`.
- **Quality:** umxl (best weights) median SDR: vocals 7.21, drums 7.15, bass 6.02, other 4.89 — roughly 2 dB behind Demucs, 4 dB behind RoFormer vocals.
- **License blocker:** **umxl weights are CC BY-NC-SA 4.0, explicitly non-commercial** (repo README). That is incompatible with a license-clean open-source product whose users may use outputs commercially.
- **Maintenance:** light (torch 2.0 compat update April 2024).
- **Verdict:** ruled out — weights license plus lowest quality of the modern options.

### Spleeter (Deezer)

- **Repo:** [deezer/spleeter](https://github.com/deezer/spleeter), MIT code, 2/4/5-stem TensorFlow models; pretrained-weights license not separately stated.
- **Maintenance:** last tagged GitHub release v2.3.0 (Sep 2021, for TF 2.5/Python 3.9) ([releases](https://github.com/deezer/spleeter/releases)); known Apple Silicon/TensorFlow compatibility problems; effectively dormant.
- **Verdict:** ruled out — TensorFlow dependency conflicts with a PyTorch stack, quality is a full generation behind (Demucs paper-era comparisons put Spleeter several dB below), and maintenance is dead. Only remaining virtue is CPU speed, which Demucs already covers acceptably.

### SCNet / SCNet-XL

- **Repo:** [starrytong/SCNet](https://github.com/starrytong/SCNet), MIT, paper [arXiv:2401.13276](https://arxiv.org/abs/2401.13276). Weights via Google Drive links (small + large).
- **Quality:** the SCNet-XL variant trained by ZFTurbo reaches average SDR 10.08 on MUSDB test / 9.92 multisong for 4 stems — the strongest *freely downloadable 4-stem* checkpoint, about +1 dB over HTDemucs ([MSST model zoo](https://github.com/ZFTurbo/Music-Source-Separation-Training/blob/main/docs/pretrained_models.md)).
- **Packaging/maintenance:** no PyPI package; realistically run through MSST. Research repo has modest activity (46 commits).
- **Verdict:** the most interesting *second-wave* 4-stem upgrade for Uncompose (better than Demucs across all four stems), but integration cost is "adopt MSST as a dependency," and CPU behavior is unproven. Watch-list, not first integration.

### ZFTurbo's Music-Source-Separation-Training (MSST)

- **Repo:** [ZFTurbo/Music-Source-Separation-Training](https://github.com/ZFTurbo/Music-Source-Separation-Training), MIT, 600+ commits, releases through v1.0.21 (Apr 20, 2025; cadence of a release every 1–3 months through early 2025) ([releases](https://github.com/ZFTurbo/Music-Source-Separation-Training/releases)).
- **Role:** the community's de-facto training *and* inference framework: BS/Mel RoFormer, MDX23C, SCNet, HTDemucs, Apollo (audio restoration, by JusperLee), BandIt, BS-Mamba2, and more, with a large curated [pretrained-model zoo](https://github.com/ZFTurbo/Music-Source-Separation-Training/blob/main/docs/pretrained_models.md) including SDR numbers per checkpoint.
- **Caveats:** requirements.txt-style install (has a pyproject but is not a polished PyPI library); the model zoo aggregates third-party checkpoints whose individual licenses are mostly undeclared; ZFTurbo's own checkpoints ship from the MIT repo's GitHub releases without a separate weights license statement.

### Wrappers: UVR5 and audio-separator

- **UVR5:** [Anjok07/ultimatevocalremovergui](https://github.com/Anjok07/ultimatevocalremovergui), MIT, Tkinter desktop GUI (Win 10+/macOS Big Sur+/Linux), bundles VR-arch, MDX-Net, MDX23C, Demucs v3/v4; RoFormer/SCNet/BandIt support arrived via v5.6 beta "rofo" patches (Nov 2024 builds on [SourceForge mirror](https://sourceforge.net/projects/ult-vocal-remover-uvr.mirror/files/v5.6/)). GPU minimum GTX 1060 6 GB. It is an end-user app, not a library — relevant to Uncompose mainly as the source/licensor of the model checkpoints ("honor the MIT license by providing credit to UVR"). Release cadence is slow (v5.6 core is 2023 with 2024 patches).
- **audio-separator:** [nomadkaraoke/python-audio-separator](https://github.com/nomadkaraoke/python-audio-separator) / [PyPI `audio-separator`](https://pypi.org/project/audio-separator/). MIT. **v0.44.5 released 2026-07-20** — actively maintained right now. Python 3.10–3.14. CLI + Python API, `--list_models` with metrics, auto-download to a configurable cache. Hardware: CPU, CUDA (11.8/12.x) via ONNX Runtime + torch, Apple CoreML, experimental DirectML. Supports MDX-Net, VR, Demucs (4/6-stem), and MDXC/RoFormer models from UVR. Self-described as code largely derived from UVR. This is the obvious "one dependency, many models" integration point; the trade-off is a heavy dependency tree (torch **and** onnxruntime) and inheriting the per-model license ambiguity of the UVR model zoo.

---

## Open questions and risks

1. **Community RoFormer weights licensing is the biggest legal gray zone.** The viperx BS-RoFormer checkpoint (the most famous vocal model) sits in UVR's [TRvlvr/model_repo](https://github.com/TRvlvr/model_repo/releases) with no license file found; whether UVR's "our models are MIT-with-credit" statement covers third-party-trained checkpoints it merely hosts is unresolved. The Kim Mel-Band RoFormer is tagged MIT on [Hugging Face](https://huggingface.co/KimberleyJSN/melbandroformer) but with an empty model card. Action: for anything shipped by default, prefer (a) Demucs weights, (b) Kim MelBand (MIT tag), (c) ZFTurbo-trained checkpoints from the MIT MSST repo; treat viperx-class weights as user-opt-in downloads, and consider asking authors directly.
2. **Training-data provenance is undisclosed for nearly all top checkpoints** (Demucs used 800 internal songs; community RoFormers use undisclosed datasets). No model in this space offers a data-provenance guarantee; this is an ecosystem-wide condition, not a differentiator, but worth a line in Uncompose's docs.
3. **umxl (Open-Unmix) is CC BY-NC-SA — do not bundle.** Only cleanly excludable if Uncompose never depends on openunmix defaults ([repo](https://github.com/sigsep/open-unmix-pytorch)).
4. **CPU floor is Demucs-only among high-quality models.** RoFormers are decisively better for vocals but effectively require a GPU (or the immature GGML/quantized ports like [BSRoformer.cpp](https://github.com/chenmozhijin/BSRoformer.cpp)). Product implication: architecture should express per-model hardware tiers ("works everywhere" vs "GPU recommended") from day one.
5. **Demucs upstream is frozen.** Both repos are effectively unmaintained (archive + feature-freeze). PyTorch API drift could eventually break `pip install demucs`; wrappers (audio-separator) are the practical hedge, or vendoring the small inference path.
6. **4-stem state of the art is moving to SCNet-XL / multi-stem RoFormers,** available only through MSST-style tooling, not pip packages. If Uncompose's model layer is designed around "checkpoint + config + runner" rather than "pip package," these arrive cheaply later.
7. **MVSEP leaderboard caveat:** top entries are ensembles or MVSep-internal models not all freely downloadable; use ZFTurbo's model-zoo SDR table (same benchmark) for what's actually obtainable ([leaderboard](https://mvsep.com/quality_checker/leaderboard2.php?sort=insts), [model zoo](https://github.com/ZFTurbo/Music-Source-Separation-Training/blob/main/docs/pretrained_models.md)).
8. **Unverified details to confirm at integration time:** exact RAM footprint of Demucs CPU inference on long tracks; SCNet CPU speed; whether audio-separator's Demucs path matches upstream `demucs` output bit-for-bit; current license metadata of each checkpoint audio-separator downloads by default.

---

## Sources

- Demucs (archived original): https://github.com/facebookresearch/demucs
- Demucs (maintained fork + README perf/licensing): https://github.com/adefossez/demucs and https://raw.githubusercontent.com/adefossez/demucs/main/README.md
- ZFTurbo MSST repo: https://github.com/ZFTurbo/Music-Source-Separation-Training
- ZFTurbo model zoo (SDR per checkpoint, download links): https://github.com/ZFTurbo/Music-Source-Separation-Training/blob/main/docs/pretrained_models.md
- ZFTurbo MSST releases: https://github.com/ZFTurbo/Music-Source-Separation-Training/releases
- BS-RoFormer paper: https://arxiv.org/abs/2309.02612
- Mel-Band RoFormer paper: https://arxiv.org/abs/2310.01809
- lucidrains BS-RoFormer implementation: https://github.com/lucidrains/BS-RoFormer
- Kim Mel-Band RoFormer weights (MIT tag): https://huggingface.co/KimberleyJSN/melbandroformer
- UVR model host (viperx ckpt, no license found): https://github.com/TRvlvr/model_repo/releases
- MVSEP quality-checker leaderboard: https://mvsep.com/quality_checker/leaderboard2.php?sort=insts
- KUIELab MDX-Net: https://github.com/kuielab/mdx-net
- UVR5 GUI: https://github.com/Anjok07/ultimatevocalremovergui
- UVR v5.6 RoFormer beta builds (Nov 2024): https://sourceforge.net/projects/ult-vocal-remover-uvr.mirror/files/v5.6/
- Open-Unmix: https://github.com/sigsep/open-unmix-pytorch
- Spleeter: https://github.com/deezer/spleeter and https://github.com/deezer/spleeter/releases
- SCNet: https://github.com/starrytong/SCNet and https://arxiv.org/abs/2401.13276
- audio-separator: https://github.com/nomadkaraoke/python-audio-separator and https://pypi.org/project/audio-separator/
- BSRoformer.cpp (CPU/GGML port): https://github.com/chenmozhijin/BSRoformer.cpp
- Demucs CPU benchmark (i5-12400F): https://www.linuxlinks.com/machine-learning-linux-demucs-music-source-separation/2/
- Demucs GPU benchmark (RTX 3060, Stemuc paper): https://biblioteca.sbrt.org.br/articlefile/4978.pdf
