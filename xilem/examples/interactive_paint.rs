// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Interactive line fractal explorer built with a custom paint widget.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use masonry::core::*;
use masonry::dpi::LogicalSize;
use masonry::peniko::{Fill, ImageBrush, ImageFormat};
use masonry::properties::types::{AsUnit, CrossAxisAlignment};
use masonry::util::fill;
use masonry::vello::kurbo::{Affine, BezPath, Circle, Line, Point, Rect, Size, Stroke, Vec2};
use masonry::vello::Scene;
use masonry_winit::app::{EventLoop, EventLoopBuilder};
use vello::peniko::{ImageAlphaType, ImageData};
use winit::error::EventLoopError;
use xilem::core::{Arg, MessageContext, Mut, View, ViewMarker};
use xilem::style::Style;
use xilem::view::{
    checkbox, flex_col, flex_row, grid, label, portal, sized_box, text_button, text_input, GridExt,
};
use xilem::{Color, Pod, TextAlign, ViewCtx, WidgetView, WindowOptions, Xilem};
use xilem_core::{Edit, MessageResult};

const CANVAS_WIDTH: f64 = 920.0;
const CANVAS_HEIGHT: f64 = 620.0;
const PRESET_GRID_COLUMNS: i32 = 4;
const PRESET_PANEL_HEIGHT: f64 = 160.0;
const HANDLE_RADIUS: f64 = 8.0;
const BASELINE_RADIUS: f64 = HANDLE_RADIUS + 1.5;
const NESTED_ENDPOINT_RADIUS: f64 = 4.5;
const NESTED_HIT_TOLERANCE: f64 = 3.0;
const HIT_RADIUS: f64 = 14.0;
const MIN_SEGMENT_LENGTH: f64 = 2.5;
const RENDER_BATCH_BUDGET: Duration = Duration::from_millis(5);
const DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(300);
const PRESET_TARGET_WIDTH: f64 = 560.0;
const PRESET_TARGET_HEIGHT: f64 = 250.0;
const PRESET_TARGET_CENTER: (f64, f64) = (350.0, 150.0);
const FRACTAL_COLOR: [u8; 4] = [28, 96, 99, 255];

