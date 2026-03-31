# Xilem Architecture Overview

Xilem is a reactive, natively-compiled GUI framework built on a layered stack: **Masonry** handles the widget tree and platform integration, while **Vello** provides GPU-accelerated 2D rendering.

## The Stack

```
┌─────────────────────────────────────────────┐
│  Xilem (Application Logic)                 │
│  - View trait, reactive state management    │
│  - Element tree (widget diffing)            │
├─────────────────────────────────────────────┤
│  Masonry (Widget Framework)                 │
│  - Widget trait, layout, event handling     │
│  - Pass system, accessibility               │
├─────────────────────────────────────────────┤
│  Masonry-Winit (Platform Backend)           │
│  - Window creation, event loop              │
│  - Input translation                        │
├─────────────────────────────────────────────┤
│  Vello (Rendering)                          │
│  - GPU-accelerated 2D scenes                │
│  - Path rendering, gradients, images        │
├─────────────────────────────────────────────┤
│  Winit (Platform Windows)                  │
│  - Window management                        │
└─────────────────────────────────────────────┘
```

## Reactive Architecture

Xilem follows a **reactive model** inspired by React, Elm, and SwiftUI:

1. **State Change** - Application state mutates (e.g., button click)
2. **View Generation** - User-provided functions generate a lightweight **view tree**
3. **Diffing** - The new view tree is compared against the previous one
4. **Widget Mutation** - Masonry updates the retained **element tree** (widget tree) based on differences

```rust
fn app_logic(data: &mut AppData) -> impl View<AppData> {
    button("Click me", |data: &mut AppData| data.count += 1)
}
```

This pattern avoids manual DOM/UI updates—the framework handles synchronization between the view description and the actual widget tree.

## Masonry: Widget Framework

Masonry provides the foundation for building native Rust GUIs. It owns the widget tree and manages the rendering lifecycle through a **pass system**.

### The Widget Trait

Every UI component implements the `Widget` trait with key methods:

- `on_pointer_event` / `on_text_event` - Handle user input
- `layout` - Compute size given constraints
- `paint` - Render into a Vello `Scene`
- `update` - Respond to property/state changes

### Pass System

Masonry runs computations (passes) over the widget tree each frame:

| Category | Passes | Purpose |
|----------|--------|---------|
| **Event** | `on_pointer_event`, `on_text_event`, `on_access_event` | Handle input with event bubbling |
| **Rewrite** | `mutate`, `update_widget_tree`, `update_focus`, `layout`, `compose` | Sync tree state after events |
| **Render** | `paint`, `accessibility` | Produce output for display |

**Rewrite passes** may run multiple iterations until all invalidation flags are cleared. Layout flows top-down (constraints) then bottom-up (sizes), while compose assigns transforms for final positioning.

## Vello: GPU Rendering

Vello is a 2D graphics library that compiles drawing commands into GPU-friendly scenes:

- **Paths & strokes** - Anti-aliased vector rendering
- **Fills** - Solid colors, gradients, patterns
- **Images** - Raster data drawn into the scene
- **Transforms** - Affine transformations for positioning

Widgets receive a `Scene` in their `paint` method and add drawing commands:

```rust
fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
    scene.fill(Fill::NonZero, Affine::IDENTITY, Color::BLUE, None, &rect);
    scene.stroke(&Stroke::new(2.0), Affine::IDENTITY, Color::BLACK, None, &path);
}
```

Scenes are stitch together in pre-order (parent before children), allowing natural layering.

## Event Flow

```
User Input (click, keypress)
    │
    ▼
Winit receives platform event
    │
    ▼
Masonry-Winit translates to Masonry events
    │
    ▼
RenderRoot runs Event Passes
    │   - Bubbles up from target widget to root
    │   - Widgets mark events as "handled"
    ▼
RenderRoot runs Rewrite Passes
    │   - Mutate callbacks execute
    │   - Widget tree updates (add/remove children)
    │   - Layout recomputes sizes
    │   - Compose assigns transforms
    ▼
Xilem applies view changes
    │   - Diff view tree against previous
    │   - Update Masonry widget tree accordingly
    ▼
RenderRoot runs Render Passes
    │   - Each widget paints into Scene
    │   - Vello composites final frame
    ▼
GPU displays result
```

## Key Types

| Type | Layer | Role |
|------|-------|------|
| `View` trait | Xilem | Describe UI declaratively |
| `ViewCtx` | Xilem | Bridge between views and Masonry |
| `WidgetPod` | Masonry | Owns a child widget with its state |
| `WidgetMut` | Masonry | Mutable access to widgets with metadata propagation |
| `WidgetRef` | Masonry | Read-only access to widget state |
| `Scene` | Vello | Accumulator for 2D drawing commands |
| `RenderRoot` | Masonry | Composition root owning the widget tree |

## Custom Widgets

Building a custom widget involves implementing `Widget` and using Vello for rendering:

```rust
impl Widget for CanvasWidget {
    type Action = CanvasAction;  // Messages sent back to Xilem

    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        // Handle clicks, drags, etc.
        if let PointerEvent::Up(_) = event {
            ctx.submit_action(CanvasAction::Geometry(self.geometry.clone()));
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        // Draw with Vello
        scene.fill(Fill::NonZero, Affine::IDENTITY, Color::WHITE, None, &rect);
        scene.draw_image(&image_brush, Affine::IDENTITY);
    }
}
```

The `WidgetPod` wraps the custom widget, and Xilem connects the widget's `Action` type to application messages through the view system.
