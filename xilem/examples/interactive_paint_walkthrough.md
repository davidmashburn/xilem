# `interactive_paint.rs` Annotated Walkthrough

This document is a dense companion to `interactive_paint.rs`. It walks the source file in order and explains what each highlighted block is doing. The explanations are intentionally literal so you can trace the file top to bottom without having to guess how the pieces fit together.

How to read this:
- Each section covers a contiguous line range from the source file.
- The numbered code blocks quote the local source snapshot for the section being discussed.
- The note list directly under each code block explains those same line numbers in order.

## Lines 1-25: File Header And Imports

```rust
   1 | // Copyright 2024 the Xilem Authors
   2 | // SPDX-License-Identifier: Apache-2.0
   3 | 
   4 | //! Interactive line fractal explorer built with a custom paint widget.
   5 | 
   6 | use std::collections::VecDeque;
   7 | use std::time::{Duration, Instant};
   8 | 
   9 | use masonry::core::*;
  10 | use masonry::dpi::LogicalSize;
  11 | use masonry::peniko::{Fill, ImageBrush, ImageFormat};
  12 | use masonry::properties::types::{AsUnit, CrossAxisAlignment};
  13 | use masonry::util::fill;
  14 | use masonry::vello::kurbo::{Affine, BezPath, Circle, Line, Point, Rect, Size, Stroke, Vec2};
  15 | use masonry::vello::Scene;
  16 | use masonry_winit::app::{EventLoop, EventLoopBuilder};
  17 | use vello::peniko::{ImageAlphaType, ImageData};
  18 | use winit::error::EventLoopError;
  19 | use xilem::core::{Arg, MessageContext, Mut, View, ViewMarker};
  20 | use xilem::style::Style;
  21 | use xilem::view::{checkbox, flex_col, flex_row, label, sized_box, text_button, text_input};
  22 | use xilem::{Color, Pod, TextAlign, ViewCtx, WidgetView, WindowOptions, Xilem};
  23 | use xilem_core::{Edit, MessageResult};
  24 | 
  25 | const CANVAS_WIDTH: f64 = 920.0;
```

- Line 1: Copyright notice for the example source file.
- Line 2: License identifier declaring the file is shipped under Apache 2.0.
- Line 3: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 4: Crate-level documentation comment summarizing the purpose of this example.
- Line 5: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 6: Pulls in the double-ended queue used as the incremental render work queue.
- Line 7: Imports time utilities used for frame budgeting and double-click detection.
- Line 8: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 9: Wildcard import of Masonry core widget traits and context types used throughout the custom widget.
- Line 10: Imports logical window sizing for the example window configuration.
- Line 11: Imports image and fill helpers needed to paint the backing bitmap into the scene.
- Line 12: Imports Xilem layout units and cross-axis alignment helpers for the control panel UI.
- Line 13: Imports Masonry’s convenience fill helper used to paint circles and background rectangles.
- Line 14: Imports the geometry types used for paths, circles, lines, points, transforms, and vectors.
- Line 15: Brings in the Vello scene type that receives all drawing commands during paint.
- Line 16: Imports event loop types used to launch the desktop app.
- Line 17: Imports image metadata types used to wrap the raster backing buffer as a drawable image.
- Line 18: Imports the error type returned when building or running the window event loop.
- Line 19: Imports the Xilem core traits and helper types that connect app state to the custom canvas view.
- Line 20: Imports styling support, even though the example mostly relies on direct builder methods.
- Line 21: Imports the stock controls used to build the buttons, labels, checkbox, and text field.
- Line 22: Imports higher-level Xilem UI types, colors, and the top-level application wrapper.
- Line 23: Imports edit-state plumbing and the message result type used by the custom view bridge.
- Line 24: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 25: Defines the fixed canvas width used both for layout and for the raster backing buffer.

## Lines 26-50: Canvas Constants

```rust
  26 | const CANVAS_HEIGHT: f64 = 620.0;
  27 | const HANDLE_RADIUS: f64 = 8.0;
  28 | const BASELINE_RADIUS: f64 = HANDLE_RADIUS + 1.5;
  29 | const NESTED_ENDPOINT_RADIUS: f64 = 4.5;
  30 | const NESTED_HIT_TOLERANCE: f64 = 3.0;
  31 | const HIT_RADIUS: f64 = 14.0;
  32 | const MIN_SEGMENT_LENGTH: f64 = 2.5;
  33 | const RENDER_BATCH_BUDGET: Duration = Duration::from_millis(5);
  34 | const DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(300);
  35 | 
  36 | #[derive(Clone, Debug)]
  37 | struct FractalGeometry {
  38 |     generator_points: Vec<(f64, f64)>,
  39 |     baseline_points: [(f64, f64); 2],
  40 |     endpoint_docked: [bool; 2],
  41 | }
  42 | 
  43 | impl FractalGeometry {
  44 |     fn koch() -> Self {
  45 |         Self {
  46 |             generator_points: vec![
  47 |                 (160.0, 150.0),
  48 |                 (260.0, 150.0),
  49 |                 (310.0, 85.0),
  50 |                 (360.0, 150.0),
```

- Line 26: Defines the fixed canvas height used both for layout and for the raster backing buffer.
- Line 27: Defines the radius for normal red generator handles.
- Line 28: Defines the larger radius used for blue baseline handles so they stay easy to hit.
- Line 29: Defines the inner radius for the nested active dot inside a docked endpoint control.
- Line 30: Defines the extra tolerance used when picking the smaller nested endpoint editor.
- Line 31: Defines the looser picking radius used for hit-testing around shapes and handles.
- Line 32: Defines the segment-size cutoff where recursive subdivision stops and the line is rasterized directly.
- Line 33: Sets the per-frame time budget for incremental raster work.
- Line 34: Defines the timing window used to interpret two clicks as a double-click.
- Line 35: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 36: Derives common trait implementations so geometry can be cloned into the widget and printed during debugging.
- Line 37: Starts the geometry struct that holds the current generator polyline, baseline, and docking flags.
- Line 38: Stores the editable red control polyline that defines the fractal generator.
- Line 39: Stores the blue baseline endpoints that define how the generator is mapped into recursive segments.
- Line 40: Stores whether each red endpoint is currently locked onto its matching blue baseline point.
- Line 41: Closes the `FractalGeometry` struct definition.
- Line 42: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 43: Begins helper constructors and accessors for `FractalGeometry`.
- Line 44: Defines the Koch-inspired preset geometry.
- Line 45: Returns a new geometry value built inline.
- Line 46: Stores the editable red control polyline that defines the fractal generator.
- Line 47: Adds one control point to the preset generator polyline.
- Line 48: Adds one control point to the preset generator polyline.
- Line 49: Adds one control point to the preset generator polyline.
- Line 50: Adds one control point to the preset generator polyline.

## Lines 51-75: Fractal Geometry Model

```rust
  51 |             ],
  52 |             baseline_points: [(160.0, 150.0), (460.0, 150.0)],
  53 |             endpoint_docked: [true, true],
  54 |         }
  55 |     }
  56 | 
  57 |     fn lightning() -> Self {
  58 |         Self {
  59 |             generator_points: vec![
  60 |                 (170.0, 150.0),
  61 |                 (235.0, 125.0),
  62 |                 (285.0, 190.0),
  63 |                 (350.0, 110.0),
  64 |                 (415.0, 165.0),
  65 |                 (480.0, 145.0),
  66 |             ],
  67 |             baseline_points: [(170.0, 150.0), (480.0, 145.0)],
  68 |             endpoint_docked: [true, true],
  69 |         }
  70 |     }
  71 | 
  72 |     fn canyon() -> Self {
  73 |         Self {
  74 |             generator_points: vec![
  75 |                 (170.0, 155.0),
```

- Line 51: Closes the generator point list for this preset.
- Line 52: Stores the blue baseline endpoints that define how the generator is mapped into recursive segments.
- Line 53: Stores whether each red endpoint is currently locked onto its matching blue baseline point.
- Line 54: Closes the inline struct literal for the preset geometry.
- Line 55: This line closes the current block or scope.
- Line 56: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 57: Defines the jagged lightning-style preset geometry.
- Line 58: Returns a new geometry value built inline.
- Line 59: Stores the editable red control polyline that defines the fractal generator.
- Line 60: Adds one control point to the preset generator polyline.
- Line 61: Adds one control point to the preset generator polyline.
- Line 62: Adds one control point to the preset generator polyline.
- Line 63: Adds one control point to the preset generator polyline.
- Line 64: Adds one control point to the preset generator polyline.
- Line 65: Adds one control point to the preset generator polyline.
- Line 66: Closes the generator point list for this preset.
- Line 67: Stores the blue baseline endpoints that define how the generator is mapped into recursive segments.
- Line 68: Stores whether each red endpoint is currently locked onto its matching blue baseline point.
- Line 69: Closes the inline struct literal for the preset geometry.
- Line 70: This line closes the current block or scope.
- Line 71: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 72: Defines the wider canyon-style preset geometry.
- Line 73: Returns a new geometry value built inline.
- Line 74: Stores the editable red control polyline that defines the fractal generator.
- Line 75: Adds one control point to the preset generator polyline.

## Lines 76-100: Fractal Geometry Model

```rust
  76 |                 (245.0, 155.0),
  77 |                 (285.0, 215.0),
  78 |                 (350.0, 95.0),
  79 |                 (415.0, 215.0),
  80 |                 (455.0, 155.0),
  81 |                 (530.0, 155.0),
  82 |             ],
  83 |             baseline_points: [(170.0, 155.0), (530.0, 155.0)],
  84 |             endpoint_docked: [true, true],
  85 |         }
  86 |     }
  87 | 
  88 |     fn baseline(&self) -> [(f64, f64); 2] {
  89 |         self.baseline_points
  90 |     }
  91 | }
  92 | 
  93 | #[derive(Debug)]
  94 | struct InteractivePaintApp {
  95 |     depth: usize,
  96 |     depth_input: String,
  97 |     show_guides: bool,
  98 |     geometry: FractalGeometry,
  99 |     preset_name: &'static str,
 100 | }
```

