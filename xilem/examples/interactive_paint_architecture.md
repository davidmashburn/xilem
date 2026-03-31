# Interactive Paint Example - Architecture Summary

This document explains the key architecture concepts demonstrated in `interactive_paint.rs`, a line fractal explorer that combines Xilem's reactive UI with a custom Vello-based canvas widget.

## Layered Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Xilem View Layer                                       │
│  - `app_logic()` generates declarative view tree        │
│  - `CanvasView` bridges to custom widget                │
│  - Messages flow up, state flows down                   │
├─────────────────────────────────────────────────────────┤
│  Masonry Widget Layer                                   │
│  - `CanvasWidget` implements `Widget` trait             │
│  - Pass system: events → update → layout → paint       │
│  - `WidgetPod` wraps the widget                        │
├─────────────────────────────────────────────────────────┤
│  Vello Rendering Layer                                  │
│  - `Scene` accumulates drawing commands                 │
│  - `ImageBrush` draws rasterized backing buffer        │
│  - `Stroke`, `Fill` primitives for guides               │
└─────────────────────────────────────────────────────────┘
```

## State Architecture

The app uses a **bidirectional state model**:

### App State (Xilem Layer)
```rust
struct InteractivePaintApp {
    depth: usize,
    geometry: FractalGeometry,     // Committed state from canvas
    show_guides: bool,
    // ...
}
```
- Lives in Xilem's centralized state
- Updated when canvas commits changes (`PointerEvent::Up`)
- Drives UI controls (preset buttons, depth input)

### Widget State (Masonry Layer)
```rust
struct CanvasWidget {
    geometry: FractalGeometry,     // Transient during drag
    depth: usize,
    drag_target: Option<DragTarget>,
    pending_segments: VecDeque<SegmentJob>,  // Incremental render queue
    raster_buffer: Vec<u8>,        // RGBA backing buffer
    // ...
}
```
- Owns interactive state during drag operations
- Only commits to app state on `PointerEvent::Up`
- Maintains raster buffer and render work queue

## View Bridge Pattern

The `CanvasView` struct connects Xilem's view system to the custom Masonry widget:

```rust
impl View<Edit<InteractivePaintApp>, (), ViewCtx> for CanvasView {
    type Element = Pod<CanvasWidget>;  // Pod wraps widget for Xilem
    
    fn build(&self, ctx: &mut ViewCtx, app_state: &AppData) -> Self::Element {
        let widget = CanvasWidget { /* ... */ };
        ctx.with_action_widget(|ctx| ctx.create_pod(widget))
    }
    
    fn rebuild(&self, prev: &Self, element: Mut<'_, Pod<CanvasWidget>>, app_state: &AppData) {
        // Copy cheap view-only flags eagerly.
        // Reset render state only when geometry or depth changes.
    }
    
    fn message(&self, message: &mut MessageContext, app_state: &mut AppData) -> MessageResult<()> {
        match message.take_message::<CanvasAction>() {
            Some(CanvasAction::Geometry(geo)) => {
                app_state.geometry = geo;  // Commit from widget
                MessageResult::Action(())
            }
        }
    }
}
```

**Key insight**: The widget owns transient state; only commits to app state on explicit actions (drag end).

## Message Flow

```
┌──────────────────────────────────────────────────────────────┐
│ User drags on canvas                                        │
│                                                              │
│ ┌────────────────────────────────────────────────────────┐  │
│ │ on_pointer_event()                                     │  │
│ │ - hit_test() determines DragTarget                     │  │
│ │ - Captures pointer (ctx.capture_pointer())            │  │
│ │ - Stores drag_start_geometry for stable deltas        │  │
│ │ - Updates widget.geometry with apply_drag()           │  │
│ │ - ctx.request_anim_frame() for incremental render     │  │
│ └────────────────────────────────────────────────────────┘  │
│                                                              │
│ ┌────────────────────────────────────────────────────────┐  │
│ │ on_anim_frame() - incremental rendering              │  │
│ │ - Pops SegmentJobs from VecDeque                      │  │
│ │ - rasterize_line() into raster_buffer                │  │
│ │ - Budget: 5ms per frame to stay responsive           │  │
│ │ - ctx.request_paint_only() to refresh display         │  │
│ └────────────────────────────────────────────────────────┘  │
│                                                              │
│ ┌────────────────────────────────────────────────────────┐  │
│ │ paint()                                               │  │
│ │ - Fills background, draws preview panel                │  │
│ │ - Draws raster_buffer as ImageBrush                  │  │
│ │ - Overlays guide geometry (baseline, handles)         │  │
│ └────────────────────────────────────────────────────────┘  │
│                                                              │
│ ┌────────────────────────────────────────────────────────┐  │
│ │ PointerEvent::Up                                       │  │
│ │ - ctx.submit_action(CanvasAction::Geometry(...))      │  │
│ │ - Commits final geometry to Xilem                    │  │
│ └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

