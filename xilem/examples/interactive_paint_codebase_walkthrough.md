# Interactive Paint Codebase Walkthrough

This is a current-structure walkthrough of [interactive_paint.rs](/Users/davmash/Git/xilem/xilem/examples/interactive_paint.rs). It is meant to be read alongside the source after the performance and benchmark work on `codex/faster-raster-pass`.

Unlike the older snapshot-style walkthrough, this document focuses on the major subsystems and how they fit together now:

- preset and geometry generation
- app state vs widget state
- Xilem view construction
- Masonry widget behavior
- fractal expansion and rasterization
- benchmark modes and performance-specific code
- entrypoints and tests

## 1. File Shape

`interactive_paint.rs` is doing four jobs in one file:

1. define the fractal preset library
2. define the interactive app state and UI
3. define the custom canvas widget and render pipeline
4. define a benchmark harness for serial, parallel, and vector comparisons

That means the file is not just an example UI. It is also the benchmark lab for the rendering experiments.

The high-level order in the file is:

1. imports and constants
2. preset definitions and geometry helpers
3. app state and view bridge types
4. custom widget implementation
5. interaction and geometry math helpers
6. rasterization helpers
7. benchmark helpers
8. `main()` and tests

## 2. Constants And Global Configuration

At the top of the file you will find the configuration constants that shape both the UI and the renderer:

- canvas dimensions
- handle radii and hit-testing tolerance
- `MIN_SEGMENT_LENGTH`
- render batch budget
- preset layout settings
- fractal color
- benchmark parallel split factor

The most important constant for performance behavior is `MIN_SEGMENT_LENGTH`. That is the cutoff that stops recursive expansion and rasterizes a segment directly. It is the reason some presets saturate early even at very high nominal depth.

The most important constant for benchmark parallelism is `BENCHMARK_PARALLEL_SPLIT_FACTOR`. It controls how aggressively the benchmark expands the initial frontier before distributing work to threads.

## 3. Preset System

The preset system is more sophisticated than the original three-shape version.

### Core types

- `PresetGroup`
- `GeneratorKind`
- `PresetSpec`

`PresetGroup` is just for UI organization. It separates `Classic` from `Experiment`.

`GeneratorKind` is the key abstraction:

- `Turtle { commands, angle_deg }`
- `Points(&'static [(f64, f64)])`

That lets a preset either be defined directly by explicit control points or indirectly through turtle commands that are converted into a polyline.

`PRESETS` is the master catalog. It drives the preset buttons, geometry generation, and benchmark target lookup.

### Why this matters

This was a major structural improvement because it decoupled “preset idea” from “manually placed points”. It made it easy to add many more presets and benchmark families without hand-authoring every shape.

## 4. Geometry Model

The central data model is `FractalGeometry`.

It stores:

- `generator_points`
- `baseline_points`
- `endpoint_docked`

This is the full editable fractal definition:

- the red polyline is the generator
- the blue baseline defines how the generator is normalized and remapped
- the docking flags determine whether generator endpoints stay attached to the baseline endpoints

Important helper methods and constructors around geometry:

- `geometry_from_points(...)`
- `geometry_from_turtle(...)`
- `preset_geometry(...)`
- `normalized_points_from_baseline(...)`
- `local_segments_from_baseline(...)`

The normalization helpers are the mathematical core of the whole example. They convert a world-space polyline into a local baseline-relative representation so the same generator can be reapplied recursively to any segment.

## 5. App State Layer

`InteractivePaintApp` is the Xilem-side app model.

It stores:

- current depth
- text input backing string for depth
- show-guides flag
- committed geometry
- selected preset name

This state is what the declarative UI reads from and writes to.

Important point: this is not the whole interaction model. During dragging, the widget owns transient geometry state locally and only commits the result back into app state when appropriate.

That split is the reason the example feels responsive without forcing every pointer move through the top-level app state.

## 6. Message And Bridge Layer

The bridge from Xilem to Masonry lives in:

- `CanvasAction`
- `CanvasView`

`CanvasAction` currently carries committed geometry changes from the widget back to app state.

`CanvasView` is the custom view wrapper that:

- builds the widget
- rebuilds it when app state changes
- translates widget actions back into `InteractivePaintApp`

This section is important because it is where the app-state/widget-state contract is enforced.

### Rebuild behavior

`CanvasView::rebuild(...)` is careful about what forces rerender work:

- guide-only changes should not rebuild geometry work
- geometry or depth changes should reset render progress

This was one of the earlier correctness issues that got fixed while cleaning up the example.

## 7. Widget State Layer

`CanvasWidget` is where most of the interesting runtime behavior lives.