- Line 76: Adds one control point to the preset generator polyline.
- Line 77: Adds one control point to the preset generator polyline.
- Line 78: Adds one control point to the preset generator polyline.
- Line 79: Adds one control point to the preset generator polyline.
- Line 80: Adds one control point to the preset generator polyline.
- Line 81: Adds one control point to the preset generator polyline.
- Line 82: Closes the generator point list for this preset.
- Line 83: Stores the blue baseline endpoints that define how the generator is mapped into recursive segments.
- Line 84: Stores whether each red endpoint is currently locked onto its matching blue baseline point.
- Line 85: Closes the inline struct literal for the preset geometry.
- Line 86: This line closes the current block or scope.
- Line 87: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 88: Provides a short accessor that returns the current blue baseline endpoints.
- Line 89: Returns the stored baseline array directly.
- Line 90: This line closes the current block or scope.
- Line 91: This line closes the current block or scope.
- Line 92: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 93: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 94: Starts the top-level app state struct stored by Xilem.
- Line 95: Stores the recursion depth currently requested by the user.
- Line 96: Stores the raw text field contents so partially typed numeric input can exist temporarily.
- Line 97: Tracks whether the guide scaffold and handles should be drawn.
- Line 98: Holds the persistent fractal geometry edited through the canvas.
- Line 99: Stores the label of the active preset so reset can rebuild the right default shape.
- Line 100: This line closes the current block or scope.

## Lines 101-125: Top-Level App State

```rust
 101 | 
 102 | impl Default for InteractivePaintApp {
 103 |     fn default() -> Self {
 104 |         Self {
 105 |             depth: 5,
 106 |             depth_input: "5".to_string(),
 107 |             show_guides: true,
 108 |             geometry: FractalGeometry::koch(),
 109 |             preset_name: "Koch-ish",
 110 |         }
 111 |     }
 112 | }
 113 | 
 114 | impl InteractivePaintApp {
 115 |     fn set_preset(&mut self, preset_name: &'static str, geometry: FractalGeometry) {
 116 |         self.preset_name = preset_name;
 117 |         self.geometry = geometry;
 118 |     }
 119 | 
 120 |     fn set_depth(&mut self, depth: usize) {
 121 |         self.depth = depth;
 122 |         self.depth_input = self.depth.to_string();
 123 |     }
 124 | 
 125 |     fn branch_factor(&self) -> usize {
```

- Line 101: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 102: Begins the default app-state implementation used at startup.
- Line 103: Constructs the default line fractal explorer state.
- Line 104: Returns a new geometry value built inline.
- Line 105: Sets the initial recursion depth.
- Line 106: Initializes the text input to match the starting depth.
- Line 107: Starts with the guide overlay visible.
- Line 108: Holds the persistent fractal geometry edited through the canvas.
- Line 109: Stores the label of the active preset so reset can rebuild the right default shape.
- Line 110: This line closes the current block or scope.
- Line 111: This line closes the current block or scope.
- Line 112: This line closes the current block or scope.
- Line 113: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 114: Begins methods for updating or deriving top-level app state.
- Line 115: Declares a helper that swaps both the preset label and the underlying geometry together.
- Line 116: Updates the stored preset name.
- Line 117: Replaces the current geometry with the preset geometry.
- Line 118: This line closes the current block or scope.
- Line 119: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 120: Declares a helper that updates the numeric depth and keeps the text field in sync.
- Line 121: Stores the new recursion depth.
- Line 122: Normalizes the text field back to the canonical string form of the depth.
- Line 123: This line closes the current block or scope.
- Line 124: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 125: Declares a helper that computes how many child segments each generator expansion produces.

## Lines 126-150: Top-Level App State

```rust
 126 |         self.geometry.generator_points.len().saturating_sub(1)
 127 |     }
 128 | 
 129 |     fn estimated_segments_label(&self) -> String {
 130 |         let branch_factor = self.branch_factor().max(1);
 131 |         let mut total = 1u128;
 132 |         for _ in 0..self.depth.min(64) {
 133 |             total = total.saturating_mul(branch_factor as u128);
 134 |         }
 135 |         if self.depth > 64 || total > 999_999_999_999 {
 136 |             "Segments: huge".to_string()
 137 |         } else {
 138 |             format!("Segments: {total}")
 139 |         }
 140 |     }
 141 | }
 142 | 
 143 | #[derive(Clone, Debug)]
 144 | enum CanvasAction {
 145 |     Geometry(FractalGeometry),
 146 | }
 147 | 
 148 | #[derive(Clone, Copy, Debug)]
 149 | struct SegmentJob {
 150 |     start: Point,
```

- Line 126: Returns the number of polyline segments in the generator, clamped so it never underflows.
- Line 127: This line closes the current block or scope.
- Line 128: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 129: Declares a helper that estimates how many leaf segments the current depth implies.
- Line 130: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 131: Starts with one segment before any recursive expansion.
- Line 132: Iterates once per depth level, but caps the estimate loop to avoid absurd work.
- Line 133: Multiplies by the branch factor using saturating arithmetic so the estimate never overflows.
- Line 134: This line closes the current block or scope.
- Line 135: If the estimate would be too large to be meaningful, collapse it to a human-readable warning label.
- Line 136: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 137: This line opens a new block or nested scope.
- Line 138: Returns a concrete segment-count label when the estimate stays tractable.
- Line 139: This line closes the current block or scope.
- Line 140: This line closes the current block or scope.
- Line 141: This line closes the current block or scope.
- Line 142: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 143: Derives common trait implementations so geometry can be cloned into the widget and printed during debugging.
- Line 144: Defines the message type sent from the custom canvas widget back into app state.
- Line 145: Carries a full geometry replacement when the widget wants to commit edits back to the app.
- Line 146: This line closes the current block or scope.
- Line 147: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 148: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 149: Starts the small work item struct used by incremental rendering.
- Line 150: Stores the world-space start point of a pending recursive segment.

## Lines 151-175: Canvas Messages And Drag State

```rust
 151 |     end: Point,
 152 |     depth: usize,
 153 | }
 154 | 
 155 | #[derive(Clone, Debug)]
 156 | enum DragTarget {
 157 |     EndpointEditor(usize),
 158 |     GeneratorPoint(usize),
 159 |     BasePoint(usize),
 160 |     GeneratorShape,
 161 |     Baseline,
 162 | }
 163 | 
 164 | struct CanvasWidget {
 165 |     geometry: FractalGeometry,
 166 |     depth: usize,
 167 |     show_guides: bool,
 168 |     drag_target: Option<DragTarget>,
 169 |     drag_anchor: Option<Point>,
 170 |     drag_start_geometry: Option<FractalGeometry>,
 171 |     endpoint_revealed: [bool; 2],
 172 |     last_click: Option<(usize, Instant, Point)>,
 173 |     pending_segments: VecDeque<SegmentJob>,
 174 |     raster_buffer: Vec<u8>,
 175 | }
```

- Line 151: Stores the world-space end point of a pending recursive segment.
- Line 152: Stores the recursion depth currently requested by the user.
- Line 153: This line closes the current block or scope.
- Line 154: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 155: Derives common trait implementations so geometry can be cloned into the widget and printed during debugging.
- Line 156: Starts the enum describing every draggable thing on the canvas.
- Line 157: Represents the nested red endpoint editor inside a docked control.
- Line 158: Represents a normal red generator control point.
- Line 159: Represents a blue baseline endpoint.
- Line 160: Represents dragging the red scaffold as a whole.
- Line 161: Represents dragging the blue baseline segment as a whole.
- Line 162: This line closes the current block or scope.
- Line 163: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 164: Starts the custom widget that owns local interaction state and the backing raster.
- Line 165: Holds the persistent fractal geometry edited through the canvas.
- Line 166: Stores the recursion depth currently requested by the user.
- Line 167: Tracks whether the guide scaffold and handles should be drawn.
- Line 168: Tracks what the user is currently dragging, if anything.
- Line 169: Stores the pointer position where the current drag began.
- Line 170: Stores the geometry snapshot from drag start so each move can be computed from a stable base.
- Line 171: Tracks whether each docked endpoint’s inner red editor dot is currently visible.
- Line 172: Imports time utilities used for frame budgeting and double-click detection.
- Line 173: Pulls in the double-ended queue used as the incremental render work queue.
- Line 174: Persistent RGBA backing buffer that accumulates finished line pixels.
- Line 175: This line closes the current block or scope.

## Lines 176-200: Canvas Widget State

```rust
 176 | 
 177 | impl Widget for CanvasWidget {
 178 |     type Action = CanvasAction;
 179 | 
 180 |     fn on_anim_frame(
 181 |         &mut self,
 182 |         ctx: &mut UpdateCtx<'_>,
 183 |         _: &mut PropertiesMut<'_>,
 184 |         _interval: u64,
 185 |     ) {
 186 |         if self.pending_segments.is_empty() {
 187 |             return;
 188 |         }
 189 | 
 190 |         let local_points = normalized_points_from_baseline(
 191 |             &self.geometry.generator_points,
 192 |             self.geometry.baseline(),
 193 |         );
 194 |         let deadline = Instant::now() + RENDER_BATCH_BUDGET;
 195 |         while Instant::now() < deadline {
 196 |             let Some(job) = self.pending_segments.pop_back() else {
 197 |                 break;
 198 |             };
 199 |             if job.depth == 0
 200 |                 || distance(job.start, job.end) <= MIN_SEGMENT_LENGTH
```

- Line 176: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 177: Begins the Masonry widget implementation for the canvas.
- Line 178: Declares the action type the widget can emit upward.
- Line 179: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 180: Runs once per animation frame and advances the incremental fractal renderer for a bounded slice of time.
- Line 181: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 182: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 183: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 184: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 185: This line opens a new block or nested scope.
- Line 186: If there is no pending recursive work, this animation-frame callback can return immediately.
- Line 187: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 188: This line closes the current block or scope.
- Line 189: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 190: Begins normalizing the current generator into coordinates relative to the active blue baseline.
- Line 191: Passes the current red generator points into that normalization helper.
- Line 192: Uses the current baseline rather than assuming the red endpoints define the frame.
- Line 193: Finishes the helper call and stores the resulting local coordinates.
- Line 194: Computes the deadline for this incremental batch of raster work.
- Line 195: Continues processing queued segment jobs until the time budget is exhausted.
- Line 196: Pops one pending segment off the back of the deque for depth-first style incremental processing.
- Line 197: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 198: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 199: If the segment is ready to finish, too small to subdivide further, or the generator is invalid, rasterize it directly.
- Line 200: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.

## Lines 201-225: Canvas Widget Implementation

```rust
 201 |                     &mut self.raster_buffer,
 202 |                     CANVAS_WIDTH as usize,
 203 |                     CANVAS_HEIGHT as usize,
 204 |                     job.start,
 205 |                     job.end,
 206 |                     [28, 96, 99, 255],
 207 |                 );
 208 |                 continue;
 209 |             }
 210 |             for pair in local_points.windows(2).rev() {
 211 |                 self.pending_segments.push_back(SegmentJob {
 212 |                     start: map_local(job.start, job.end, pair[0]),
 213 |                     end: map_local(job.start, job.end, pair[1]),
 214 |                     depth: job.depth - 1,
 215 |                 });
 216 |             }
 217 |         }
 218 | 
 219 |         if !self.pending_segments.is_empty() {
 220 |             ctx.request_anim_frame();
 221 |         }
 222 |         ctx.request_paint_only();
 223 |     }
 224 | 
 225 |     fn register_children(&mut self, _: &mut RegisterCtx<'_>) {}
```

