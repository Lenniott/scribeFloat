# Intel Mac & Windows Transcription Performance — Deep Dive

> **Scope:** Why transcription is slow on Intel Macs and Windows (even on *tiny*),
> how much speed is realistically recoverable, and at what effort/risk.
> **Status:** Research only. No code changed by this document.
> **Companion:** Implementation-ready follow-ups live in [`remaining-work.md`](./remaining-work.md);
> this doc explains the *why* and *how much* behind them.

---

## 1. Executive summary

**Can we speed it up? Yes — meaningfully, on CPU, with low risk.** A realistic
combined **~3–6×** is available on Intel/Windows from changes that touch only the
build configuration and the model catalog:

| Lever | Realistic gain (CPU) | Effort | Risk |
|---|---|---|---|
| Q4_0 quantized models | ~3–5× decode | Low | ~≤0.1 WER |
| OpenBLAS + OpenMP on non-Apple builds | ~3–4× **encoder** | Medium (CI + bundling) | Low |
| Thread cap = physical cores | *already done* | — | — |
| GPU offload (Vulkan / OpenVINO on iGPU) | reported 3×+ more, **encoder only** | High | Driver/bundle |

**Can we match Apple Silicon? No — not on CPU.** M-series wins because the
`aarch64-macos` build runs the **Metal GPU** path; Intel/Windows run a **CPU-only**
build. Public benchmarks put M-series Metal roughly **an order of magnitude** ahead of
a comparable Intel i7 doing CPU-only inference. CPU tuning (BLAS + Q4_0) *narrows* the
gap; only **iGPU offload** narrows it further, and even then the decoder stays CPU-bound
so the encoder-only speedup doesn't fully close it.

**The single most important finding:** the current non-Apple build has **no math
acceleration backend at all** — and a code comment claims otherwise (see §2).

---

## 2. Current state & root cause

### 2.1 What each platform actually runs today

| Platform | Build path | Acceleration in effect |
|---|---|---|
| Apple Silicon (`aarch64-macos`) | `whisper-rs` `metal` feature | **Metal GPU** + Apple Accelerate/BLAS |
| **Intel Mac** (`x86_64-macos`) | default `whisper-rs`, no features | **Generic CPU** — Apple BLAS flags *no-op* (see below) |
| **Windows** (`x86_64`) | default `whisper-rs`, no features | **Generic CPU**, no BLAS |

Source of truth:

- `src-tauri/Cargo.toml:33` — `whisper-rs = "0.16"` (no features).
- `src-tauri/Cargo.toml:49-50` — `metal` feature **only** for
  `cfg(all(target_arch = "aarch64", target_os = "macos"))`.

### 2.2 The misleading comment

`src-tauri/Cargo.toml:31-32` states:

> *"Apple Silicon gets Metal GPU acceleration; other targets use the CPU build
> (Accelerate + AVX2 are wired in via `.cargo/config.toml` CMAKE_ARGS)."*

But `.cargo/config.toml:10` sets only:

```
CMAKE_ARGS = "-DGGML_ACCELERATE=ON -DGGML_BLAS=ON -DGGML_BLAS_VENDOR=Apple"
```

`GGML_BLAS_VENDOR=Apple` resolves Apple's Accelerate framework — which **does not
exist on Windows and is irrelevant on Intel without Accelerate linkage**. On non-Apple
targets ggml's CMake `find_package`/`find_library` for the Apple vendor **fails soft and
silently disables BLAS** (the config comment at `.cargo/config.toml:4-6` acknowledges
this "gracefully no-ops" behaviour). There is **no `-march`/AVX flag here either** — that
phrase in the comment is aspirational, not real.

**Net:** Windows and Intel Mac get a *plain* whisper.cpp CPU build with **no GEMM
backend**. whisper.cpp still auto-uses **AVX2/FMA/F16C at runtime** (so it isn't the
worst case), but the encoder's matmuls run without an optimized BLAS — which is exactly
the part BLAS and the Metal path accelerate most.

### 2.3 What is already well-tuned (don't regress)

- **Thread count** — `services/model.rs:13-16,576` caps inference at *physical* cores
  (max 8). This is correct: hyperthreading hurts matmul-heavy work. Confirmed by
  whisper.cpp community findings (diminishing returns past physical cores).
- **Context caching** — `services/model.rs:92-103` caches `WhisperContext` per model
  path, removing the ~300 ms (tiny) → ~2 s (large) cold-load tax per transcribe.
- **Sampling** — `FullParams::new(SamplingStrategy::Greedy { best_of: 1 })` is already
  the fastest strategy; beam search would be slower.