## Hybrid Rendering

The example demonstrates **two rendering approaches** combined:

### 1. Incremental Raster (CPU)
```rust
fn on_anim_frame(&mut self, ctx: &mut UpdateCtx, interval: u64) {
    let deadline = Instant::now() + RENDER_BATCH_BUDGET;
    while Instant::now() < deadline {
        let Some(job) = self.pending_segments.pop_back() else { break };
        rasterize_line(&mut self.raster_buffer, width, height, job.start, job.end, color);
    }
    ctx.request_paint_only();
}
```
- Fractal lines are rasterized directly into `raster_buffer`
- Time-budgeted (5ms) to keep UI responsive
- Work queue (`VecDeque<SegmentJob>`) for incremental processing

### 2. Immediate Mode Vello (GPU)
```rust
fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
    // Background
    scene.fill(Fill::NonZero, Affine::IDENTITY, Color::from_rgb8(246, 242, 233), ...);
    
    // Rasterized fractal as image
    let image = ImageBrush::new(ImageData {
        data: self.raster_buffer.clone().into(),
        format: ImageFormat::Rgba8,
        // ...
    });
    scene.draw_image(&image, Affine::IDENTITY);
    
    // Vector guides overlaid
    paint_baseline_line(scene, self.geometry.baseline());
    paint_baseline_handles(scene, &self.geometry, ...);
}
```
- Vello draws backgrounds and guides (GPU-accelerated)
- Raster buffer drawn as `ImageBrush` (bridging CPU/GPU)
- Natural layering: raster → guides → handles

## Hit Testing Strategy

The example implements a **layered hit test** with priority:

```rust
fn hit_test(&self, point: Point) -> Option<DragTarget> {
    // Priority 1: Nested endpoint editors (topmost layer)
    if let Some(handle) = self.revealed_endpoint_handle(base_index) {
        if distance(point, handle) <= NESTED_ENDPOINT_RADIUS + NESTED_HIT_TOLERANCE {
            return Some(DragTarget::EndpointEditor(base_index));
        }
    }
    
    // Priority 2: Baseline handles (outer ring)
    if dist <= BASELINE_RADIUS && !nested_claims_center {
        return Some(DragTarget::BasePoint(index));
    }
    
    // Priority 3: Generator points
    if distance(point, generator_point) <= HIT_RADIUS { ... }
    
    // Priority 4: Shape / line (lowest priority)
    if point_near_polyline(point, &self.geometry.generator_points, HIT_RADIUS) {
        return Some(DragTarget::GeneratorShape);
    }
}
```

**Key insight**: Z-order determines hit priority. Double-click toggles visibility of nested editors, allowing access to baseline handles beneath.

## Coordinate Systems

The example uses three coordinate spaces:

| System | Origin | Use |
|--------|--------|-----|
| **World** | Canvas top-left | Final pixel positions |
| **Normalized** | Baseline endpoints | Fractal recursion |
| **Local** | Each segment | Recursive subdivision |

```rust
// World → Normalized
fn normalized_points_from_baseline(points, baseline) -> Vec<(f64, f64)> {
    let local_x = offset.dot(line) / length_sq;   // Along baseline
    let local_y = cross(line, offset) / length_sq; // Perpendicular
    (local_x, local_y)
}

// Normalized → World  
fn map_local(start: Point, end: Point, local: (f64, f64)) -> Point {
    let line = end - start;
    start + line * local.0 + perpendicular(line) * local.1
}
```

## Key Masonry Concepts

### WidgetPod
Wraps the custom widget and manages:
- Lifecycle (register, add, remove)
- Pass coordination
- Action submission

### Context Types
| Context | Access | Use |
|---------|--------|-----|
| `EventCtx` | Mutable | Handle events, submit actions, request passes |
| `UpdateCtx` | Mutable | Respond to updates, request anim frame |
| `PaintCtx` | Read-only | Get size, access scene |
| `Mut<'_, Pod<T>>` | Mutable | Direct widget mutation in Xilem |

### Pass Requests
- `ctx.request_anim_frame()` - Schedule animation callback
- `ctx.request_paint_only()` - Request repaint without full layout
- `ctx.capture_pointer()` - Grab exclusive pointer events
- `ctx.submit_action::<T>(action)` - Send message to Xilem

## Pattern Summary

This example demonstrates:

1. **Custom widget + Vello**: Leaf widget doing its own rendering
2. **Hybrid rendering**: CPU raster + GPU vector composition
3. **View bridge**: Connecting Xilem views to Masonry widgets
4. **Bidirectional state**: Widget-local transient vs. app-level committed
5. **Incremental computation**: Time-budgeted rendering with work queues
6. **Event bubbling**: Pointer events flow up parent chain
7. **Layered hit testing**: Z-order determines interaction priority