- Line 201: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 202: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 203: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 204: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 205: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 206: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 207: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 208: Skips recursive expansion once the current segment has been rasterized.
- Line 209: This line closes the current block or scope.
- Line 210: Walks through each generator segment in reverse so the queue preserves intuitive expansion order when popping from the back.
- Line 211: Pushes a newly mapped child segment job onto the pending queue.
- Line 212: Maps the local start point of the generator segment into the current world-space parent segment.
- Line 213: Maps the local end point of the generator segment into the current world-space parent segment.
- Line 214: Decrements the remaining recursion depth for each child segment.
- Line 215: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 216: This line closes the current block or scope.
- Line 217: This line closes the current block or scope.
- Line 218: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 219: If there is still work left after this frame’s budget, schedule another animation frame.
- Line 220: Schedules another animation frame so the incremental renderer resumes immediately after this drag update.
- Line 221: This line closes the current block or scope.
- Line 222: Requests a repaint so the newly rasterized pixels become visible.
- Line 223: This line closes the current block or scope.
- Line 224: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 225: This widget has no child widgets to register.

## Lines 226-250: Canvas Widget Implementation

```rust
 226 | 
 227 |     fn accessibility_role(&self) -> masonry::accesskit::Role {
 228 |         masonry::accesskit::Role::Canvas
 229 |     }
 230 | 
 231 |     fn accessibility(
 232 |         &mut self,
 233 |         _: &mut AccessCtx<'_>,
 234 |         _: &PropertiesRef<'_>,
 235 |         node: &mut masonry::accesskit::Node,
 236 |     ) {
 237 |         node.set_label(
 238 |             "Interactive line fractal editor with draggable generator points and baseline.",
 239 |         );
 240 |     }
 241 | 
 242 |     fn children_ids(&self) -> ChildrenIds {
 243 |         ChildrenIds::new()
 244 |     }
 245 | 
 246 |     fn layout(
 247 |         &mut self,
 248 |         _: &mut LayoutCtx<'_>,
 249 |         _: &mut PropertiesMut<'_>,
 250 |         bc: &BoxConstraints,
```

- Line 226: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 227: Reports the accessibility role as a canvas-like region.
- Line 228: Returns the accessibility role value used by assistive technology.
- Line 229: This line closes the current block or scope.
- Line 230: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 231: Provides an accessibility label describing what the canvas lets the user edit.
- Line 232: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 233: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 234: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 235: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 236: This line opens a new block or nested scope.
- Line 237: Sets the descriptive accessibility label for the canvas node.
- Line 238: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 239: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 240: This line closes the current block or scope.
- Line 241: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 242: Returns an empty child list because the canvas is a leaf widget.
- Line 243: Creates an empty `ChildrenIds` collection.
- Line 244: This line closes the current block or scope.
- Line 245: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 246: Implements layout by constraining the widget to the fixed canvas dimensions.
- Line 247: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 248: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 249: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 250: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.

## Lines 251-275: Canvas Widget Implementation

```rust
 251 |     ) -> Size {
 252 |         bc.constrain((CANVAS_WIDTH, CANVAS_HEIGHT))
 253 |     }
 254 | 
 255 |     fn on_pointer_event(
 256 |         &mut self,
 257 |         ctx: &mut EventCtx<'_>,
 258 |         _: &mut PropertiesMut<'_>,
 259 |         event: &PointerEvent,
 260 |     ) {
 261 |         match event {
 262 |             PointerEvent::Down(e) => {
 263 |                 let local_pos = ctx.local_position(e.state.position);
 264 |                 if let Some(base_index) = self.baseline_hit(local_pos) {
 265 |                     let now = Instant::now();
 266 |                     if self.geometry.endpoint_docked[base_index]
 267 |                         && matches!(
 268 |                             self.last_click,
 269 |                             Some((last_index, last_time, last_pos))
 270 |                                 if last_index == base_index
 271 |                                     && now.duration_since(last_time) <= DOUBLE_CLICK_THRESHOLD
 272 |                                     && distance(local_pos, last_pos) <= HIT_RADIUS
 273 |                         )
 274 |                     {
 275 |                         self.endpoint_revealed[base_index] = !self.endpoint_revealed[base_index];
```

- Line 251: This line opens a new block or nested scope.
- Line 252: Returns the canvas size after applying parent box constraints.
- Line 253: This line closes the current block or scope.
- Line 254: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 255: Begins low-level pointer event handling for press, drag, release, and cancel.
- Line 256: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 257: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 258: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 259: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 260: This line opens a new block or nested scope.
- Line 261: Dispatches pointer handling based on the specific pointer event variant.
- Line 262: Handles the start of a pointer interaction.
- Line 263: Converts the global pointer position into local canvas coordinates.
- Line 264: Checks whether the press landed on a baseline handle, which matters for double-click toggling.
- Line 265: Imports time utilities used for frame budgeting and double-click detection.
- Line 266: Only docked endpoints participate in the reveal/hide double-click gesture.
- Line 267: Matches the last click against the same handle, within time and distance thresholds, to detect a double-click.
- Line 268: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 269: Matches the last click against the same handle, within time and distance thresholds, to detect a double-click.
- Line 270: Matches the last click against the same handle, within time and distance thresholds, to detect a double-click.
- Line 271: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 272: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 273: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 274: This line opens a new block or nested scope.
- Line 275: Toggles whether the nested red editor dot is visible for this docked endpoint.

## Lines 276-300: Canvas Widget Implementation

```rust
 276 |                         self.last_click = None;
 277 |                         ctx.request_paint_only();
 278 |                         return;
 279 |                     }
 280 |                     self.last_click = Some((base_index, now, local_pos));
 281 |                 } else {
 282 |                     self.last_click = None;
 283 |                 }
 284 |                 if let Some(target) = self.hit_test(local_pos) {
 285 |                     ctx.capture_pointer();
 286 |                     self.drag_target = Some(target);
 287 |                     self.drag_anchor = Some(local_pos);
 288 |                     self.drag_start_geometry = Some(self.geometry.clone());
 289 |                 }
 290 |             }
 291 |             PointerEvent::Move(e) => {
 292 |                 if !ctx.is_active() {
 293 |                     return;
 294 |                 }
 295 |                 if let (Some(target), Some(anchor), Some(start_geometry)) = (
 296 |                     self.drag_target.as_ref(),
 297 |                     self.drag_anchor,
 298 |                     self.drag_start_geometry.as_ref(),
 299 |                 ) {
 300 |                     let current = ctx.local_position(e.current.position);
```

- Line 276: Clears double-click state after a successful toggle so a third click starts fresh.
- Line 277: Requests a repaint so the newly rasterized pixels become visible.
- Line 278: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 279: This line closes the current block or scope.
- Line 280: Stores this click as the candidate first click for a future double-click.
- Line 281: This line opens a new block or nested scope.
- Line 282: Clears double-click state after a successful toggle so a third click starts fresh.
- Line 283: This line closes the current block or scope.
- Line 284: If the click hit any draggable target, start an active drag sequence.
- Line 285: Captures the pointer so subsequent drag events keep coming to this widget.
- Line 286: Remembers exactly what kind of thing is being dragged.
- Line 287: Stores the drag anchor point so later movement can be turned into a delta.
- Line 288: Clones the starting geometry so every drag update can be derived from the original pose rather than accumulating error.
- Line 289: This line closes the current block or scope.
- Line 290: This line closes the current block or scope.
- Line 291: Handles pointer movement during an active drag.
- Line 292: Ignores move events unless this widget currently owns an active pointer drag.
- Line 293: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 294: This line closes the current block or scope.
- Line 295: Only proceeds when all drag bookkeeping values are available.
- Line 296: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 297: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 298: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 299: This line opens a new block or nested scope.
- Line 300: Reads the current local pointer position for this move event.

## Lines 301-325: Canvas Widget Implementation

```rust
 301 |                     let delta = current - anchor;
 302 |                     self.geometry = apply_drag(start_geometry, target, delta);
 303 |                     self.reset_render_progress();
 304 |                     ctx.request_anim_frame();
 305 |                     ctx.request_paint_only();
 306 |                 }
 307 |             }
 308 |             PointerEvent::Up(_) => {
 309 |                 if ctx.is_active() {
 310 |                     ctx.release_pointer();
 311 |                 }
 312 |                 if let Some(start_geometry) = self.drag_start_geometry.as_ref() {
 313 |                     if self.geometry.generator_points != start_geometry.generator_points {
 314 |                         ctx.submit_action::<CanvasAction>(CanvasAction::Geometry(
 315 |                             self.geometry.clone(),
 316 |                         ));
 317 |                     }
 318 |                 }
 319 |                 self.drag_target = None;
 320 |                 self.drag_anchor = None;
 321 |                 self.drag_start_geometry = None;
 322 |                 ctx.request_paint_only();
 323 |             }
 324 |             PointerEvent::Cancel(_) => {
 325 |                 if ctx.is_active() {
```

- Line 301: Computes the drag delta relative to where the drag started.
- Line 302: Applies the drag to the starting geometry to produce a new transient geometry.
- Line 303: Clears the backing buffer and rebuilds the render queue because the visible fractal changed.
- Line 304: Schedules another animation frame so the incremental renderer resumes immediately after this drag update.
- Line 305: Requests a repaint so the newly rasterized pixels become visible.
- Line 306: This line closes the current block or scope.
- Line 307: This line closes the current block or scope.
- Line 308: Handles pointer release and commits the final geometry back to app state if it actually changed.
- Line 309: Releases the captured pointer if this widget still owns it.
- Line 310: Releases the captured pointer if this widget still owns it.
- Line 311: This line closes the current block or scope.
- Line 312: Compares the final geometry against the drag-start geometry before emitting an action.
- Line 313: Only commit if the generator polyline changed; this avoids redundant app-state traffic.
- Line 314: Submits the final geometry back to the Xilem view layer.
- Line 315: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 316: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 317: This line closes the current block or scope.
- Line 318: This line closes the current block or scope.
- Line 319: Clears the current drag target at the end of the interaction.
- Line 320: Clears the stored drag anchor because no drag is active anymore.
- Line 321: Clears the drag-start snapshot because the drag session is over.
- Line 322: Requests a repaint so the newly rasterized pixels become visible.
- Line 323: This line closes the current block or scope.
- Line 324: Handles canceled pointer interactions by discarding drag bookkeeping without committing changes.
- Line 325: Releases the captured pointer if this widget still owns it.