- **Record-start preload** — tiny/base preloaded on record start
  (`PRELOAD_ELIGIBLE_MODEL_IDS`, `model.rs:21`) so stop→transcribe is instant.
- **RTF logging already exists** — every transcribe emits
  `[transcribe] model=… audio=…s wall=…s rtf=…x threads=…`. **This is the measurement
  tool** for everything below; we can quantify each change with real numbers.

---

## 3. Where the time goes

Whisper has two phases with very different acceleration profiles:

- **Encoder** — large, dense matmuls over the mel spectrogram. **BLAS and GPU offload
  help here the most.** This is the bulk of the Apple-vs-Intel gap.
- **Decoder** — autoregressive, token-by-token, latency-bound. BLAS helps little; GPU
  offload helps only partially. On a long recording this becomes the floor.

Implications:
- On *tiny*, the model is small enough that **compute, not RAM**, is the bottleneck —
  16 GB is plenty. So "more RAM" won't help; faster math will.
- BLAS gives a big *encoder* win but the *decoder* still runs on CPU, so end-to-end gains
  are below the headline encoder number.
- Quantization (Q4_0) attacks the **decoder/memory-bandwidth** side, which is why it
  composes well with BLAS (different bottleneck).

---

## 4. Levers, ranked by payoff / effort / risk

### Tier 1 — ship first (CPU, low risk, high payoff)

**A. Q4_0 quantized models (~3–5× decode, ≤0.1 WER)**
Current catalog is Q5_1/Q5_0 (`services/model.rs:44-90`). On **CPU**, Q5 carries an
unpacking penalty; **Q4_0 is the fastest-decoding format** for CPU-only inference with
negligible accuracy loss. This is already fully scoped as
[`remaining-work.md §1`](./remaining-work.md) (catalog swap + `-q5`→`-q4` id migration in
`config.rs`). Highest payoff for the least code.

**B. OpenBLAS + OpenMP on non-Apple targets (~3–4× encoder)**
`whisper-rs` exposes `openblas` and `openmp` cargo features (verified in the whisper-rs
source `[features]`). Enable them for non-Apple targets, e.g.:

```toml
# Cargo.toml — non-Apple gets an actual GEMM backend
[target.'cfg(not(target_os = "macos"))'.dependencies]
whisper-rs = { version = "0.16", features = ["openblas", "openmp"] }
# (Intel macOS could instead keep Accelerate by fixing the CMAKE_ARGS vendor.)
```

whisper.cpp documents BLAS via `-DGGML_BLAS=1` (OpenBLAS vendor). **The catch is
distribution** (see §6): the OpenBLAS runtime must be **bundled** into the shipped DMG/EXE
and the CI runners must have OpenBLAS available at build time. This is the main cost of
this lever — not the code, the packaging.

> Intel MKL is reportedly ~10–20% faster than OpenBLAS on Intel, but adds licensing and a
> much larger redistributable. Not worth it for v1.

**C. Intel-macOS: fix the Accelerate vendor (free win on that platform)**
The Apple BLAS flags only help if Accelerate is actually linked. Verifying/repairing the
`CMAKE_ARGS` so Intel macOS links Accelerate (or switching it to OpenBLAS like Windows)
recovers the encoder GEMM win the comment *claims* already exists.

### Tier 2 — defer until baselines exist (GPU, high effort)

**D. iGPU offload via Vulkan or OpenVINO (encoder-only, reported 3×+)**
`whisper-rs` exposes `vulkan` and `intel-sycl`; whisper.cpp documents Vulkan
(`-DGGML_VULKAN=1`, "cross-vendor") and OpenVINO (`-DWHISPER_OPENVINO=1`, with a noted
**slow first run** while it compiles the IR blob). Third-party reports (e.g. Phoronix on
whisper.cpp 1.8.3) cite large iGPU speedups, but **the official README publishes no
Vulkan/OpenVINO numbers**, so treat specific multipliers as unverified. This is the only
path that *narrows the Apple gap further*, but it adds driver/runtime bundling and
per-machine variability. **Recommendation: defer** until we have Tier-1 RTF baselines from
real Intel/Windows users (via the `[transcribe] rtf=` log).

### Non-levers / things already correct

- **Threading** — already physical-core capped. Possible future tweak: revisit the
  hard cap of 8 on high-core i9/i7 desktops, but low priority.
- **`-march=native` — do NOT use.** See §6.
- **More RAM** — not the bottleneck for tiny/base (§3).

---

## 5. Benchmark reality (set expectations)

