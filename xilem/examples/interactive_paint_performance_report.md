# Interactive Paint Performance Report

Date: 2026-04-07

## Scope

This report summarizes the performance work done on the `interactive_paint` example, what changed, what actually helped, and how to benchmark it correctly.

The main lesson is simple:

- `dev` benchmarks were misleadingly slow.
- `--release` Rust is already much faster than the earlier JS baseline.
- `-C target-cpu=native` pushes the serial path further.
- Parallel rasterization is a major win for genuinely large-growth presets.
- Parallel rasterization is a loss for small or saturated presets.
- The benchmark now uses an adaptive serial/parallel choice to avoid the obvious losses.

## Benchmark Commands

Use these commands from the workspace root:

```bash
cargo interactive-paint-bench
RUSTFLAGS='-C target-cpu=native' cargo interactive-paint-bench-native
```

Those aliases are defined in [.cargo/config.toml](/Users/davmash/Git/xilem/.cargo/config.toml).

Do not compare JS against plain `cargo run` in `dev`. The `dev` profile understated Rust performance by a very large margin.

## Important Commits

- `ffa082e9` `Optimize interactive paint raster benchmarks`
- `3a8d7ab6` `Speed up interactive paint segment expansion`
- `18664724` `Add release and parallel interactive paint benchmarks`
- `d88af224` `Make interactive paint benchmarks adapt to workload size`
- `36d7aec3` `Add interactive paint benchmark cargo aliases`

The current branch for this work is `codex/faster-raster-pass`.

## What Changed

### Kept raster-path improvements

- Opaque Bresenham raster path for fully opaque fractal lines.
- Front/back image buffering for the live widget.
- Cached local generator segments.
- Reused benchmark scratch allocations.
- Reused parent segment frame math when expanding child segments.

### Benchmark-only improvements

- Added a parallel raster benchmark path using `std::thread::scope`.
- Split the job frontier into worker shards.
- Rasterized into per-thread buffers and merged them afterward.
- Added a benchmark warning for non-`--release` runs.
- Added adaptive mode so the benchmark can choose serial or parallel by workload size.

### What did not win

- Vector scene construction was consistently slower than the raster path for this workload.
- Parallel rasterization was much worse on small or already saturated presets.
- More low-level scalar pixel tricks were not consistently worth keeping.

## Presets That Matter Most

For high-growth stress tests:

- `Peano Serpent`
- `Gosper Seed`

For saturated workloads where parallelism usually loses:

- `Koch Curve`
- `Minkowski Sausage`
- `Metro Weave`
- `Switchback`

## Baseline Findings

### Before correct optimization context

Early comparisons were mostly made against `dev` Rust builds. That made Rust look slower than JS.

That conclusion was wrong for real throughput.

### Release-mode correction

Once the same benchmark was run in `--release`, Rust was already clearly ahead of the earlier JS numbers.

Representative `--release` results:

| Preset / depth | Release raster |
|---|---:|
| Peano Serpent 7 | 104.3M lines/sec |
| Peano Serpent 6 | 73.3M lines/sec |
| Gosper Seed 8 | 100.7M lines/sec |
| Metro Weave 8 | 115.4M lines/sec |
| Switchback 8 | 92.1M lines/sec |

Against the earlier JS numbers from this machine:

| Preset / depth | Rust release | JS | Rust advantage |
|---|---:|---:|---:|
| Peano Serpent 7 | 104.3M | 16.97M | about 6.1x |
| Gosper Seed 8 | 100.7M | 15.94M | about 6.3x |
| Metro Weave 8 | 115.4M | 18.46M | about 6.3x |
| Switchback 8 | 92.1M | 10.85M | about 8.5x |

## Native CPU Results

Using:

```bash
RUSTFLAGS='-C target-cpu=native' cargo interactive-paint-bench-native
```

Representative serial results:

| Preset / depth | Native serial |
|---|---:|
| Peano Serpent 8 | 85.9M to 100.2M |
| Peano Serpent 7 | 99.8M to 104.5M |
| Peano Serpent 6 | 57.2M to 75.6M |
| Gosper Seed 8 | 84.7M to 97.8M |
| Gosper Seed 7 | 71.6M to 87.0M |
| Metro Weave 8 | 81.2M to 114.6M |

The benchmark is somewhat noisy between runs, especially on mid-sized cases.

## Serial vs Parallel vs Adaptive

Adaptive does not introduce a third renderer. It only chooses between serial and parallel.

### Cases where parallel is clearly a win

| Preset / depth | Serial | Parallel or adaptive parallel | Result |
|---|---:|---:|---|
| Peano Serpent 5 | 47.3M to 47.6M | 102.8M to 113.2M | parallel wins |
| Peano Serpent 6 | 57.2M to 75.6M | 230.9M to 274.0M | parallel wins |
| Peano Serpent 7 | 99.8M to 104.5M | 380.4M to 437.8M | parallel wins |
| Peano Serpent 8 | 85.9M to 100.2M | 255.9M to 410.4M | parallel wins |
| Gosper Seed 6 | 55.3M to 67.9M | 106.7M to 120.5M | parallel wins |
| Gosper Seed 8 | 84.7M to 97.8M | 132.1M to 147.3M | parallel usually wins |

### Cases where serial is clearly better

| Preset / depth | Serial | Parallel | Result |
|---|---:|---:|---|
| Koch Curve 16 | 84.2M to 86.7M | about 1.3M to 1.5M | serial wins hard |
| Minkowski Sausage 8 | 45.1M to 96.5M | about 11M to 12M | serial wins |
| Metro Weave 8 | 81.2M to 114.6M | about 11M to 12M | serial wins |
| Switchback 8 | 83.3M to 85.7M | about 2.1M | serial wins |

### Why this happens

Parallel rasterization has real overhead:

- thread scheduling
- frontier splitting
- per-thread buffer allocation
- final buffer merge

That overhead is worth paying only when the job graph is large enough.

## Adaptive Heuristic

The current benchmark chooses parallel when:

- more than one worker is available, and
- `rasterized_lines >= 50_000` or `expanded_jobs >= 100_000`

Otherwise it stays serial.

This behavior is implemented in [interactive_paint.rs](/Users/davmash/Git/xilem/xilem/examples/interactive_paint.rs#L1770).

### What adaptive gets right

- It stays serial for `Koch`, `Minkowski`, `Metro Weave`, and `Switchback`.
- It switches to parallel for large `Peano Serpent` cases.
- It switches to parallel for larger `Gosper Seed` cases.

### Edge-case weakness

The weakest boundary is `Gosper Seed` around depth `7`.

In one earlier native run, adaptive selected parallel and lost:

| Preset / depth | Serial | Adaptive parallel |
|---|---:|---:|
| Gosper Seed 7 | 32.6M | 26.9M |

That specific failure did not reproduce consistently in later reruns. Later runs showed parallel winning on the same row.

So the current issue is benchmark variance near the threshold, not a cleanly wrong threshold.

## Best Current Interpretation

- Rust is significantly faster than the earlier JS baseline when benchmarked correctly.
- The most meaningful throughput path is optimized CPU rasterization, not vector scene construction.
- `target-cpu=native` is worth using for local machine-ceiling measurements.
- Parallel rasterization is best treated as a workload-specific optimization, not a universal default.
- Adaptive mode is the right benchmark policy for now, but it is still sensitive to noise near `Gosper Seed` depth `7`.

## Recommended Next Step

If the adaptive benchmark needs to be made more robust, the next improvement should be:

- keep the current rule as a first pass
- run a small serial-vs-parallel probe only near the threshold
- choose the winner based on those probe timings instead of on work-size rules alone

That would preserve the big `Peano Serpent` wins while reducing the chance of a noisy mis-pick on edge rows.