## Lines 326-350: Canvas Widget Implementation

```rust
 326 |                     ctx.release_pointer();
 327 |                 }
 328 |                 self.drag_target = None;
 329 |                 self.drag_anchor = None;
 330 |                 self.drag_start_geometry = None;
 331 |                 ctx.request_paint_only();
 332 |             }
 333 |             _ => {}
 334 |         }
 335 |     }
 336 | 
 337 |     fn update(&mut self, ctx: &mut UpdateCtx<'_>, _: &mut PropertiesMut<'_>, event: &Update) {
 338 |         if matches!(event, Update::WidgetAdded) {
 339 |             self.reset_render_progress();
 340 |             ctx.request_anim_frame();
 341 |             ctx.request_paint_only();
 342 |         }
 343 |     }
 344 | 
 345 |     fn paint(&mut self, ctx: &mut PaintCtx<'_>, _: &PropertiesRef<'_>, scene: &mut Scene) {
 346 |         let rect = ctx.size().to_rect();
 347 |         fill(scene, &rect, Color::from_rgb8(246, 242, 233));
 348 | 
 349 |         let preview_rect = Rect::from_origin_size((24.0, 24.0), (872.0, 560.0));
 350 |         scene.fill(
```

- Line 326: Releases the captured pointer if this widget still owns it.
- Line 327: This line closes the current block or scope.
- Line 328: Clears the current drag target at the end of the interaction.
- Line 329: Clears the stored drag anchor because no drag is active anymore.
- Line 330: Clears the drag-start snapshot because the drag session is over.
- Line 331: Requests a repaint so the newly rasterized pixels become visible.
- Line 332: This line closes the current block or scope.
- Line 333: Ignores any pointer events not explicitly handled above.
- Line 334: This line closes the current block or scope.
- Line 335: This line closes the current block or scope.
- Line 336: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 337: Responds to widget lifecycle updates from Masonry.
- Line 338: When the widget is first added, clear and seed the renderer so the fractal starts drawing.
- Line 339: Clears the backing buffer and rebuilds the render queue because the visible fractal changed.
- Line 340: Schedules another animation frame so the incremental renderer resumes immediately after this drag update.
- Line 341: Requests a repaint so the newly rasterized pixels become visible.
- Line 342: This line closes the current block or scope.
- Line 343: This line closes the current block or scope.
- Line 344: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 345: Begins custom painting of the canvas and its overlay controls.
- Line 346: Captures the full canvas bounds as a rectangle.
- Line 347: Paints the warm background behind the drawing area.
- Line 348: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 349: Defines the inner preview panel that visually frames the drawing region.
- Line 350: Fills the preview panel with a translucent white surface.

## Lines 351-375: Canvas Widget Implementation

```rust
 351 |             Fill::NonZero,
 352 |             Affine::IDENTITY,
 353 |             Color::from_rgba8(255, 255, 255, 235),
 354 |             None,
 355 |             &preview_rect,
 356 |         );
 357 |         scene.stroke(
 358 |             &Stroke::new(1.0),
 359 |             Affine::IDENTITY,
 360 |             Color::from_rgb8(204, 188, 160),
 361 |             None,
 362 |             &preview_rect,
 363 |         );
 364 | 
 365 |         let image = ImageBrush::new(ImageData {
 366 |             data: self.raster_buffer.clone().into(),
 367 |             format: ImageFormat::Rgba8,
 368 |             alpha_type: ImageAlphaType::Alpha,
 369 |             width: CANVAS_WIDTH as u32,
 370 |             height: CANVAS_HEIGHT as u32,
 371 |         });
 372 |         scene.draw_image(&image, Affine::IDENTITY);
 373 | 
 374 |         paint_baseline_line(scene, self.geometry.baseline());
 375 |         if self.show_guides {
```

- Line 351: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 352: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 353: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 354: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 355: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 356: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 357: Strokes the preview panel border.
- Line 358: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 359: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 360: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 361: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 362: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 363: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 364: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 365: Wraps the raw RGBA backing buffer as an image brush.
- Line 366: Clones the raster buffer into image data for this paint pass.
- Line 367: Declares the backing buffer pixel format.
- Line 368: Declares that the image data carries alpha values.
- Line 369: Supplies the image width expected by the renderer.
- Line 370: Supplies the image height expected by the renderer.
- Line 371: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 372: Draws the raster backing image into the scene.
- Line 373: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 374: Paints the blue baseline line over the fractal image.
- Line 375: Only paints guide geometry and handles when the user has the guide overlay enabled.

## Lines 376-400: Canvas Widget Implementation

```rust
 376 |             paint_generator_guides(scene, &self.geometry, self.endpoint_revealed);
 377 |         }
 378 |         paint_baseline_handles(scene, &self.geometry, self.endpoint_revealed);
 379 |     }
 380 | }
 381 | 
 382 | impl CanvasWidget {
 383 |     fn reset_render_progress(&mut self) {
 384 |         self.raster_buffer.fill(0);
 385 |         self.pending_segments.clear();
 386 |         if self.geometry.generator_points.len() >= 2 {
 387 |             let baseline = self.geometry.baseline();
 388 |             self.pending_segments.push_back(SegmentJob {
 389 |                 start: baseline[0].into(),
 390 |                 end: baseline[1].into(),
 391 |                 depth: self.depth,
 392 |             });
 393 |         }
 394 |     }
 395 | 
 396 |     fn hit_test(&self, point: Point) -> Option<DragTarget> {
 397 |         for base_index in 0..2 {
 398 |             if let Some(handle) = self.revealed_endpoint_handle(base_index) {
 399 |                 if distance(point, handle) <= NESTED_ENDPOINT_RADIUS + NESTED_HIT_TOLERANCE {
 400 |                     return Some(DragTarget::EndpointEditor(base_index));
```

- Line 376: Paints the red generator scaffold and its handles.
- Line 377: This line closes the current block or scope.
- Line 378: Paints the blue baseline handles on top of everything else.
- Line 379: This line closes the current block or scope.
- Line 380: This line closes the current block or scope.
- Line 381: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 382: Begins helper methods that belong only to the custom canvas widget.
- Line 383: Clears any previous rasterization and reseeds the render queue from the current baseline and depth.
- Line 384: Erases the backing buffer to transparent black so a fresh render can start.
- Line 385: Drops any old pending jobs from the previous render.
- Line 386: Only seeds the render queue if the generator has enough points to define at least one segment.
- Line 387: Reads the current baseline once so the seed job uses a consistent pair of endpoints.
- Line 388: Pushes a newly mapped child segment job onto the pending queue.
- Line 389: Sets the seed job start point to the first baseline endpoint.
- Line 390: Sets the seed job end point to the second baseline endpoint.
- Line 391: Seeds the queue with the full requested recursion depth.
- Line 392: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 393: This line closes the current block or scope.
- Line 394: This line closes the current block or scope.
- Line 395: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 396: Begins hit-testing logic that decides what should respond to a click or drag.
- Line 397: Checks nested red endpoint editors first so the inner active dot wins when visible.
- Line 398: Looks up the actual center point of the revealed nested editor handle, if one exists.
- Line 399: If the pointer is close enough to the nested editor dot, select it as the drag target.
- Line 400: If a nested editor handle claims the hit, return that drag target immediately.

## Lines 401-425: CanvasWidget Helpers

```rust
 401 |                 }
 402 |             }
 403 |         }
 404 |         for (index, handle) in self.geometry.baseline().iter().enumerate() {
 405 |             let dist = distance(point, (*handle).into());
 406 |             let nested_claims_center =
 407 |                 self.geometry.endpoint_docked[index] && self.endpoint_revealed[index];
 408 |             if dist <= BASELINE_RADIUS
 409 |                 && (!nested_claims_center || dist > NESTED_ENDPOINT_RADIUS + 2.0)
 410 |             {
 411 |                 return Some(DragTarget::BasePoint(index));
 412 |             }
 413 |         }
 414 |         for (index, handle) in self.geometry.generator_points.iter().enumerate() {
 415 |             if (index == 0 && self.geometry.endpoint_docked[0] && !self.endpoint_revealed[0])
 416 |                 || (index == self.geometry.generator_points.len() - 1
 417 |                     && self.geometry.endpoint_docked[1]
 418 |                     && !self.endpoint_revealed[1])
 419 |             {
 420 |                 continue;
 421 |             }
 422 |             if distance(point, (*handle).into()) <= HIT_RADIUS {
 423 |                 return Some(DragTarget::GeneratorPoint(index));
 424 |             }
 425 |         }
```

- Line 401: This line closes the current block or scope.
- Line 402: This line closes the current block or scope.
- Line 403: This line closes the current block or scope.
- Line 404: Then tests the baseline handles themselves.
- Line 405: Measures distance from the pointer to the current baseline handle center.
- Line 406: Detects the case where a revealed nested editor occupies the same center as the outer baseline handle.
- Line 407: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 408: Lets the outer baseline handle win only in the ring outside the nested active dot.
- Line 409: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 410: This line opens a new block or nested scope.
- Line 411: Returns the matching baseline drag target.
- Line 412: This line closes the current block or scope.
- Line 413: This line closes the current block or scope.
- Line 414: Tests normal generator points after nested and baseline handles.
- Line 415: Skips hidden docked endpoints so they do not block the baseline control beneath them.
- Line 416: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 417: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 418: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 419: This line opens a new block or nested scope.
- Line 420: Skips recursive expansion once the current segment has been rasterized.
- Line 421: This line closes the current block or scope.
- Line 422: If a visible generator handle is within the hit radius, select it.
- Line 423: Returns a drag target for that generator handle.
- Line 424: This line closes the current block or scope.
- Line 425: This line closes the current block or scope.

## Lines 426-450: CanvasWidget Helpers

```rust
 426 |         if point_near_polyline(point, &self.geometry.generator_points, HIT_RADIUS) {
 427 |             return Some(DragTarget::GeneratorShape);
 428 |         }
 429 |         let baseline = self.geometry.baseline();
 430 |         if point_to_segment_distance(point, baseline[0].into(), baseline[1].into()) <= HIT_RADIUS {
 431 |             return Some(DragTarget::Baseline);
 432 |         }
 433 |         None
 434 |     }
 435 | 
 436 |     fn baseline_hit(&self, point: Point) -> Option<usize> {
 437 |         self.geometry
 438 |             .baseline()
 439 |             .iter()
 440 |             .enumerate()
 441 |             .find_map(|(index, handle)| {
 442 |                 (distance(point, (*handle).into()) <= HIT_RADIUS).then_some(index)
 443 |             })
 444 |     }
 445 | 
 446 |     fn revealed_endpoint_handle(&self, base_index: usize) -> Option<Point> {
 447 |         if !self.endpoint_revealed[base_index] {
 448 |             return None;
 449 |         }
 450 |         let point_index = if base_index == 0 {
```