const KOCH_POINTS: &[(f64, f64)] = &[
    (160.0, 150.0),
    (260.0, 150.0),
    (310.0, 85.0),
    (360.0, 150.0),
    (460.0, 150.0),
];
const LIGHTNING_POINTS: &[(f64, f64)] = &[
    (170.0, 150.0),
    (235.0, 125.0),
    (285.0, 190.0),
    (350.0, 110.0),
    (415.0, 165.0),
    (480.0, 145.0),
];
const CANYON_POINTS: &[(f64, f64)] = &[
    (170.0, 155.0),
    (245.0, 155.0),
    (285.0, 215.0),
    (350.0, 95.0),
    (415.0, 215.0),
    (455.0, 155.0),
    (530.0, 155.0),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PresetGroup {
    Classic,
    Experiment,
}

#[derive(Clone, Copy, Debug)]
enum GeneratorKind {
    Turtle {
        commands: &'static str,
        angle_deg: f64,
    },
    Points(&'static [(f64, f64)]),
}

#[derive(Clone, Copy, Debug)]
struct PresetSpec {
    name: &'static str,
    group: PresetGroup,
    generator: GeneratorKind,
}

const PRESETS: &[PresetSpec] = &[
    PresetSpec {
        name: "Koch Curve",
        group: PresetGroup::Classic,
        generator: GeneratorKind::Points(KOCH_POINTS),
    },
    PresetSpec {
        name: "Anti-Koch Inlet",
        group: PresetGroup::Classic,
        generator: GeneratorKind::Turtle {
            commands: "F-F++F-F",
            angle_deg: 60.0,
        },
    },
    PresetSpec {
        name: "Cesaro 70",
        group: PresetGroup::Classic,
        generator: GeneratorKind::Turtle {
            commands: "F+F--F+F",
            angle_deg: 70.0,
        },
    },
    PresetSpec {
        name: "Cesaro 85",
        group: PresetGroup::Classic,
        generator: GeneratorKind::Turtle {
            commands: "F+F--F+F",
            angle_deg: 85.0,
        },
    },
    PresetSpec {
        name: "Minkowski Sausage",
        group: PresetGroup::Classic,
        generator: GeneratorKind::Turtle {
            commands: "F+F-F-FF+F+F-F",
            angle_deg: 90.0,
        },
    },
    PresetSpec {
        name: "Minkowski Mirror",
        group: PresetGroup::Classic,
        generator: GeneratorKind::Turtle {
            commands: "F-F+F+FF-F-F+F",
            angle_deg: 90.0,
        },
    },
    PresetSpec {
        name: "Levy C",
        group: PresetGroup::Classic,
        generator: GeneratorKind::Turtle {
            commands: "F-F",
            angle_deg: 90.0,
        },
    },
    PresetSpec {
        name: "Dragon Fold",
        group: PresetGroup::Classic,
        generator: GeneratorKind::Turtle {
            commands: "F+F",
            angle_deg: 90.0,
        },
    },
    PresetSpec {
        name: "Terdragon",
        group: PresetGroup::Classic,
        generator: GeneratorKind::Turtle {
            commands: "F+F-F",
            angle_deg: 120.0,
        },
    },
    PresetSpec {
        name: "Arrowhead",
        group: PresetGroup::Classic,
        generator: GeneratorKind::Turtle {
            commands: "F+F-F",
            angle_deg: 60.0,
        },
    },
    PresetSpec {
        name: "Gosper Seed",
        group: PresetGroup::Classic,
        generator: GeneratorKind::Turtle {
            commands: "A-B--B+A++AA+B-",
            angle_deg: 60.0,
        },
    },
    PresetSpec {
        name: "Hilbert U",
        group: PresetGroup::Classic,
        generator: GeneratorKind::Turtle {
            commands: "F+F-F",
            angle_deg: 90.0,
        },
    },
    PresetSpec {
        name: "Peano Serpent",
        group: PresetGroup::Classic,
        generator: GeneratorKind::Turtle {
            commands: "FF+F+F+FF-F-F+FF",
            angle_deg: 90.0,
        },
    },
    PresetSpec {
        name: "Lightning",
        group: PresetGroup::Experiment,
        generator: GeneratorKind::Points(LIGHTNING_POINTS),
    },
    PresetSpec {
        name: "Canyon",
        group: PresetGroup::Experiment,
        generator: GeneratorKind::Points(CANYON_POINTS),
    },
    PresetSpec {
        name: "Sawblade",
        group: PresetGroup::Experiment,
        generator: GeneratorKind::Turtle {
            commands: "F+F-F+F-F+F",
            angle_deg: 60.0,
        },
    },
    PresetSpec {
        name: "Harbor Steps",
        group: PresetGroup::Experiment,
        generator: GeneratorKind::Turtle {
            commands: "FF+F-F+FF--F+F",
            angle_deg: 90.0,
        },
    },
    PresetSpec {
        name: "Crown",
        group: PresetGroup::Experiment,
        generator: GeneratorKind::Turtle {
            commands: "F+F-F+F--F+F",
            angle_deg: 72.0,
        },
    },
    PresetSpec {
        name: "Orbit Hook",
        group: PresetGroup::Experiment,
        generator: GeneratorKind::Turtle {
            commands: "F+F++F--F-F",
            angle_deg: 45.0,
        },
    },
    PresetSpec {
        name: "Metro Weave",
        group: PresetGroup::Experiment,
        generator: GeneratorKind::Turtle {
            commands: "F+F-F-F+FF-F+F",
            angle_deg: 90.0,
        },
    },
    PresetSpec {
        name: "Trident",
        group: PresetGroup::Experiment,
        generator: GeneratorKind::Turtle {
            commands: "F+F--F++F--F+F",
            angle_deg: 60.0,
        },
    },
    PresetSpec {
        name: "Needle Fern",
        group: PresetGroup::Experiment,
        generator: GeneratorKind::Turtle {
            commands: "F+F-F+F++F-F",
            angle_deg: 36.0,
        },
    },
    PresetSpec {
        name: "Ribbon Fold",
        group: PresetGroup::Experiment,
        generator: GeneratorKind::Turtle {
            commands: "F-F+F++F-F",
            angle_deg: 72.0,
        },
    },
    PresetSpec {
        name: "Wave Tank",
        group: PresetGroup::Experiment,
        generator: GeneratorKind::Turtle {
            commands: "F+F-F--F+F++F-F",
            angle_deg: 45.0,
        },
    },
    PresetSpec {
        name: "Catapult",
        group: PresetGroup::Experiment,
        generator: GeneratorKind::Turtle {
            commands: "F++F-F+F--F",
            angle_deg: 60.0,
        },
    },
    PresetSpec {
        name: "Switchback",
        group: PresetGroup::Experiment,
        generator: GeneratorKind::Turtle {
            commands: "FF-F+F-F+FF+F-F",
            angle_deg: 90.0,
        },
    },
    PresetSpec {
        name: "Kite Spine",
        group: PresetGroup::Experiment,
        generator: GeneratorKind::Turtle {
            commands: "F+F+F--F-F",
            angle_deg: 72.0,
        },
    },
];
const BENCHMARK_PRESET_NAMES: &[&str] = &[
    "Koch Curve",
    "Minkowski Sausage",
    "Gosper Seed",
    "Metro Weave",
    "Switchback",
    "Peano Serpent",
];

#[derive(Clone, Debug)]
struct FractalGeometry {
    generator_points: Vec<(f64, f64)>,
    baseline_points: [(f64, f64); 2],
    endpoint_docked: [bool; 2],
}

impl FractalGeometry {
    fn baseline(&self) -> [(f64, f64); 2] {
        self.baseline_points
    }
}

#[derive(Debug)]
struct InteractivePaintApp {
    depth: usize,
    depth_input: String,
    show_guides: bool,
    geometry: FractalGeometry,
    preset_id: usize,
}

impl Default for InteractivePaintApp {
    fn default() -> Self {
        Self {
            depth: 5,
            depth_input: "5".to_string(),
            show_guides: true,
            geometry: preset_geometry(0),
            preset_id: 0,
        }
    }
}

impl InteractivePaintApp {
    fn active_preset(&self) -> &'static PresetSpec {
        &PRESETS[self.preset_id]
    }

    fn apply_preset(&mut self, preset_id: usize) {
        self.preset_id = preset_id;
        self.geometry = preset_geometry(preset_id);
    }

    fn reset_current_preset(&mut self) {
        self.geometry = preset_geometry(self.preset_id);
    }

    fn set_depth(&mut self, depth: usize) {
        self.depth = depth;
        self.depth_input = self.depth.to_string();
    }

    fn branch_factor(&self) -> usize {
        self.geometry.generator_points.len().saturating_sub(1)
    }

    fn estimated_segments_label(&self) -> String {
        let branch_factor = self.branch_factor().max(1);
        let mut total = 1u128;
        for _ in 0..self.depth.min(64) {
            total = total.saturating_mul(branch_factor as u128);
        }
        if self.depth > 64 || total > 999_999_999_999 {
            "Segments: huge".to_string()
        } else {
            format!("Segments: {total}")
        }
    }
}

#[derive(Clone, Debug)]
enum CanvasAction {
    Geometry(FractalGeometry),
}

#[derive(Clone, Copy, Debug)]
struct SegmentJob {
    start: Point,
    end: Point,
    depth: usize,
}

struct BenchmarkStats {
    elapsed: Duration,
    expanded_jobs: u128,
    rasterized_lines: u128,
}

#[derive(Clone, Debug)]
enum DragTarget {
    EndpointEditor(usize),
    GeneratorPoint(usize),
    BasePoint(usize),
    GeneratorShape,
    Baseline,
}

struct CanvasWidget {
    geometry: FractalGeometry,
    depth: usize,
    show_guides: bool,
    drag_target: Option<DragTarget>,
    drag_anchor: Option<Point>,
    drag_start_geometry: Option<FractalGeometry>,
    endpoint_revealed: [bool; 2],
    last_click: Option<(usize, Instant, Point)>,
    pending_segments: VecDeque<SegmentJob>,
    raster_buffer: Vec<u8>,
}

impl Widget for CanvasWidget {
    type Action = CanvasAction;

    fn on_anim_frame(
        &mut self,
        ctx: &mut UpdateCtx<'_>,
        _: &mut PropertiesMut<'_>,
        _interval: u64,
    ) {
        if self.pending_segments.is_empty() {
            return;
        }

        let local_points = normalized_points_from_baseline(
            &self.geometry.generator_points,
            self.geometry.baseline(),
        );
        let deadline = Instant::now() + RENDER_BATCH_BUDGET;
        while Instant::now() < deadline {
            let Some(job) = self.pending_segments.pop_back() else {
                break;
            };
            if job.depth == 0 || segment_too_small(job.start, job.end) || local_points.len() < 2 {
                rasterize_line(
                    &mut self.raster_buffer,
                    CANVAS_WIDTH as usize,
                    CANVAS_HEIGHT as usize,
                    job.start,
                    job.end,
                    FRACTAL_COLOR,
                );
                continue;
            }
            for pair in local_points.windows(2).rev() {
                self.pending_segments.push_back(SegmentJob {
                    start: map_local(job.start, job.end, pair[0]),
                    end: map_local(job.start, job.end, pair[1]),
                    depth: job.depth - 1,
                });
            }
        }

        if !self.pending_segments.is_empty() {
            ctx.request_anim_frame();
        }
        ctx.request_paint_only();
    }

    fn register_children(&mut self, _: &mut RegisterCtx<'_>) {}

    fn accessibility_role(&self) -> masonry::accesskit::Role {
        masonry::accesskit::Role::Canvas
    }

    fn accessibility(
        &mut self,
        _: &mut AccessCtx<'_>,
        _: &PropertiesRef<'_>,
        node: &mut masonry::accesskit::Node,
    ) {
        node.set_label(
            "Interactive line fractal editor with draggable generator points and baseline.",
        );
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn layout(
        &mut self,
        _: &mut LayoutCtx<'_>,
        _: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        bc.constrain((CANVAS_WIDTH, CANVAS_HEIGHT))
    }

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        let now = Instant::now();
        if let Some((_, last_time, _)) = self.last_click {
            if now.duration_since(last_time) > DOUBLE_CLICK_THRESHOLD {
                self.last_click = None;
            }
        }

        match event {
            PointerEvent::Down(e) => {
                let local_pos = ctx.local_position(e.state.position);
                if let Some(base_index) = self.baseline_hit(local_pos) {
                    if self.geometry.endpoint_docked[base_index]
                        && matches!(
                            self.last_click,
                            Some((last_index, last_time, last_pos))
                                if last_index == base_index
                                    && now.duration_since(last_time) <= DOUBLE_CLICK_THRESHOLD
                                    && distance(local_pos, last_pos) <= HIT_RADIUS
                        )
                    {
                        self.endpoint_revealed[base_index] = !self.endpoint_revealed[base_index];
                        self.last_click = None;
                        ctx.request_paint_only();
                        return;
                    }
                    self.last_click = Some((base_index, now, local_pos));
                } else {
                    self.last_click = None;
                }
                if let Some(target) = self.hit_test(local_pos) {
                    ctx.capture_pointer();
                    self.drag_target = Some(target);
                    self.drag_anchor = Some(local_pos);
                    self.drag_start_geometry = Some(self.geometry.clone());
                }
            }
            PointerEvent::Move(e) => {
                if !ctx.is_active() {
                    return;
                }
                if let (Some(target), Some(anchor), Some(start_geometry)) = (
                    self.drag_target.as_ref(),
                    self.drag_anchor,
                    self.drag_start_geometry.as_ref(),
                ) {
                    let current = ctx.local_position(e.current.position);
                    let delta = current - anchor;
                    self.geometry = apply_drag(start_geometry, target, delta);
                    self.reset_render_progress();
                    ctx.request_anim_frame();
                    ctx.request_paint_only();
                }
            }
            PointerEvent::Up(_) => {
                if ctx.is_active() {
                    ctx.release_pointer();
                }
                if let Some(start_geometry) = self.drag_start_geometry.as_ref() {
                    if self.geometry.generator_points != start_geometry.generator_points {
                        ctx.submit_action::<CanvasAction>(CanvasAction::Geometry(
                            self.geometry.clone(),
                        ));
                    }
                }
                self.drag_target = None;
                self.drag_anchor = None;
                self.drag_start_geometry = None;
                ctx.request_paint_only();
            }
            PointerEvent::Cancel(_) => {
                if ctx.is_active() {
                    ctx.release_pointer();
                }
                self.drag_target = None;
                self.drag_anchor = None;
                self.drag_start_geometry = None;
                ctx.request_paint_only();
            }
            _ => {}
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::WidgetAdded) {
            self.reset_render_progress();
            ctx.request_anim_frame();
            ctx.request_paint_only();
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _: &PropertiesRef<'_>, scene: &mut Scene) {
        let rect = ctx.size().to_rect();
        fill(scene, &rect, Color::from_rgb8(246, 242, 233));

        let preview_rect = Rect::from_origin_size((24.0, 24.0), (872.0, 560.0));
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgba8(255, 255, 255, 235),
            None,
            &preview_rect,
        );
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            Color::from_rgb8(204, 188, 160),
            None,
            &preview_rect,
        );

        let image = ImageBrush::new(ImageData {
            data: self.raster_buffer.clone().into(),
            format: ImageFormat::Rgba8,
            alpha_type: ImageAlphaType::Alpha,
            width: CANVAS_WIDTH as u32,
            height: CANVAS_HEIGHT as u32,
        });
        scene.draw_image(&image, Affine::IDENTITY);

        paint_baseline_line(scene, self.geometry.baseline());
        if self.show_guides {
            paint_generator_guides(scene, &self.geometry, self.endpoint_revealed);
        }
        paint_baseline_handles(scene, &self.geometry, self.endpoint_revealed);
    }
}

impl CanvasWidget {
    fn reset_render_progress(&mut self) {
        self.raster_buffer.fill(0);
        self.pending_segments.clear();
        if self.geometry.generator_points.len() >= 2 {
            let baseline = self.geometry.baseline();
            self.pending_segments.push_back(SegmentJob {
                start: baseline[0].into(),
                end: baseline[1].into(),
                depth: self.depth,
            });
        }
    }

    fn hit_test(&self, point: Point) -> Option<DragTarget> {
        for base_index in 0..2 {
            if let Some(handle) = self.revealed_endpoint_handle(base_index) {
                if distance(point, handle) <= NESTED_ENDPOINT_RADIUS + NESTED_HIT_TOLERANCE {
                    return Some(DragTarget::EndpointEditor(base_index));
                }
            }
        }
        for (index, handle) in self.geometry.baseline().iter().enumerate() {
            let dist = distance(point, (*handle).into());
            let nested_claims_center =
                self.geometry.endpoint_docked[index] && self.endpoint_revealed[index];
            if dist <= BASELINE_RADIUS
                && (!nested_claims_center
                    || dist > NESTED_ENDPOINT_RADIUS + NESTED_HIT_TOLERANCE - 1.0)
            {
                return Some(DragTarget::BasePoint(index));
            }
        }
        for (index, handle) in self.geometry.generator_points.iter().enumerate() {
            if (index == 0 && self.geometry.endpoint_docked[0] && !self.endpoint_revealed[0])
                || (index == self.geometry.generator_points.len() - 1
                    && self.geometry.endpoint_docked[1]
                    && !self.endpoint_revealed[1])
            {
                continue;
            }
            if distance(point, (*handle).into()) <= HIT_RADIUS {
                return Some(DragTarget::GeneratorPoint(index));
            }
        }
        if point_near_polyline(point, &self.geometry.generator_points, HIT_RADIUS) {
            return Some(DragTarget::GeneratorShape);
        }
        let baseline = self.geometry.baseline();
        if point_to_segment_distance(point, baseline[0].into(), baseline[1].into()) <= HIT_RADIUS {
            return Some(DragTarget::Baseline);
        }
        None
    }

    fn baseline_hit(&self, point: Point) -> Option<usize> {
        self.geometry
            .baseline()
            .iter()
            .enumerate()
            .find_map(|(index, handle)| {
                (distance(point, (*handle).into()) <= HIT_RADIUS).then_some(index)
            })
    }

    fn revealed_endpoint_handle(&self, base_index: usize) -> Option<Point> {
        if !self.endpoint_revealed[base_index] {
            return None;
        }
        let point_index = if base_index == 0 {
            0
        } else {
            self.geometry.generator_points.len() - 1
        };
        let point = self.geometry.generator_points[point_index];
        Some(point.into())
    }
}

struct CanvasView;

impl ViewMarker for CanvasView {}

impl View<Edit<InteractivePaintApp>, (), ViewCtx> for CanvasView {
    type Element = Pod<CanvasWidget>;
    type ViewState = ();

    fn build(
        &self,
        ctx: &mut ViewCtx,
        app_state: Arg<'_, Edit<InteractivePaintApp>>,
    ) -> (Self::Element, Self::ViewState) {
        let widget = CanvasWidget {
            geometry: app_state.geometry.clone(),
            depth: app_state.depth,
            show_guides: app_state.show_guides,
            drag_target: None,
            drag_anchor: None,
            drag_start_geometry: None,
            endpoint_revealed: [false, false],
            last_click: None,
            pending_segments: VecDeque::new(),
            raster_buffer: vec![0; (CANVAS_WIDTH as usize) * (CANVAS_HEIGHT as usize) * 4],
        };
        (ctx.with_action_widget(|ctx| ctx.create_pod(widget)), ())
    }

    fn rebuild(
        &self,
        _prev: &Self,
        _: &mut Self::ViewState,
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: Arg<'_, Edit<InteractivePaintApp>>,
    ) {
        let geometry_changed = element.widget.geometry.generator_points
            != app_state.geometry.generator_points
            || element.widget.geometry.baseline_points != app_state.geometry.baseline_points
            || element.widget.geometry.endpoint_docked != app_state.geometry.endpoint_docked;

        let depth_changed = element.widget.depth != app_state.depth;
        let guides_changed = element.widget.show_guides != app_state.show_guides;

        if guides_changed {
            element.widget.show_guides = app_state.show_guides;
        }

        if geometry_changed || depth_changed {
            element.widget.geometry = app_state.geometry.clone();
            element.widget.depth = app_state.depth;
            element.widget.endpoint_revealed = [false, false];
            element.widget.reset_render_progress();
            element.ctx.request_anim_frame();
        } else {
            return;
        }
        element.ctx.request_paint_only();
    }

    fn teardown(
        &self,
        _: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
    ) {
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        _: &mut Self::ViewState,
        message: &mut MessageContext,
        _element: Mut<'_, Self::Element>,
        app_state: Arg<'_, Edit<InteractivePaintApp>>,
    ) -> MessageResult<()> {
        if message.take_first().is_some() {
            return MessageResult::Stale;
        }
        match message.take_message::<CanvasAction>() {
            Some(action) => {
                match *action {
                    CanvasAction::Geometry(ref geometry) => {
                        app_state.geometry = geometry.clone();
                    }
                }
                MessageResult::Action(())
            }
            None => MessageResult::Stale,
        }
    }
}

fn app_logic(data: &mut InteractivePaintApp) -> impl WidgetView<Edit<InteractivePaintApp>> + use<> {
    flex_col((
        label("Line Fractal Explorer").text_size(26.0),
        flex_row((
            label(format!("Preset: {}", data.active_preset().name)),
            label(format!("Depth: {}", data.depth)),
            label(data.estimated_segments_label()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap(18.0.px()),
        preset_section(data, "Classic Presets", PresetGroup::Classic),
        preset_section(data, "Experimental Presets", PresetGroup::Experiment),
        flex_row((text_button("Reset current preset", |data: &mut InteractivePaintApp| {
            data.reset_current_preset();
        }),))
        .cross_axis_alignment(CrossAxisAlignment::Center),
        flex_row((
            sized_box(label("Depth")).width(48.px()),
            text_button("-", |data: &mut InteractivePaintApp| {
                data.set_depth(data.depth.saturating_sub(1));
            }),
            sized_box(
                text_input(
                    data.depth_input.clone(),
                    |data: &mut InteractivePaintApp, value| {
                        let trimmed = value.trim();
                        if let Ok(parsed) = trimmed.parse::<usize>() {
                            data.set_depth(parsed);
                        } else {
                            data.depth_input = value;
                        }
                    },
                )
                .on_enter(|data: &mut InteractivePaintApp, value| {
                    if let Ok(parsed) = value.trim().parse::<usize>() {
                        data.set_depth(parsed);
                    } else {
                        data.depth_input = data.depth.to_string();
                    }
                })
                .text_alignment(TextAlign::Center),
            )
            .width(80.px()),
            text_button("+", |data: &mut InteractivePaintApp| {
                data.set_depth(data.depth.saturating_add(1));
            }),
            checkbox("Show guides", data.show_guides, |data: &mut InteractivePaintApp, checked| {
                data.show_guides = checked;
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap(12.0.px()),
        sized_box(CanvasView)
            .width(CANVAS_WIDTH.px())
            .height(CANVAS_HEIGHT.px()),
        label("Drag red handles or the red scaffold to shape the generator. Drag blue handles or the blue line to reposition the baseline."),
    ))
    .gap(14.0.px())
    .padding(20.0)
}

fn preset_section(
    data: &InteractivePaintApp,
    title: &'static str,
    group: PresetGroup,
) -> impl WidgetView<Edit<InteractivePaintApp>> + use<> {
    let mut grid_items = Vec::new();
    let mut count = 0usize;
    for (preset_id, preset) in PRESETS.iter().enumerate() {
        if preset.group != group {
            continue;
        }
        let is_active = data.preset_id == preset_id;
        let label_text = if is_active {
            format!("{} *", preset.name)
        } else {
            preset.name.to_string()
        };
        grid_items.push(
            text_button(label_text, move |data: &mut InteractivePaintApp| {
                data.apply_preset(preset_id);
            })
            .disabled(is_active)
            .grid_pos(
                (count as i32) % PRESET_GRID_COLUMNS,
                (count as i32) / PRESET_GRID_COLUMNS,
            ),
        );
        count += 1;
    }
    let rows = ((count as i32) + PRESET_GRID_COLUMNS - 1) / PRESET_GRID_COLUMNS;

    flex_col((
        label(title).text_size(16.0),
        sized_box(portal(
            grid(grid_items, PRESET_GRID_COLUMNS, rows.max(1)).spacing(8.0.px()),
        ))
        .height(PRESET_PANEL_HEIGHT.px())
        .width(860.0.px()),
    ))
    .gap(8.0.px())
}

fn apply_drag(
    start_geometry: &FractalGeometry,
    target: &DragTarget,
    delta: Vec2,
) -> FractalGeometry {
    let mut geometry = start_geometry.clone();
    let baseline = start_geometry.baseline();
    match *target {
        DragTarget::EndpointEditor(base_index) => {
            let point_index = if base_index == 0 {
                0
            } else {
                geometry.generator_points.len() - 1
            };
            geometry.generator_points[point_index].0 += delta.x;
            geometry.generator_points[point_index].1 += delta.y;
            if distance(
                geometry.generator_points[point_index].into(),
                geometry.baseline_points[base_index].into(),
            ) <= BASELINE_RADIUS
            {
                geometry.generator_points[point_index] = geometry.baseline_points[base_index];
                geometry.endpoint_docked[base_index] = true;
            } else {
                geometry.endpoint_docked[base_index] = false;
            }
        }
        DragTarget::GeneratorPoint(index) => {
            if index == 0 || index == geometry.generator_points.len() - 1 {
                let base_index = if index == 0 { 0 } else { 1 };
                geometry.generator_points[index].0 += delta.x;
                geometry.generator_points[index].1 += delta.y;
                geometry.endpoint_docked[base_index] = false;
                sync_docked_endpoints(&mut geometry);
            } else if let Some(point) = geometry.generator_points.get_mut(index) {
                point.0 += delta.x;
                point.1 += delta.y;
            }
        }
        DragTarget::BasePoint(index) => {
            let mut new_baseline = baseline;
            new_baseline[index] = (baseline[index].0 + delta.x, baseline[index].1 + delta.y);
            geometry.baseline_points = new_baseline;
            geometry.generator_points = transform_points_between_baselines(
                &start_geometry.generator_points,
                baseline,
                new_baseline,
            );
            sync_docked_endpoints(&mut geometry);
        }
        DragTarget::GeneratorShape | DragTarget::Baseline => {
            for point in &mut geometry.generator_points {
                point.0 += delta.x;
                point.1 += delta.y;
            }
            if matches!(*target, DragTarget::Baseline) {
                for point in &mut geometry.baseline_points {
                    point.0 += delta.x;
                    point.1 += delta.y;
                }
                sync_docked_endpoints(&mut geometry);
            } else {
                geometry.endpoint_docked = [false, false];
            }
        }
    }
    geometry
}

fn sync_docked_endpoints(geometry: &mut FractalGeometry) {
    if geometry.endpoint_docked[0] {
        geometry.generator_points[0] = geometry.baseline_points[0];
    }
    if geometry.endpoint_docked[1] {
        let last = geometry.generator_points.len() - 1;
        geometry.generator_points[last] = geometry.baseline_points[1];
    }
}

fn transform_points_between_baselines(
    points: &[(f64, f64)],
    old_baseline: [(f64, f64); 2],
    new_baseline: [(f64, f64); 2],
) -> Vec<(f64, f64)> {
    let local_points = normalized_points_from_baseline(points, old_baseline);
    local_points
        .into_iter()
        .map(|local| {
            let mapped = map_local(new_baseline[0].into(), new_baseline[1].into(), local);
            (mapped.x, mapped.y)
        })
        .collect()
}

fn geometry_from_points(points: &[(f64, f64)]) -> FractalGeometry {
    let generator_points = points.to_vec();
    let baseline_points = [
        generator_points.first().copied().unwrap_or((0.0, 0.0)),
        generator_points.last().copied().unwrap_or((1.0, 0.0)),
    ];
    FractalGeometry {
        generator_points,
        baseline_points,
        endpoint_docked: [true, true],
    }
}

fn preset_geometry(preset_id: usize) -> FractalGeometry {
    let preset = PRESETS.get(preset_id).copied().unwrap_or(PRESETS[0]);
    match preset.generator {
        GeneratorKind::Points(points) => geometry_from_points(points),
        GeneratorKind::Turtle {
            commands,
            angle_deg,
        } => geometry_from_turtle(commands, angle_deg),
    }
}

fn normalized_points_from_baseline(
    points: &[(f64, f64)],
    baseline: [(f64, f64); 2],
) -> Vec<(f64, f64)> {
    if points.len() < 2 {
        return vec![(0.0, 0.0), (1.0, 0.0)];
    }

    let start = Point::new(baseline[0].0, baseline[0].1);
    let end = Point::new(baseline[1].0, baseline[1].1);
    let line = end - start;
    let length_sq = line.hypot2();
    if length_sq <= f64::EPSILON {
        return vec![(0.0, 0.0), (1.0, 0.0)];
    }

    points
        .iter()
        .map(|&(x, y)| {
            let offset = Point::new(x, y) - start;
            let local_x = offset.dot(line) / length_sq;
            let local_y = cross(line, offset) / length_sq;
            (local_x, local_y)
        })
        .collect()
}

fn map_local(start: Point, end: Point, local: (f64, f64)) -> Point {
    let direction = end - start;
    let perpendicular = Vec2::new(-direction.y, direction.x);
    start + direction * local.0 + perpendicular * local.1
}

fn geometry_from_turtle(commands: &str, angle_deg: f64) -> FractalGeometry {
    let raw_points = turtle_points(commands, angle_deg);
    let baseline = [raw_points[0], *raw_points.last().unwrap()];
    let local_points = normalized_points_from_baseline(&raw_points, baseline);

    let (mut min_x, mut max_x, mut min_y, mut max_y) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for &(x, y) in &local_points {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }

    let local_width = (max_x - min_x).max(1.0);
    let local_height = (max_y - min_y).max(0.3);
    let scale = (PRESET_TARGET_WIDTH / local_width)
        .min(PRESET_TARGET_HEIGHT / local_height)
        .min(360.0);

    let mut mapped_points: Vec<(f64, f64)> = local_points
        .iter()
        .map(|&(x, y)| (x * scale, y * scale))
        .collect();

    let (mut mapped_min_x, mut mapped_max_x, mut mapped_min_y, mut mapped_max_y) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for &(x, y) in &mapped_points {
        mapped_min_x = mapped_min_x.min(x);
        mapped_max_x = mapped_max_x.max(x);
        mapped_min_y = mapped_min_y.min(y);
        mapped_max_y = mapped_max_y.max(y);
    }
    let offset_x = PRESET_TARGET_CENTER.0 - (mapped_min_x + mapped_max_x) * 0.5;
    let offset_y = PRESET_TARGET_CENTER.1 - (mapped_min_y + mapped_max_y) * 0.5;
    for point in &mut mapped_points {
        point.0 += offset_x;
        point.1 += offset_y;
    }

    geometry_from_points(&mapped_points)
}

fn turtle_points(commands: &str, angle_deg: f64) -> Vec<(f64, f64)> {
    let turn = angle_deg.to_radians();
    let mut heading = 0.0f64;
    let mut points = vec![(0.0, 0.0)];
    for ch in commands.chars() {
        match ch {
            '+' => heading += turn,
            '-' => heading -= turn,
            ' ' | '\n' | '\t' => {}
            _ => {
                let (x, y) = points.last().copied().unwrap();
                points.push((x + heading.cos(), y + heading.sin()));
            }
        }
    }

    if points.len() < 2 || distance(points[0].into(), (*points.last().unwrap()).into()) <= 1e-6 {
        points.push((1.0, 0.0));
    }
    points
}

fn segment_too_small(start: Point, end: Point) -> bool {
    (end - start).hypot2() <= MIN_SEGMENT_LENGTH * MIN_SEGMENT_LENGTH
}

fn paint_generator_guides(
    scene: &mut Scene,
    geometry: &FractalGeometry,
    endpoint_revealed: [bool; 2],
) {
    let points = &geometry.generator_points;
    if points.len() >= 2 {
        let mut path = BezPath::new();
        path.move_to(Point::new(points[0].0, points[0].1));
        for point in &points[1..] {
            path.line_to(Point::new(point.0, point.1));
        }
        scene.stroke(
            &Stroke::new(2.0),
            Affine::IDENTITY,
            Color::from_rgb8(186, 57, 39),
            None,
            &path,
        );
    }

    for (index, &(x, y)) in points.iter().enumerate() {
        let is_endpoint = index == 0 || index == points.len() - 1;
        let slot = if index == 0 { 0 } else { 1 };
        if is_endpoint && geometry.endpoint_docked[slot] {
            paint_docked_endpoint(scene, Point::new(x, y), endpoint_revealed[slot]);
        } else {
            fill(
                scene,
                &Circle::new((x, y), HANDLE_RADIUS),
                Color::from_rgb8(215, 83, 63),
            );
        }
    }
}

fn paint_baseline_line(scene: &mut Scene, base_line: [(f64, f64); 2]) {
    scene.stroke(
        &Stroke::new(3.0),
        Affine::IDENTITY,
        Color::from_rgb8(55, 108, 171),
        None,
        &Line::new(base_line[0], base_line[1]),
    );
}

fn paint_baseline_handles(
    scene: &mut Scene,
    geometry: &FractalGeometry,
    endpoint_revealed: [bool; 2],
) {
    for (index, point) in geometry.baseline().into_iter().enumerate() {
        if geometry.endpoint_docked[index] {
            paint_docked_endpoint(scene, point.into(), endpoint_revealed[index]);
        } else {
            fill(
                scene,
                &Circle::new(point, BASELINE_RADIUS),
                Color::from_rgb8(87, 140, 201),
            );
        }
    }
}

fn point_near_polyline(point: Point, polyline: &[(f64, f64)], max_distance: f64) -> bool {
    polyline.windows(2).any(|segment| {
        point_to_segment_distance(point, segment[0].into(), segment[1].into()) <= max_distance
    })
}

fn point_to_segment_distance(point: Point, start: Point, end: Point) -> f64 {
    let line = end - start;
    let length_sq = line.hypot2();
    if length_sq <= f64::EPSILON {
        return distance(point, start);
    }
    let t = ((point - start).dot(line) / length_sq).clamp(0.0, 1.0);
    let projection = start + line * t;
    distance(point, projection)
}

fn distance(a: Point, b: Point) -> f64 {
    (a - b).hypot()
}

fn cross(a: Vec2, b: Vec2) -> f64 {
    a.x * b.y - a.y * b.x
}

fn paint_docked_endpoint(scene: &mut Scene, center: Point, red_active: bool) {
    let outer_radius = BASELINE_RADIUS;
    let inner_radius = NESTED_ENDPOINT_RADIUS;
    let blue = Color::from_rgb8(87, 140, 201);
    let red = Color::from_rgb8(184, 49, 90);

    let (outer_color, inner_color, ring_color) = if red_active {
        (blue, red, Color::from_rgb8(230, 238, 250))
    } else {
        (red, blue, Color::from_rgb8(250, 227, 235))
    };

    fill(scene, &Circle::new(center, outer_radius), outer_color);
    scene.stroke(
        &Stroke::new(1.0),
        Affine::IDENTITY,
        ring_color,
        None,
        &Circle::new(center, outer_radius),
    );
    fill(scene, &Circle::new(center, inner_radius), inner_color);
}

fn rasterize_line(
    buffer: &mut [u8],
    width: usize,
    height: usize,
    start: Point,
    end: Point,
    color: [u8; 4],
) {
    if color[3] == u8::MAX {
        rasterize_line_opaque(buffer, width, height, start, end, color);
        return;
    }

    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let steps = dx.abs().max(dy.abs()).ceil() as usize;
    if steps == 0 {
        blend_pixel(
            buffer,
            width,
            height,
            start.x.round() as isize,
            start.y.round() as isize,
            color,
        );
        return;
    }

    for step in 0..=steps {
        let t = step as f64 / steps as f64;
        let x = start.x + dx * t;
        let y = start.y + dy * t;
        blend_pixel(
            buffer,
            width,
            height,
            x.round() as isize,
            y.round() as isize,
            color,
        );
    }
}

fn rasterize_line_opaque(
    buffer: &mut [u8],
    width: usize,
    height: usize,
    start: Point,
    end: Point,
    color: [u8; 4],
) {
    let mut x0 = start.x.round() as isize;
    let mut y0 = start.y.round() as isize;
    let x1 = end.x.round() as isize;
    let y1 = end.y.round() as isize;

    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;

    loop {
        set_pixel_opaque(buffer, width, height, x0, y0, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let twice_error = error * 2;
        if twice_error >= dy {
            error += dy;
            x0 += sx;
        }
        if twice_error <= dx {
            error += dx;
            y0 += sy;
        }
    }
}

fn set_pixel_opaque(
    buffer: &mut [u8],
    width: usize,
    height: usize,
    x: isize,
    y: isize,
    color: [u8; 4],
) {
    if x < 0 || y < 0 || x >= width as isize || y >= height as isize {
        return;
    }
    let idx = ((y as usize * width) + x as usize) * 4;
    buffer[idx] = color[0];
    buffer[idx + 1] = color[1];
    buffer[idx + 2] = color[2];
    buffer[idx + 3] = color[3];
}

fn blend_pixel(buffer: &mut [u8], width: usize, height: usize, x: isize, y: isize, color: [u8; 4]) {
    if x < 0 || y < 0 || x >= width as isize || y >= height as isize {
        return;
    }
    let idx = ((y as usize * width) + x as usize) * 4;
    let alpha = color[3] as f32 / 255.0;
    let inv_alpha = 1.0 - alpha;
    buffer[idx] = (color[0] as f32 * alpha + buffer[idx] as f32 * inv_alpha).round() as u8;
    buffer[idx + 1] = (color[1] as f32 * alpha + buffer[idx + 1] as f32 * inv_alpha).round() as u8;
    buffer[idx + 2] = (color[2] as f32 * alpha + buffer[idx + 2] as f32 * inv_alpha).round() as u8;
    buffer[idx + 3] =
        ((alpha + (buffer[idx + 3] as f32 / 255.0) * inv_alpha) * 255.0).round() as u8;
}

fn benchmark_rasterize(depth: usize, geometry: &FractalGeometry) -> BenchmarkStats {
    let width = CANVAS_WIDTH as usize;
    let height = CANVAS_HEIGHT as usize;
    let mut buffer = vec![0u8; width * height * 4];
    let baseline = geometry.baseline();

    let local_points = normalized_points_from_baseline(&geometry.generator_points, baseline);
    let mut pending_segments: Vec<SegmentJob> = Vec::new();
    if geometry.generator_points.len() >= 2 {
        pending_segments.push(SegmentJob {
            start: baseline[0].into(),
            end: baseline[1].into(),
            depth,
        });
    }

    let start = Instant::now();
    let mut expanded_jobs = 0u128;
    let mut rasterized_lines = 0u128;
    while let Some(job) = pending_segments.pop() {
        expanded_jobs += 1;
        if job.depth == 0 || segment_too_small(job.start, job.end) || local_points.len() < 2 {
            rasterized_lines += 1;
            rasterize_line(
                &mut buffer,
                width,
                height,
                job.start,
                job.end,
                FRACTAL_COLOR,
            );
        } else {
            for pair in local_points.windows(2).rev() {
                pending_segments.push(SegmentJob {
                    start: map_local(job.start, job.end, pair[0]),
                    end: map_local(job.start, job.end, pair[1]),
                    depth: job.depth - 1,
                });
            }
        }
    }
    BenchmarkStats {
        elapsed: start.elapsed(),
        expanded_jobs,
        rasterized_lines,
    }
}

fn count_leaf_segments(depth: usize, branch_factor: usize) -> u128 {
    let mut total = 1u128;
    for _ in 0..depth {
        total = total.saturating_mul(branch_factor as u128);
    }
    total
}

fn count_total_jobs(depth: usize, branch_factor: usize) -> u128 {
    let mut total = 0u128;
    let mut level = 1u128;
    for _ in 0..=depth {
        total = total.saturating_add(level);
        level = level.saturating_mul(branch_factor as u128);
    }
    total
}

fn format_u128(value: u128) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    let len = digits.len();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push('_');
        }
        out.push(ch);
    }
    out
}

fn preset_id_by_name(name: &str) -> Option<usize> {
    PRESETS.iter().position(|preset| preset.name == name)
}

fn benchmark_depths(branch_factor: usize) -> &'static [usize] {
    if branch_factor >= 7 {
        &[3, 4, 5, 6, 7, 8]
    } else if branch_factor >= 5 {
        &[4, 5, 6, 7, 8, 10]
    } else {
        &[5, 8, 10, 12, 14, 16]
    }
}

fn run_benchmark() {
    println!("\n=== Rust Fractal Rasterization Benchmark ===\n");
    println!("Canvas size: {}x{}", CANVAS_WIDTH, CANVAS_HEIGHT);
    println!("Presets: {}\n", BENCHMARK_PRESET_NAMES.join(", "));

    for preset_name in BENCHMARK_PRESET_NAMES {
        let Some(preset_id) = preset_id_by_name(preset_name) else {
            continue;
        };
        let geometry = preset_geometry(preset_id);
        let branch_factor = geometry.generator_points.len().saturating_sub(1).max(1);
        println!(
            "{} (branch factor {}, {} control points)",
            preset_name,
            branch_factor,
            geometry.generator_points.len()
        );

        for &depth in benchmark_depths(branch_factor) {
            let theoretical_lines = count_leaf_segments(depth, branch_factor);
            let theoretical_jobs = count_total_jobs(depth, branch_factor);

            let iterations = 5;
            let mut times = Vec::with_capacity(iterations);
            let mut expanded_jobs = 0u128;
            let mut rasterized_lines = 0u128;
            for _ in 0..iterations {
                let stats = benchmark_rasterize(depth, &geometry);
                expanded_jobs = stats.expanded_jobs;
                rasterized_lines = stats.rasterized_lines;
                times.push(stats.elapsed);
            }

            let avg = times.iter().sum::<Duration>() / iterations as u32;
            let lines_per_sec = rasterized_lines as f64 / avg.as_secs_f64();
            let jobs_per_sec = expanded_jobs as f64 / avg.as_secs_f64();

            println!(
                "  depth {:>2}: theoretical lines={}, jobs={} | actual lines={}, jobs={} | {:.0} lines/sec, {:.0} jobs/sec",
                depth,
                format_u128(theoretical_lines),
                format_u128(theoretical_jobs),
                format_u128(rasterized_lines),
                format_u128(expanded_jobs),
                lines_per_sec,
                jobs_per_sec
            );
        }
        println!();
    }
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = InteractivePaintApp::default();
    let app = Xilem::new_simple(
        data,
        app_logic,
        WindowOptions::new("Interactive Line Fractal")
            .with_initial_inner_size(LogicalSize::new(980.0, 860.0)),
    );
    app.run_in(event_loop)
}

fn main() -> Result<(), EventLoopError> {
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--benchmark".to_string()) {
        run_benchmark();
        return Ok(());
    }
    run(EventLoop::with_user_event())
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let mut event_loop = EventLoop::with_user_event();
    event_loop.with_android_app(app);

    run(event_loop).expect("Can create app");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_points_can_use_baseline_distinct_from_red_endpoints() {
        let points = vec![(10.0, 10.0), (20.0, 30.0), (40.0, 10.0)];
        let baseline = [(0.0, 10.0), (50.0, 10.0)];
        let local = normalized_points_from_baseline(&points, baseline);

        assert!((local[0].0 - 0.2).abs() < 1e-9);
        assert!((local[0].1 - 0.0).abs() < 1e-9);
        assert!((local[2].0 - 0.8).abs() < 1e-9);
        assert!((local[2].1 - 0.0).abs() < 1e-9);
    }

    #[test]
    fn map_local_respects_segment_frame() {
        let start = Point::new(5.0, 7.0);
        let end = Point::new(25.0, 7.0);
        let mapped = map_local(start, end, (0.5, 0.25));

        assert!((mapped.x - 15.0).abs() < 1e-9);
        assert!((mapped.y - 12.0).abs() < 1e-9);
    }

    #[test]
    fn dragging_generator_shape_translates_all_generator_points_only() {
        let geometry = geometry_from_points(KOCH_POINTS);
        let moved = apply_drag(
            &geometry,
            &DragTarget::GeneratorShape,
            Vec2::new(10.0, -5.0),
        );

        assert_eq!(
            moved.generator_points[0],
            (
                geometry.generator_points[0].0 + 10.0,
                geometry.generator_points[0].1 - 5.0
            )
        );
        assert_eq!(
            moved.generator_points[moved.generator_points.len() - 1],
            (
                geometry.generator_points[geometry.generator_points.len() - 1].0 + 10.0,
                geometry.generator_points[geometry.generator_points.len() - 1].1 - 5.0
            )
        );
    }

    #[test]
    fn dragging_display_endpoint_transforms_interior_control_points() {
        let geometry = geometry_from_points(KOCH_POINTS);
        let moved = apply_drag(&geometry, &DragTarget::BasePoint(0), Vec2::new(20.0, 30.0));

        assert_eq!(
            moved.baseline()[0],
            (
                geometry.baseline()[0].0 + 20.0,
                geometry.baseline()[0].1 + 30.0
            )
        );
        assert_ne!(moved.generator_points[1], geometry.generator_points[1]);
    }

    #[test]
    fn preset_library_has_balanced_groups_and_valid_geometry() {
        let classic_count = PRESETS
            .iter()
            .filter(|preset| preset.group == PresetGroup::Classic)
            .count();
        let experiment_count = PRESETS
            .iter()
            .filter(|preset| preset.group == PresetGroup::Experiment)
            .count();

        assert!((10..=15).contains(&classic_count));
        assert!((10..=15).contains(&experiment_count));

        for (preset_id, preset) in PRESETS.iter().enumerate() {
            let geometry = preset_geometry(preset_id);
            assert!(
                geometry.generator_points.len() >= 2,
                "{} should have at least two points",
                preset.name
            );
            assert!(
                distance(
                    geometry.baseline_points[0].into(),
                    geometry.baseline_points[1].into()
                ) > 1.0,
                "{} should have a non-degenerate baseline",
                preset.name
            );
        }
    }
}