| Setup | Relative speed (qualitative) |
|---|---|
| Apple Silicon + Metal | baseline "fast" (current good experience) |
| Intel i7 **CPU-only, today** | ~order of magnitude slower than M-series |
| Intel i7 CPU + Q4_0 + OpenBLAS | ~3–6× faster than today; still slower than M-series |
| Intel i7 + iGPU (Vulkan/OpenVINO) | encoder accelerated; decoder still CPU |

**Caveat on numbers:** public head-to-head i7-CPU-vs-M1 RTF figures are sparse and the
strong iGPU multipliers come from third-party posts, not the whisper.cpp README. Rather
than commit to a target multiplier, **collect real RTF from the existing
`[transcribe] rtf=` log** on a few Intel/Windows machines before and after Tier 1. That
turns this from estimate into measurement and gives the go/no-go signal for Tier 2.

---

## 6. Distribution constraints (these shape the solution)

Releases are built in **GitHub Actions** and shipped as **DMG (mac) / EXE (Windows)** that
arbitrary users download. Therefore:

1. **No `-march=native` / `target-cpu=native`.** A binary built on a CI runner with newer
   SIMD will **crash with "illegal instruction"** on an older user CPU. whisper.cpp already
   runtime-detects AVX2/FMA/F16C — rely on that; keep a safe x86-64 baseline. (Also watch
   for the historical AVX512-forced-build crash class when bumping whisper.cpp versions.)
2. **BLAS must be bundled.** OpenBLAS is a runtime dependency — the `.dll` (Windows) /
   `.dylib` (Intel `.app`) must be packaged via Tauri `resources`/`externalBin`, and the CI
   build job must install OpenBLAS (e.g. vcpkg on the Windows runner) so the link succeeds.
   This packaging work — not the Rust change — is the real effort behind Tier 1B.
3. **GPU runtimes are worse** for bundling (Vulkan loader / OpenVINO runtime), reinforcing
   the "defer Tier 2" call.

---

## 7. Recommended sequencing

1. **Tier 1A — Q4_0 catalog swap** ([`remaining-work.md §1`](./remaining-work.md)). Biggest
   payoff per line of code; no new dependencies.
2. **Tier 1C — fix Intel-macOS Accelerate vendor.** Cheap; recovers the win the comment
   already promised.
3. **Tier 1B — OpenBLAS + OpenMP on Windows (+ Intel mac if not using Accelerate).** Gate by
   `cfg(not(target_os = "macos"))`; do the CI + bundle work; correct the misleading
   `Cargo.toml:31-32` comment as part of this.
4. **Measure** — gather `[transcribe] rtf=` numbers from Intel/Windows users.
5. **Tier 2 — GPU offload (Vulkan/OpenVINO).** Only if the measured CPU result is still
   short of acceptable. Ties into the two-pass draft/refine idea in
   [`remaining-work.md §4`](./remaining-work.md).

Low-RAM is a separate UX concern (medium/large on 8 GB), already scoped in
[`remaining-work.md §3`](./remaining-work.md) — not a speed lever for tiny/base.

---

## 8. Sources

Verified directly during this research:

- **whisper-rs cargo features** (`openblas`, `openmp`, `metal`, `cuda`, `vulkan`,
  `hipblas`, `coreml`, `intel-sycl`) — whisper-rs `Cargo.toml` `[features]`,
  <https://github.com/tazz4843/whisper-rs>
- **whisper.cpp acceleration backends & build flags** (`GGML_BLAS=1` OpenBLAS,
  `WHISPER_COREML=1` ">x3 vs CPU", `WHISPER_OPENVINO=1` slow-first-run, `GGML_VULKAN=1`
  cross-vendor) — whisper.cpp README, <https://github.com/ggml-org/whisper.cpp>

Reported by third parties (directionally useful; specific multipliers **not** independently
verified — the whisper.cpp README publishes no Vulkan/OpenVINO numbers):

- whisper.cpp thread scaling / physical-core sweet spot — whisper.cpp discussions #200, #403.
- OpenBLAS ~3–4× encoder; encoder-vs-decoder split — whisper.cpp discussions #589, #2662.
- SYCL/Vulkan/BLAS on Intel comparison — whisper.cpp discussion #2996.
- Q4_0 fastest on CPU; quantization tradeoffs — whisper.cpp discussion #3752; quantization
  analysis, arXiv:2503.09905.
- `-march=native` illegal-instruction & AVX512 forced-build crashes — whisper.cpp issues
  #290, #2928.
- iGPU "12×" headline — Phoronix, whisper.cpp 1.8.3 (could not re-fetch; treat as a claim).