- Line 426: Checks whether the pointer is close enough to the red scaffold polyline itself.
- Line 427: Returns a whole-shape drag target for the generator scaffold.
- Line 428: This line closes the current block or scope.
- Line 429: Reads the current baseline once so the seed job uses a consistent pair of endpoints.
- Line 430: Checks whether the pointer is near the blue baseline segment.
- Line 431: Returns a whole-line drag target for the baseline.
- Line 432: This line closes the current block or scope.
- Line 433: If nothing was hit, report no drag target.
- Line 434: This line closes the current block or scope.
- Line 435: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 436: Begins a helper that detects whether a click landed on one of the baseline handle centers.
- Line 437: Iterates over the two baseline points with their indexes.
- Line 438: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 439: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 440: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 441: Returns the index of the first baseline handle inside the hit radius.
- Line 442: Returns the index of the first baseline handle inside the hit radius.
- Line 443: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 444: This line closes the current block or scope.
- Line 445: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 446: Begins a helper that returns the nested editor handle center when it is visible.
- Line 447: Short-circuits when that endpoint is not currently revealed.
- Line 448: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 449: This line closes the current block or scope.
- Line 450: Maps baseline slot 0 or 1 to the corresponding first or last generator point index.

## Lines 451-475: CanvasWidget Helpers

```rust
 451 |             0
 452 |         } else {
 453 |             self.geometry.generator_points.len() - 1
 454 |         };
 455 |         let point = self.geometry.generator_points[point_index];
 456 |         Some(point.into())
 457 |     }
 458 | }
 459 | 
 460 | struct CanvasView;
 461 | 
 462 | impl ViewMarker for CanvasView {}
 463 | 
 464 | impl View<Edit<InteractivePaintApp>, (), ViewCtx> for CanvasView {
 465 |     type Element = Pod<CanvasWidget>;
 466 |     type ViewState = ();
 467 | 
 468 |     fn build(
 469 |         &self,
 470 |         ctx: &mut ViewCtx,
 471 |         app_state: Arg<'_, Edit<InteractivePaintApp>>,
 472 |     ) -> (Self::Element, Self::ViewState) {
 473 |         let widget = CanvasWidget {
 474 |             geometry: app_state.geometry.clone(),
 475 |             depth: app_state.depth,
```

- Line 451: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 452: This line opens a new block or nested scope.
- Line 453: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 454: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 455: Reads the actual endpoint coordinate from the generator polyline.
- Line 456: Wraps the endpoint coordinate as a Vello `Point` for hit-testing.
- Line 457: This line closes the current block or scope.
- Line 458: This line closes the current block or scope.
- Line 459: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 460: Declares a zero-sized view marker used to mount the custom widget inside the Xilem view tree.
- Line 461: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 462: Marks the canvas view type as a Xilem view marker.
- Line 463: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 464: Begins the Xilem view implementation that bridges app state and the custom widget.
- Line 465: Declares that this view produces a pod-wrapped `CanvasWidget`.
- Line 466: The view has no custom retained view state outside the widget itself.
- Line 467: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 468: Builds the widget from the current app state the first time the view appears.
- Line 469: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 470: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 471: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 472: This line opens a new block or nested scope.
- Line 473: Constructs the initial `CanvasWidget` value from app state.
- Line 474: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 475: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.

## Lines 476-500: View Bridge

```rust
 476 |             show_guides: app_state.show_guides,
 477 |             drag_target: None,
 478 |             drag_anchor: None,
 479 |             drag_start_geometry: None,
 480 |             endpoint_revealed: [false, false],
 481 |             last_click: None,
 482 |             pending_segments: VecDeque::new(),
 483 |             raster_buffer: vec![0; (CANVAS_WIDTH as usize) * (CANVAS_HEIGHT as usize) * 4],
 484 |         };
 485 |         (ctx.with_action_widget(|ctx| ctx.create_pod(widget)), ())
 486 |     }
 487 | 
 488 |     fn rebuild(
 489 |         &self,
 490 |         _prev: &Self,
 491 |         _: &mut Self::ViewState,
 492 |         _: &mut ViewCtx,
 493 |         mut element: Mut<'_, Self::Element>,
 494 |         app_state: Arg<'_, Edit<InteractivePaintApp>>,
 495 |     ) {
 496 |         let needs_reset = element.widget.geometry.generator_points
 497 |             != app_state.geometry.generator_points
 498 |             || element.widget.geometry.baseline_points != app_state.geometry.baseline_points
 499 |             || element.widget.geometry.endpoint_docked != app_state.geometry.endpoint_docked
 500 |             || element.widget.depth != app_state.depth;
```

- Line 476: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 477: Tracks what the user is currently dragging, if anything.
- Line 478: Stores the pointer position where the current drag began.
- Line 479: Stores the geometry snapshot from drag start so each move can be computed from a stable base.
- Line 480: Tracks whether each docked endpoint’s inner red editor dot is currently visible.
- Line 481: Stores the previous click information for double-click detection on docked endpoints.
- Line 482: Pulls in the double-ended queue used as the incremental render work queue.
- Line 483: Persistent RGBA backing buffer that accumulates finished line pixels.
- Line 484: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 485: Creates the pod and wires it up so widget actions flow through the Xilem view context.
- Line 486: This line closes the current block or scope.
- Line 487: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 488: Rebuilds the widget whenever the parent view is refreshed with new app state.
- Line 489: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 490: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 491: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 492: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 493: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 494: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 495: This line opens a new block or nested scope.
- Line 496: Decides whether the fractal image must be discarded and rerendered from scratch.
- Line 497: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 498: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 499: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 500: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.

## Lines 501-525: View Bridge

```rust
 501 |         element.widget.geometry = app_state.geometry.clone();
 502 |         element.widget.depth = app_state.depth;
 503 |         element.widget.show_guides = app_state.show_guides;
 504 |         if needs_reset {
 505 |             element.widget.endpoint_revealed = [false, false];
 506 |             element.widget.reset_render_progress();
 507 |             element.ctx.request_anim_frame();
 508 |         }
 509 |         element.ctx.request_paint_only();
 510 |     }
 511 | 
 512 |     fn teardown(
 513 |         &self,
 514 |         _: &mut Self::ViewState,
 515 |         ctx: &mut ViewCtx,
 516 |         element: Mut<'_, Self::Element>,
 517 |     ) {
 518 |         ctx.teardown_leaf(element);
 519 |     }
 520 | 
 521 |     fn message(
 522 |         &self,
 523 |         _: &mut Self::ViewState,
 524 |         message: &mut MessageContext,
 525 |         _element: Mut<'_, Self::Element>,
```

- Line 501: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 502: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 503: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 504: If a reset is needed, also hide nested endpoint editors and restart rendering.
- Line 505: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 506: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 507: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 508: This line closes the current block or scope.
- Line 509: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 510: This line closes the current block or scope.
- Line 511: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 512: Tears down the leaf widget when the view is removed.
- Line 513: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 514: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 515: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 516: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 517: This line opens a new block or nested scope.
- Line 518: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 519: This line closes the current block or scope.
- Line 520: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 521: Handles messages sent upward from the widget into the view layer.
- Line 522: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 523: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 524: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 525: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.

## Lines 526-550: View Bridge

```rust
 526 |         app_state: Arg<'_, Edit<InteractivePaintApp>>,
 527 |     ) -> MessageResult<()> {
 528 |         if message.take_first().is_some() {
 529 |             return MessageResult::Stale;
 530 |         }
 531 |         match message.take_message::<CanvasAction>() {
 532 |             Some(action) => {
 533 |                 match *action {
 534 |                     CanvasAction::Geometry(ref geometry) => {
 535 |                         app_state.geometry = geometry.clone();
 536 |                     }
 537 |                 }
 538 |                 MessageResult::Action(())
 539 |             }
 540 |             None => MessageResult::Stale,
 541 |         }
 542 |     }
 543 | }
 544 | 
 545 | fn app_logic(data: &mut InteractivePaintApp) -> impl WidgetView<Edit<InteractivePaintApp>> + use<> {
 546 |     flex_col((
 547 |         label("Line Fractal Explorer").text_size(26.0),
 548 |         flex_row((
 549 |             label(format!("Preset: {}", data.preset_name)),
 550 |             label(format!("Depth: {}", data.depth)),
```

- Line 526: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 527: This line opens a new block or nested scope.
- Line 528: Consumes and discards any stale positional message payload before checking the typed action.
- Line 529: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 530: This line closes the current block or scope.
- Line 531: Pulls the next typed `CanvasAction` from the message queue, if one exists.
- Line 532: When a canvas action arrives, match on the specific action payload.
- Line 533: This line opens a new block or nested scope.
- Line 534: The only current action variant replaces the app geometry with the widget’s committed geometry.
- Line 535: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 536: This line closes the current block or scope.
- Line 537: This line closes the current block or scope.
- Line 538: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 539: This line closes the current block or scope.
- Line 540: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 541: This line closes the current block or scope.
- Line 542: This line closes the current block or scope.
- Line 543: This line closes the current block or scope.
- Line 544: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 545: Begins the declarative Xilem view tree for the whole app window.
- Line 546: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 547: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 548: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 549: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 550: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.

## Lines 551-575: UI Composition

```rust
 551 |             label(data.estimated_segments_label()),
 552 |         ))
 553 |         .cross_axis_alignment(CrossAxisAlignment::Center)
 554 |         .gap(18.0.px()),
 555 |         flex_row((
 556 |             text_button("Koch-ish", |data: &mut InteractivePaintApp| {
 557 |                 data.set_preset("Koch-ish", FractalGeometry::koch());
 558 |             }),
 559 |             text_button("Lightning", |data: &mut InteractivePaintApp| {
 560 |                 data.set_preset("Lightning", FractalGeometry::lightning());
 561 |             }),
 562 |             text_button("Canyon", |data: &mut InteractivePaintApp| {
 563 |                 data.set_preset("Canyon", FractalGeometry::canyon());
 564 |             }),
 565 |             text_button("Reset", |data: &mut InteractivePaintApp| {
 566 |                 let preset = data.preset_name;
 567 |                 let geometry = match preset {
 568 |                     "Lightning" => FractalGeometry::lightning(),
 569 |                     "Canyon" => FractalGeometry::canyon(),
 570 |                     _ => FractalGeometry::koch(),
 571 |                 };
 572 |                 data.set_preset(preset, geometry);
 573 |             }),
 574 |         ))
 575 |         .cross_axis_alignment(CrossAxisAlignment::Center)
```