It stores:

- current widget-local geometry
- depth
- guide visibility
- drag bookkeeping
- endpoint reveal state
- double-click tracking
- cached `local_segments`
- pending segment work stack
- blank image
- front image
- back buffer

This is the heart of the interactive example.

### Why there are both `front_image` and `back_buffer`

The widget uses a front/back image arrangement:

- `front_image` is what paint draws right now
- `back_buffer` is where incremental raster work accumulates

When rendering completes, the back buffer is converted into a new image brush and published to the front. That removed paint-time image reconstruction from cloned bytes and made the live widget structure cleaner.

## 8. Widget Lifecycle

The key widget methods are:

- `on_anim_frame(...)`
- `on_pointer_event(...)`
- `update(...)`
- `paint(...)`

### `on_anim_frame(...)`

This is the incremental renderer.

It:

1. pops `SegmentJob`s from `pending_segments`
2. decides whether to subdivide or rasterize directly
3. writes into `back_buffer`
4. republishes the buffer when work is done

This method uses a frame deadline based on `RENDER_BATCH_BUDGET`. That keeps the interactive app responsive while still allowing deep fractal generation.

### `on_pointer_event(...)`

This is the interaction controller.

It handles:

- baseline and point hit testing
- nested endpoint reveal toggles
- pointer capture
- drag initialization
- geometry mutation during drag
- final geometry commit on release

The important detail is that dragging updates widget-local geometry first. App-level geometry only updates when the widget submits a `CanvasAction::Geometry(...)`.

### `update(...)`

This is mostly used to initialize rendering when the widget is first added.

### `paint(...)`

This draws:

1. background panel
2. preview rect
3. current `front_image`
4. baseline and guide overlays
5. handles

The fractal itself is a raster image. The guides and handles are vector overlays.

## 9. Render Reset And Publish

Two widget helpers matter a lot:

- `reset_render_progress()`
- `publish_back_buffer()`

`reset_render_progress()`:

- clears the back buffer
- rebuilds cached `local_segments`
- clears pending work
- resets the displayed image
- seeds the baseline job

`publish_back_buffer()`:

- swaps out the completed back buffer
- converts it into a new `ImageBrush`
- installs it as `front_image`

This pair forms the live widget render lifecycle.

## 10. Hit Testing And Drag Targets

The interaction layer uses `DragTarget` plus a set of hit-test helpers:

- `hit_test(...)`
- `baseline_hit(...)`
- `revealed_endpoint_handle(...)`
- geometric helpers like `point_near_polyline(...)`

The ordering is intentional:

1. nested endpoint editor
2. baseline handles
3. generator points
4. generator polyline
5. baseline line

That ordering defines z-priority for interaction.

## 11. Geometry Editing Math

The geometry edit behavior is driven by:

- `apply_drag(...)`
- helper functions for endpoint docking and baseline updates

This is where the example decides how moving one control point should affect the rest of the geometry.

The docking behavior is especially important:

- if an endpoint is docked, some drags preserve that attachment
- dragging the displayed baseline endpoint can transform interior points relative to the new baseline
- dragging the overall generator shape translates the generator without moving the baseline

The tests at the end of the file cover several of these cases directly.

## 12. Normalization And Recursive Mapping

The fractal recursion depends on three functions:

- `normalized_points_from_baseline(...)`
- `map_local(...)`
- `push_transformed_segments(...)`

### `normalized_points_from_baseline(...)`

This converts the generator polyline into baseline-relative local coordinates:

- local x is distance along the baseline
- local y is perpendicular offset from the baseline

### `map_local(...)`

This remaps a local point back into world space for a specific segment.

### `push_transformed_segments(...)`

This is the optimized expansion helper added during the perf work.

Instead of recomputing the parent segment frame separately for every child endpoint, it computes:

- direction vector
- perpendicular vector

once per parent job, then applies them to every cached local segment.

That made a real difference in the benchmark path and also simplified the expansion logic shared by the widget and benchmark code.

## 13. Rasterization Layer

The rasterization stack is:

- `rasterize_line(...)`
- `rasterize_line_opaque(...)`
- `set_pixel_opaque(...)`
- `blend_pixel(...)`

### Why two raster paths exist

The fractal uses a fully opaque color, so the hot path can skip alpha compositing.

That means:

- `rasterize_line(...)` dispatches to the opaque fast path when alpha is `255`
- only non-opaque colors use the slower blended path

### `rasterize_line_opaque(...)`

This is the important one for performance. It is the retained integer Bresenham path. The current version includes an in-bounds fast path that avoids repeated pixel-boundary checks and updates the byte offset incrementally.