- Line 551: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 552: This line closes a parenthesized or builder-expression block.
- Line 553: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 554: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 555: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 556: This line opens a new block or nested scope.
- Line 557: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 558: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 559: This line opens a new block or nested scope.
- Line 560: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 561: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 562: This line opens a new block or nested scope.
- Line 563: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 564: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 565: This line opens a new block or nested scope.
- Line 566: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 567: This line opens a new block or nested scope.
- Line 568: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 569: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 570: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 571: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 572: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 573: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 574: This line closes a parenthesized or builder-expression block.
- Line 575: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.

## Lines 576-600: UI Composition

```rust
 576 |         .gap(12.0.px()),
 577 |         flex_row((
 578 |             sized_box(label("Depth")).width(48.px()),
 579 |             text_button("-", |data: &mut InteractivePaintApp| {
 580 |                 data.set_depth(data.depth.saturating_sub(1));
 581 |             }),
 582 |             sized_box(
 583 |                 text_input(
 584 |                     data.depth_input.clone(),
 585 |                     |data: &mut InteractivePaintApp, value| {
 586 |                         let trimmed = value.trim();
 587 |                         if let Ok(parsed) = trimmed.parse::<usize>() {
 588 |                             data.set_depth(parsed);
 589 |                         } else {
 590 |                             data.depth_input = value;
 591 |                         }
 592 |                     },
 593 |                 )
 594 |                 .on_enter(|data: &mut InteractivePaintApp, value| {
 595 |                     if let Ok(parsed) = value.trim().parse::<usize>() {
 596 |                         data.set_depth(parsed);
 597 |                     } else {
 598 |                         data.depth_input = data.depth.to_string();
 599 |                     }
 600 |                 })
```

- Line 576: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 577: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 578: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 579: This line opens a new block or nested scope.
- Line 580: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 581: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 582: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 583: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 584: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 585: This line opens a new block or nested scope.
- Line 586: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 587: This line opens a new block or nested scope.
- Line 588: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 589: This line opens a new block or nested scope.
- Line 590: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 591: This line closes the current block or scope.
- Line 592: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 593: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 594: This line opens a new block or nested scope.
- Line 595: This line opens a new block or nested scope.
- Line 596: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 597: This line opens a new block or nested scope.
- Line 598: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 599: This line closes the current block or scope.
- Line 600: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.

## Lines 601-625: UI Composition

```rust
 601 |                 .text_alignment(TextAlign::Center),
 602 |             )
 603 |             .width(80.px()),
 604 |             text_button("+", |data: &mut InteractivePaintApp| {
 605 |                 data.set_depth(data.depth.saturating_add(1));
 606 |             }),
 607 |             checkbox("Show guides", data.show_guides, |data: &mut InteractivePaintApp, checked| {
 608 |                 data.show_guides = checked;
 609 |             }),
 610 |         ))
 611 |         .cross_axis_alignment(CrossAxisAlignment::Center)
 612 |         .gap(12.0.px()),
 613 |         sized_box(CanvasView)
 614 |             .width(CANVAS_WIDTH.px())
 615 |             .height(CANVAS_HEIGHT.px()),
 616 |         label("Drag red handles or the red scaffold to shape the generator. Drag blue handles or the blue line to reposition the baseline."),
 617 |     ))
 618 |     .gap(14.0.px())
 619 |     .padding(20.0)
 620 | }
 621 | 
 622 | fn apply_drag(
 623 |     start_geometry: &FractalGeometry,
 624 |     target: &DragTarget,
 625 |     delta: Vec2,
```

- Line 601: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 602: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 603: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 604: This line opens a new block or nested scope.
- Line 605: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 606: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 607: This line opens a new block or nested scope.
- Line 608: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 609: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 610: This line closes a parenthesized or builder-expression block.
- Line 611: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 612: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 613: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 614: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 615: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 616: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 617: This line closes a parenthesized or builder-expression block.
- Line 618: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 619: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 620: This line closes the current block or scope.
- Line 621: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 622: Begins the pure geometry helper that applies a drag to a snapshot of the starting geometry.
- Line 623: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 624: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 625: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.

## Lines 626-650: Drag Application Logic

```rust
 626 | ) -> FractalGeometry {
 627 |     let mut geometry = start_geometry.clone();
 628 |     let baseline = start_geometry.baseline();
 629 |     match *target {
 630 |         DragTarget::EndpointEditor(base_index) => {
 631 |             let point_index = if base_index == 0 {
 632 |                 0
 633 |             } else {
 634 |                 geometry.generator_points.len() - 1
 635 |             };
 636 |             geometry.generator_points[point_index].0 += delta.x;
 637 |             geometry.generator_points[point_index].1 += delta.y;
 638 |             if distance(
 639 |                 geometry.generator_points[point_index].into(),
 640 |                 geometry.baseline_points[base_index].into(),
 641 |             ) <= BASELINE_RADIUS
 642 |             {
 643 |                 geometry.generator_points[point_index] = geometry.baseline_points[base_index];
 644 |                 geometry.endpoint_docked[base_index] = true;
 645 |             } else {
 646 |                 geometry.endpoint_docked[base_index] = false;
 647 |             }
 648 |         }
 649 |         DragTarget::GeneratorPoint(index) => {
 650 |             if index == 0 || index == geometry.generator_points.len() - 1 {
```

- Line 626: This line opens a new block or nested scope.
- Line 627: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 628: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 629: This line opens a new block or nested scope.
- Line 630: This line opens a new block or nested scope.
- Line 631: Maps baseline slot 0 or 1 to the corresponding first or last generator point index.
- Line 632: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 633: This line opens a new block or nested scope.
- Line 634: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 635: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 636: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 637: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 638: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 639: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 640: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 641: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 642: This line opens a new block or nested scope.
- Line 643: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 644: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 645: This line opens a new block or nested scope.
- Line 646: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 647: This line closes the current block or scope.
- Line 648: This line closes the current block or scope.
- Line 649: This line opens a new block or nested scope.
- Line 650: This line opens a new block or nested scope.

## Lines 651-675: Drag Application Logic

```rust
 651 |                 let base_index = if index == 0 { 0 } else { 1 };
 652 |                 geometry.generator_points[index].0 += delta.x;
 653 |                 geometry.generator_points[index].1 += delta.y;
 654 |                 geometry.endpoint_docked[base_index] = false;
 655 |             } else if let Some(point) = geometry.generator_points.get_mut(index) {
 656 |                 point.0 += delta.x;
 657 |                 point.1 += delta.y;
 658 |             }
 659 |         }
 660 |         DragTarget::BasePoint(index) => {
 661 |             let mut new_baseline = baseline;
 662 |             new_baseline[index] = (baseline[index].0 + delta.x, baseline[index].1 + delta.y);
 663 |             geometry.baseline_points = new_baseline;
 664 |             geometry.generator_points = transform_points_between_baselines(
 665 |                 &start_geometry.generator_points,
 666 |                 baseline,
 667 |                 new_baseline,
 668 |             );
 669 |             sync_docked_endpoints(&mut geometry);
 670 |         }
 671 |         DragTarget::GeneratorShape | DragTarget::Baseline => {
 672 |             for point in &mut geometry.generator_points {
 673 |                 point.0 += delta.x;
 674 |                 point.1 += delta.y;
 675 |             }
```

- Line 651: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 652: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 653: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 654: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 655: This line opens a new block or nested scope.
- Line 656: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 657: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 658: This line closes the current block or scope.
- Line 659: This line closes the current block or scope.
- Line 660: This line opens a new block or nested scope.
- Line 661: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 662: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 663: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 664: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 665: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 666: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 667: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 668: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 669: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 670: This line closes the current block or scope.
- Line 671: This line opens a new block or nested scope.
- Line 672: This line opens a new block or nested scope.
- Line 673: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 674: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 675: This line closes the current block or scope.

## Lines 676-700: Drag Application Logic

```rust
 676 |             if matches!(*target, DragTarget::Baseline) {
 677 |                 for point in &mut geometry.baseline_points {
 678 |                     point.0 += delta.x;
 679 |                     point.1 += delta.y;
 680 |                 }
 681 |                 sync_docked_endpoints(&mut geometry);
 682 |             } else {
 683 |                 geometry.endpoint_docked = [false, false];
 684 |             }
 685 |         }
 686 |     }
 687 |     geometry
 688 | }
 689 | 
 690 | fn sync_docked_endpoints(geometry: &mut FractalGeometry) {
 691 |     if geometry.endpoint_docked[0] {
 692 |         geometry.generator_points[0] = geometry.baseline_points[0];
 693 |     }
 694 |     if geometry.endpoint_docked[1] {
 695 |         let last = geometry.generator_points.len() - 1;
 696 |         geometry.generator_points[last] = geometry.baseline_points[1];
 697 |     }
 698 | }
 699 | 
 700 | fn transform_points_between_baselines(
```

- Line 676: This line opens a new block or nested scope.
- Line 677: This line opens a new block or nested scope.
- Line 678: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 679: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 680: This line closes the current block or scope.
- Line 681: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 682: This line opens a new block or nested scope.
- Line 683: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 684: This line closes the current block or scope.
- Line 685: This line closes the current block or scope.
- Line 686: This line closes the current block or scope.
- Line 687: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 688: This line closes the current block or scope.
- Line 689: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 690: Begins a helper that enforces docked endpoints exactly matching baseline endpoints.
- Line 691: This line opens a new block or nested scope.
- Line 692: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 693: This line closes the current block or scope.
- Line 694: This line opens a new block or nested scope.
- Line 695: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 696: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 697: This line closes the current block or scope.
- Line 698: This line closes the current block or scope.
- Line 699: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 700: Begins a helper that maps points from one baseline frame into another.

## Lines 701-725: Dock Sync And Coordinate Mapping

```rust
 701 |     points: &[(f64, f64)],
 702 |     old_baseline: [(f64, f64); 2],
 703 |     new_baseline: [(f64, f64); 2],
 704 | ) -> Vec<(f64, f64)> {
 705 |     let local_points = normalized_points_from_baseline(points, old_baseline);
 706 |     local_points
 707 |         .into_iter()
 708 |         .map(|local| {
 709 |             let mapped = map_local(new_baseline[0].into(), new_baseline[1].into(), local);
 710 |             (mapped.x, mapped.y)
 711 |         })
 712 |         .collect()
 713 | }
 714 | 
 715 | 
 716 | fn normalized_points_from_baseline(
 717 |     points: &[(f64, f64)],
 718 |     baseline: [(f64, f64); 2],
 719 | ) -> Vec<(f64, f64)> {
 720 |     if points.len() < 2 {
 721 |         return vec![(0.0, 0.0), (1.0, 0.0)];
 722 |     }
 723 | 
 724 |     let start = Point::new(baseline[0].0, baseline[0].1);
 725 |     let end = Point::new(baseline[1].0, baseline[1].1);
```

- Line 701: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 702: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 703: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 704: This line opens a new block or nested scope.
- Line 705: Normalizes the current generator into baseline-relative coordinates for recursive mapping.
- Line 706: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 707: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 708: This line opens a new block or nested scope.
- Line 709: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 710: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 711: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 712: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 713: This line closes the current block or scope.
- Line 714: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 715: Blank line that separates the transformation helper from the normalization helper below it.
- Line 716: Begins the helper that projects arbitrary points into a baseline-relative coordinate frame.
- Line 717: Accepts the points to normalize.
- Line 718: Accepts the baseline that defines the local x-axis and origin.
- Line 719: Starts the function body and declares the normalized point list as the return value.
- Line 720: Handles degenerate input that does not contain enough points to define a polyline.
- Line 721: Returns a default unit segment so downstream code still has sensible endpoints.
- Line 722: Closes that early-return branch.
- Line 719: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 720: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 721: This line closes the current block or scope.
- Line 722: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 723: Begins the baseline-aware normalization routine used by both transforms and recursive rendering.
- Line 724: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 725: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.

## Lines 726-750: Dock Sync And Coordinate Mapping

```rust
 726 | ) -> Vec<(f64, f64)> {
 727 |     if points.len() < 2 {
 728 |         return vec![(0.0, 0.0), (1.0, 0.0)];
 729 |     }
 730 | 
 731 |     let start = Point::new(baseline[0].0, baseline[0].1);
 732 |     let end = Point::new(baseline[1].0, baseline[1].1);
 733 |     let line = end - start;
 734 |     let length_sq = line.hypot2();
 735 |     if length_sq <= f64::EPSILON {
 736 |         return vec![(0.0, 0.0), (1.0, 0.0)];
 737 |     }
 738 | 
 739 |     points
 740 |         .iter()
 741 |         .map(|&(x, y)| {
 742 |             let offset = Point::new(x, y) - start;
 743 |             let local_x = offset.dot(line) / length_sq;
 744 |             let local_y = cross(line, offset) / length_sq;
 745 |             (local_x, local_y)
 746 |         })
 747 |         .collect()
 748 | }
 749 | 
 750 | fn map_local(start: Point, end: Point, local: (f64, f64)) -> Point {
```

- Line 726: This line opens a new block or nested scope.
- Line 727: This line opens a new block or nested scope.
- Line 728: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 729: This line closes the current block or scope.
- Line 730: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 731: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 732: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 733: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 734: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 735: This line opens a new block or nested scope.
- Line 736: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 737: This line closes the current block or scope.
- Line 738: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 739: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 740: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 741: This line opens a new block or nested scope.
- Line 742: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 743: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 744: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 745: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 746: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 747: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 748: This line closes the current block or scope.
- Line 749: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 750: Begins the helper that maps a normalized local point back onto an arbitrary baseline segment.

## Lines 751-775: Dock Sync And Coordinate Mapping

```rust
 751 |     let direction = end - start;
 752 |     let perpendicular = Vec2::new(-direction.y, direction.x);
 753 |     start + direction * local.0 + perpendicular * local.1
 754 | }
 755 | 
 756 | fn paint_generator_guides(
 757 |     scene: &mut Scene,
 758 |     geometry: &FractalGeometry,
 759 |     endpoint_revealed: [bool; 2],
 760 | ) {
 761 |     let points = &geometry.generator_points;
 762 |     if points.len() >= 2 {
 763 |         let mut path = BezPath::new();
 764 |         path.move_to(Point::new(points[0].0, points[0].1));
 765 |         for point in &points[1..] {
 766 |             path.line_to(Point::new(point.0, point.1));
 767 |         }
 768 |         scene.stroke(
 769 |             &Stroke::new(2.0),
 770 |             Affine::IDENTITY,
 771 |             Color::from_rgb8(186, 57, 39),
 772 |             None,
 773 |             &path,
 774 |         );
 775 |     }
```

- Line 751: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 752: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 753: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 754: This line closes the current block or scope.
- Line 755: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 756: Begins painting the red generator scaffold and red/docked handles.
- Line 757: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 758: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 759: Tracks whether each docked endpoint’s inner red editor dot is currently visible.
- Line 760: This line opens a new block or nested scope.
- Line 761: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 762: This line opens a new block or nested scope.
- Line 763: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 764: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 765: This line opens a new block or nested scope.
- Line 766: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 767: This line closes the current block or scope.
- Line 768: Strokes the preview panel border.
- Line 769: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 770: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 771: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 772: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 773: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 774: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 775: This line closes the current block or scope.

## Lines 776-800: Guide And Handle Painting

```rust
 776 | 
 777 |     for (index, &(x, y)) in points.iter().enumerate() {
 778 |         let is_endpoint = index == 0 || index == points.len() - 1;
 779 |         let slot = if index == 0 { 0 } else { 1 };
 780 |         if is_endpoint && geometry.endpoint_docked[slot] {
 781 |             paint_docked_endpoint(scene, Point::new(x, y), endpoint_revealed[slot]);
 782 |         } else {
 783 |             fill(
 784 |                 scene,
 785 |                 &Circle::new((x, y), HANDLE_RADIUS),
 786 |                 Color::from_rgb8(215, 83, 63),
 787 |             );
 788 |         }
 789 |     }
 790 | }
 791 | 
 792 | fn paint_baseline_line(scene: &mut Scene, base_line: [(f64, f64); 2]) {
 793 |     scene.stroke(
 794 |         &Stroke::new(3.0),
 795 |         Affine::IDENTITY,
 796 |         Color::from_rgb8(55, 108, 171),
 797 |         None,
 798 |         &Line::new(base_line[0], base_line[1]),
 799 |     );
 800 | }
```

- Line 776: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 777: This line opens a new block or nested scope.
- Line 778: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 779: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 780: This line opens a new block or nested scope.
- Line 781: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 782: This line opens a new block or nested scope.
- Line 783: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 784: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 785: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 786: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 787: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 788: This line closes the current block or scope.
- Line 789: This line closes the current block or scope.
- Line 790: This line closes the current block or scope.
- Line 791: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 792: Begins painting the blue baseline line.
- Line 793: Strokes the preview panel border.
- Line 794: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 795: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 796: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 797: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 798: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 799: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 800: This line closes the current block or scope.

## Lines 801-825: Guide And Handle Painting

```rust
 801 | 
 802 | fn paint_baseline_handles(
 803 |     scene: &mut Scene,
 804 |     geometry: &FractalGeometry,
 805 |     endpoint_revealed: [bool; 2],
 806 | ) {
 807 |     for (index, point) in geometry.baseline().into_iter().enumerate() {
 808 |         if geometry.endpoint_docked[index] {
 809 |             paint_docked_endpoint(scene, point.into(), endpoint_revealed[index]);
 810 |         } else {
 811 |             fill(
 812 |                 scene,
 813 |                 &Circle::new(point, BASELINE_RADIUS),
 814 |                 Color::from_rgb8(87, 140, 201),
 815 |             );
 816 |         }
 817 |     }
 818 | }
 819 | 
 820 | fn point_near_polyline(point: Point, polyline: &[(f64, f64)], max_distance: f64) -> bool {
 821 |     polyline.windows(2).any(|segment| {
 822 |         point_to_segment_distance(point, segment[0].into(), segment[1].into()) <= max_distance
 823 |     })
 824 | }
 825 | 
```

- Line 801: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 802: Begins painting the blue baseline handles.
- Line 803: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 804: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 805: Tracks whether each docked endpoint’s inner red editor dot is currently visible.
- Line 806: This line opens a new block or nested scope.
- Line 807: This line opens a new block or nested scope.
- Line 808: This line opens a new block or nested scope.
- Line 809: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 810: This line opens a new block or nested scope.
- Line 811: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 812: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 813: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 814: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 815: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 816: This line closes the current block or scope.
- Line 817: This line closes the current block or scope.
- Line 818: This line closes the current block or scope.
- Line 819: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 820: Begins a helper that checks whether a point is close to any segment in a polyline.
- Line 821: This line opens a new block or nested scope.
- Line 822: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 823: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 824: This line closes the current block or scope.
- Line 825: Blank line that separates the surrounding ideas so the next block is easier to scan.

## Lines 826-850: Hit Testing Math

```rust
 826 | fn point_to_segment_distance(point: Point, start: Point, end: Point) -> f64 {
 827 |     let line = end - start;
 828 |     let length_sq = line.hypot2();
 829 |     if length_sq <= f64::EPSILON {
 830 |         return distance(point, start);
 831 |     }
 832 |     let t = ((point - start).dot(line) / length_sq).clamp(0.0, 1.0);
 833 |     let projection = start + line * t;
 834 |     distance(point, projection)
 835 | }
 836 | 
 837 | fn distance(a: Point, b: Point) -> f64 {
 838 |     (a - b).hypot()
 839 | }
 840 | 
 841 | fn cross(a: Vec2, b: Vec2) -> f64 {
 842 |     a.x * b.y - a.y * b.x
 843 | }
 844 | 
 845 | fn paint_docked_endpoint(scene: &mut Scene, center: Point, red_active: bool) {
 846 |     let outer_radius = BASELINE_RADIUS;
 847 |     let inner_radius = NESTED_ENDPOINT_RADIUS;
 848 |     let blue = Color::from_rgb8(87, 140, 201);
 849 |     let red = Color::from_rgb8(184, 49, 90);
 850 | 
```

- Line 826: Begins the exact point-to-segment distance helper.
- Line 827: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 828: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 829: This line opens a new block or nested scope.
- Line 830: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 831: This line closes the current block or scope.
- Line 832: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 833: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 834: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 835: This line closes the current block or scope.
- Line 836: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 837: Begins a simple Euclidean distance helper for points.
- Line 838: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 839: This line closes the current block or scope.
- Line 840: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 841: Begins a helper that computes the 2D scalar cross product.
- Line 842: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 843: This line closes the current block or scope.
- Line 844: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 845: Begins painting the shared concentric docked-endpoint visual.
- Line 846: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 847: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 848: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 849: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 850: Blank line that separates the surrounding ideas so the next block is easier to scan.