This was one of the major throughput wins.

## 14. Benchmark Data Structures

The benchmark section introduces:

- `BenchmarkStats`
- `BenchmarkScratch`
- `RasterBenchmarkMode`

`BenchmarkScratch` exists to reuse allocations across iterations instead of rebuilding the buffer and job stack every run.

`RasterBenchmarkMode` is the serial-vs-parallel selector used by adaptive mode.

## 15. Serial Benchmark Path

The serial benchmark flow is:

- `seed_benchmark_jobs(...)`
- `run_raster_jobs(...)`
- `benchmark_rasterize(...)`

`run_raster_jobs(...)` is the shared serial core that consumes a pending stack and either subdivides or rasterizes until the stack is empty.

This is intentionally close to the live-widget recursion logic, but stripped of UI concerns.

## 16. Parallel Benchmark Path

The parallel path is benchmark-only and lives in:

- `benchmark_parallel_rasterize(...)`
- `split_benchmark_frontier(...)`
- `merge_opaque_buffers(...)`
- `benchmark_worker_count(...)`

### How it works

1. seed the top-level baseline job
2. expand the frontier until there are enough jobs to distribute
3. shard frontier jobs across workers
4. let each worker rasterize into its own local RGBA buffer
5. merge the finished opaque buffers

### Why it is benchmark-only

This path is ideal for measuring total CPU throughput, but it is not automatically a good fit for the live widget:

- it allocates multiple large buffers
- it performs a final merge step
- it would need more UI-thread and ownership integration work for interactive use

## 17. Adaptive Benchmark Selection

Adaptive mode lives in:

- `choose_raster_benchmark_mode(...)`
- `raster_benchmark_mode_label(...)`
- `run_benchmark()`

The logic today is simple:

- stay serial for small workloads
- switch to parallel once actual work crosses the configured threshold

This is a benchmark reporting policy, not a live renderer policy.

It exists because:

- parallel wins big on `Peano Serpent` and larger `Gosper` rows
- parallel loses badly on saturated cases like `Koch`, `Metro Weave`, and `Switchback`

The performance report documents the current edge-case weakness near `Gosper Seed` depth 7.

## 18. Vector Benchmark Path

The vector comparison path is in `benchmark_vector_scene(...)`.

It expands the same fractal jobs but records each final segment as a Vello stroke in a `Scene` instead of writing pixels.

This exists mainly as a comparison point.

The result so far has been clear: for this workload, the raster path wins. Building a huge scene of line strokes is much slower than the optimized CPU raster path.

## 19. `run_benchmark()`

`run_benchmark()` ties the benchmark system together.

It:

- prints benchmark context
- iterates benchmark presets
- iterates the chosen depth set for each branch factor
- measures serial raster
- measures adaptive serial-or-parallel
- measures vector scene construction
- reports theoretical and actual work counts plus throughput

This is the main function you should read if you want to understand what the numbers in the performance report actually mean.

## 20. Entrypoints

The executable entrypoints are:

- `main()`
- `android_main(...)`

`main()` checks for `--benchmark` and either:

- runs the benchmark harness, or
- launches the interactive desktop app

That dual-purpose structure is why this example acts both like a demo and a benchmark program.

## 21. Tests

The tests at the end of the file are small but useful:

- baseline-relative normalization works
- `map_local(...)` respects the segment frame
- dragging the generator shape moves only generator points
- dragging display endpoints transforms interior control points
- preset library counts and geometry validity stay sane

These do not prove performance, but they do protect the geometry and interaction behavior that the benchmark depends on.

## 22. Suggested Reading Order In The Source

If you want to understand the file quickly, read in this order:

1. `InteractivePaintApp`
2. `CanvasView`
3. `CanvasWidget`
4. `on_anim_frame(...)`
5. `reset_render_progress()`
6. `paint(...)`
7. `normalized_points_from_baseline(...)`
8. `push_transformed_segments(...)`
9. `rasterize_line_opaque(...)`
10. `benchmark_rasterize(...)`
11. `benchmark_parallel_rasterize(...)`
12. `run_benchmark()`

That path takes you from UI state to widget state to recursion math to performance harness in the same conceptual order the example actually runs.

## 23. Current Mental Model

The cleanest way to think about the example now is:

- Xilem owns the declarative app and committed state
- Masonry owns the custom widget and transient interaction state
- Vello draws the UI shell and guide overlays
- the fractal itself is generated by recursive segment expansion
- the final image is still best produced by optimized CPU rasterization
- benchmark mode explores when serial, parallel, or vector approaches win

That is the current architecture of `interactive_paint.rs`.