## Lines 851-875: Docked Endpoint Visuals

```rust
 851 |     let (outer_color, inner_color, ring_color) = if red_active {
 852 |         (blue, red, Color::from_rgb8(230, 238, 250))
 853 |     } else {
 854 |         (red, blue, Color::from_rgb8(250, 227, 235))
 855 |     };
 856 | 
 857 |     fill(scene, &Circle::new(center, outer_radius), outer_color);
 858 |     scene.stroke(
 859 |         &Stroke::new(1.0),
 860 |         Affine::IDENTITY,
 861 |         ring_color,
 862 |         None,
 863 |         &Circle::new(center, outer_radius),
 864 |     );
 865 |     fill(scene, &Circle::new(center, inner_radius), inner_color);
 866 | }
 867 | 
 868 | fn rasterize_line(
 869 |     buffer: &mut [u8],
 870 |     width: usize,
 871 |     height: usize,
 872 |     start: Point,
 873 |     end: Point,
 874 |     color: [u8; 4],
 875 | ) {
```

- Line 851: This line opens a new block or nested scope.
- Line 852: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 853: This line opens a new block or nested scope.
- Line 854: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 855: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 856: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 857: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 858: Strokes the preview panel border.
- Line 859: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 860: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 861: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 862: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 863: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 864: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 865: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 866: This line closes the current block or scope.
- Line 867: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 868: Begins rasterizing a line directly into the RGBA backing buffer.
- Line 869: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 870: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 871: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 872: Stores the world-space start point of a pending recursive segment.
- Line 873: Stores the world-space end point of a pending recursive segment.
- Line 874: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 875: This line opens a new block or nested scope.

## Lines 876-900: Backing Buffer Rasterization

```rust
 876 |     let dx = end.x - start.x;
 877 |     let dy = end.y - start.y;
 878 |     let steps = dx.abs().max(dy.abs()).ceil() as usize;
 879 |     if steps == 0 {
 880 |         blend_pixel(
 881 |             buffer,
 882 |             width,
 883 |             height,
 884 |             start.x.round() as isize,
 885 |             start.y.round() as isize,
 886 |             color,
 887 |         );
 888 |         return;
 889 |     }
 890 | 
 891 |     for step in 0..=steps {
 892 |         let t = step as f64 / steps as f64;
 893 |         let x = start.x + dx * t;
 894 |         let y = start.y + dy * t;
 895 |         blend_pixel(
 896 |             buffer,
 897 |             width,
 898 |             height,
 899 |             x.round() as isize,
 900 |             y.round() as isize,
```

- Line 876: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 877: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 878: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 879: This line opens a new block or nested scope.
- Line 880: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 881: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 882: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 883: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 884: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 885: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 886: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 887: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 888: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 889: This line closes the current block or scope.
- Line 890: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 891: This line opens a new block or nested scope.
- Line 892: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 893: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 894: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 895: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 896: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 897: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 898: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 899: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 900: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.

## Lines 901-925: Backing Buffer Rasterization

```rust
 901 |             color,
 902 |         );
 903 |     }
 904 | }
 905 | 
 906 | fn blend_pixel(buffer: &mut [u8], width: usize, height: usize, x: isize, y: isize, color: [u8; 4]) {
 907 |     if x < 0 || y < 0 || x >= width as isize || y >= height as isize {
 908 |         return;
 909 |     }
 910 |     let idx = ((y as usize * width) + x as usize) * 4;
 911 |     let alpha = color[3] as f32 / 255.0;
 912 |     let inv_alpha = 1.0 - alpha;
 913 |     buffer[idx] = (color[0] as f32 * alpha + buffer[idx] as f32 * inv_alpha).round() as u8;
 914 |     buffer[idx + 1] = (color[1] as f32 * alpha + buffer[idx + 1] as f32 * inv_alpha).round() as u8;
 915 |     buffer[idx + 2] = (color[2] as f32 * alpha + buffer[idx + 2] as f32 * inv_alpha).round() as u8;
 916 |     buffer[idx + 3] =
 917 |         ((alpha + (buffer[idx + 3] as f32 / 255.0) * inv_alpha) * 255.0).round() as u8;
 918 | }
 919 | 
 920 | fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
 921 |     let data = InteractivePaintApp::default();
 922 |     let app = Xilem::new_simple(
 923 |         data,
 924 |         app_logic,
 925 |         WindowOptions::new("Interactive Line Fractal")
```

- Line 901: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 902: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 903: This line closes the current block or scope.
- Line 904: This line closes the current block or scope.
- Line 905: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 906: Begins alpha blending a single pixel into the RGBA backing buffer.
- Line 907: This line opens a new block or nested scope.
- Line 908: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 909: This line closes the current block or scope.
- Line 910: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 911: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 912: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 913: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 914: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 915: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 916: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 917: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 918: This line closes the current block or scope.
- Line 919: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 920: Imports the error type returned when building or running the window event loop.
- Line 921: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 922: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 923: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 924: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 925: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.

## Lines 926-950: Application Entry Points

```rust
 926 |             .with_initial_inner_size(LogicalSize::new(980.0, 860.0)),
 927 |     );
 928 |     app.run_in(event_loop)
 929 | }
 930 | 
 931 | fn main() -> Result<(), EventLoopError> {
 932 |     run(EventLoop::with_user_event())
 933 | }
 934 | 
 935 | #[cfg(target_os = "android")]
 936 | #[no_mangle]
 937 | fn android_main(app: winit::platform::android::activity::AndroidApp) {
 938 |     use winit::platform::android::EventLoopBuilderExtAndroid;
 939 | 
 940 |     let mut event_loop = EventLoop::with_user_event();
 941 |     event_loop.with_android_app(app);
 942 | 
 943 |     run(event_loop).expect("Can create app");
 944 | }
 945 | 
 946 | #[cfg(test)]
 947 | mod tests {
 948 |     use super::*;
 949 | 
 950 |     #[test]
```

- Line 926: Imports logical window sizing for the example window configuration.
- Line 927: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 928: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 929: This line closes the current block or scope.
- Line 930: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 931: Imports the error type returned when building or running the window event loop.
- Line 932: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 933: This line closes the current block or scope.
- Line 934: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 935: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 936: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 937: Android entry point used when compiling for Android targets.
- Line 938: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 939: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 940: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 941: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 942: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 943: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 944: This line closes the current block or scope.
- Line 945: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 946: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 947: Starts the unit-test module for the example geometry helpers.
- Line 948: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 949: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 950: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.

## Lines 951-975: Tests

```rust
1048 |     #[test]
1049 |     fn normalized_points_can_use_baseline_distinct_from_red_endpoints() {
1050 |         let points = vec![(10.0, 10.0), (20.0, 30.0), (40.0, 10.0)];
1051 |         let baseline = [(0.0, 10.0), (50.0, 10.0)];
1052 |         let local = normalized_points_from_baseline(&points, baseline);
1053 | 
1054 |         assert!((local[0].0 - 0.2).abs() < 1e-9);
1055 |         assert!((local[0].1 - 0.0).abs() < 1e-9);
1056 |         assert!((local[2].0 - 0.8).abs() < 1e-9);
1057 |         assert!((local[2].1 - 0.0).abs() < 1e-9);
1058 |     }
1059 | 
1060 |     #[test]
1061 |     fn map_local_respects_segment_frame() {
1062 |         let start = Point::new(5.0, 7.0);
1063 |         let end = Point::new(25.0, 7.0);
1064 |         let mapped = map_local(start, end, (0.5, 0.25));
1065 | 
1066 |         assert!((mapped.x - 15.0).abs() < 1e-9);
1067 |         assert!((mapped.y - 12.0).abs() < 1e-9);
1068 |     }
1069 | 
1070 |     #[test]
1071 |     fn dragging_generator_shape_translates_all_generator_points_only() {
1072 |         let geometry = FractalGeometry::koch();
1073 |         let moved = apply_drag(
```

- Line 951: This line opens a new block or nested scope.
- Line 952: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 953: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 954: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 955: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 956: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 957: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 958: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 959: This line closes the current block or scope.
- Line 960: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 961: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 962: Begins the helper that maps a normalized local point back onto an arbitrary baseline segment.
- Line 963: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 964: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 965: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 966: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 967: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 968: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 969: This line closes the current block or scope.
- Line 970: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 971: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 972: This line opens a new block or nested scope.
- Line 973: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 974: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 975: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.

## Lines 976-1000: Tests

```rust
 976 |             &DragTarget::GeneratorShape,
 977 |             Vec2::new(10.0, -5.0),
 978 |         );
 979 | 
 980 |         assert_eq!(
 981 |             moved.generator_points[0],
 982 |             (
 983 |                 geometry.generator_points[0].0 + 10.0,
 984 |                 geometry.generator_points[0].1 - 5.0
 985 |             )
 986 |         );
 987 |         assert_eq!(
 988 |             moved.generator_points[moved.generator_points.len() - 1],
 989 |             (
 990 |                 geometry.generator_points[geometry.generator_points.len() - 1].0 + 10.0,
 991 |                 geometry.generator_points[geometry.generator_points.len() - 1].1 - 5.0
 992 |             )
 993 |         );
 994 |     }
 995 | 
 996 |     #[test]
 997 |     fn dragging_display_endpoint_transforms_interior_control_points() {
 998 |         let geometry = FractalGeometry::koch();
 999 |         let moved = apply_drag(&geometry, &DragTarget::BasePoint(0), Vec2::new(20.0, 30.0));
1000 | 
```

- Line 976: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 977: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 978: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 979: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 980: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 981: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 982: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 983: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 984: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 985: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 986: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 987: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 988: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 989: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 990: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 991: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 992: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 993: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 994: This line closes the current block or scope.
- Line 995: Blank line that separates the surrounding ideas so the next block is easier to scan.
- Line 996: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 997: This line opens a new block or nested scope.
- Line 998: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 999: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 1000: Blank line that separates the surrounding ideas so the next block is easier to scan.

## Lines 1001-1010: Tests

```rust
1001 |         assert_eq!(
1002 |             moved.baseline()[0],
1003 |             (
1004 |                 geometry.baseline()[0].0 + 20.0,
1005 |                 geometry.baseline()[0].1 + 30.0
1006 |             )
1007 |         );
1008 |         assert_ne!(moved.generator_points[1], geometry.generator_points[1]);
1009 |     }
1010 | }
```

- Line 1001: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 1002: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 1003: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 1004: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 1005: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 1006: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 1007: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 1008: This line is part of the surrounding expression or control flow; read it together with the nearby lines in this section.
- Line 1009: This line closes the current block or scope.
- Line 1010: This line closes the current block or scope.
