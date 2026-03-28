// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Interactive line fractal explorer built with a custom paint widget.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use masonry::core::*;
use masonry::dpi::LogicalSize;
use masonry::peniko::Fill;
use masonry::properties::types::{AsUnit, CrossAxisAlignment};
use masonry::util::fill;
use masonry::vello::kurbo::{Affine, BezPath, Circle, Line, Point, Rect, Size, Stroke, Vec2};
use masonry::vello::Scene;
use masonry_winit::app::{EventLoop, EventLoopBuilder};
use winit::error::EventLoopError;
use xilem::core::{Arg, MessageContext, Mut, View, ViewMarker};
use xilem::style::Style;
use xilem::view::{checkbox, flex_col, flex_row, label, sized_box, text_button, text_input};
use xilem::{Color, Pod, TextAlign, ViewCtx, WidgetView, WindowOptions, Xilem};
use xilem_core::{Edit, MessageResult};

const CANVAS_WIDTH: f64 = 920.0;
const CANVAS_HEIGHT: f64 = 620.0;
const HANDLE_RADIUS: f64 = 8.0;
const HIT_RADIUS: f64 = 14.0;
const MIN_SEGMENT_LENGTH: f64 = 2.5;
const RENDER_BATCH_BUDGET: Duration = Duration::from_millis(5);

#[derive(Clone, Debug)]
struct FractalGeometry {
    generator_points: Vec<(f64, f64)>,
}

impl FractalGeometry {
    fn koch() -> Self {
        Self {
            generator_points: vec![
                (160.0, 150.0),
                (260.0, 150.0),
                (310.0, 85.0),
                (360.0, 150.0),
                (460.0, 150.0),
            ],
        }
    }

    fn lightning() -> Self {
        Self {
            generator_points: vec![
                (170.0, 150.0),
                (235.0, 125.0),
                (285.0, 190.0),
                (350.0, 110.0),
                (415.0, 165.0),
                (480.0, 145.0),
            ],
        }
    }

    fn canyon() -> Self {
        Self {
            generator_points: vec![
                (170.0, 155.0),
                (245.0, 155.0),
                (285.0, 215.0),
                (350.0, 95.0),
                (415.0, 215.0),
                (455.0, 155.0),
                (530.0, 155.0),
            ],
        }
    }

    fn baseline(&self) -> [(f64, f64); 2] {
        [
            self.generator_points[0],
            self.generator_points[self.generator_points.len() - 1],
        ]
    }
}

#[derive(Debug)]
struct InteractivePaintApp {
    depth: usize,
    depth_input: String,
    show_guides: bool,
    geometry: FractalGeometry,
    preset_name: &'static str,
}

impl Default for InteractivePaintApp {
    fn default() -> Self {
        Self {
            depth: 5,
            depth_input: "5".to_string(),
            show_guides: true,
            geometry: FractalGeometry::koch(),
            preset_name: "Koch-ish",
        }
    }
}

impl InteractivePaintApp {
    fn set_preset(&mut self, preset_name: &'static str, geometry: FractalGeometry) {
        self.preset_name = preset_name;
        self.geometry = geometry;
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

#[derive(Clone, Debug)]
enum DragTarget {
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
    pending_segments: VecDeque<SegmentJob>,
    rendered_segments: Vec<(Point, Point)>,
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

        let local_points = normalized_points(&self.geometry.generator_points);
        let deadline = Instant::now() + RENDER_BATCH_BUDGET;
        while Instant::now() < deadline {
            let Some(job) = self.pending_segments.pop_back() else {
                break;
            };
            if job.depth == 0
                || distance(job.start, job.end) <= MIN_SEGMENT_LENGTH
                || local_points.len() < 2
            {
                self.rendered_segments.push((job.start, job.end));
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
        match event {
            PointerEvent::Down(e) => {
                let local_pos = ctx.local_position(e.state.position);
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

        for &(start, end) in &self.rendered_segments {
            scene.stroke(
                &Stroke::new(1.15),
                Affine::IDENTITY,
                Color::from_rgb8(28, 96, 99),
                None,
                &Line::new(start, end),
            );
        }

        if self.show_guides {
            paint_generator_guides(scene, &self.geometry.generator_points);
            paint_baseline(scene, self.geometry.baseline());
        } else {
            paint_baseline(scene, self.geometry.baseline());
        }
    }
}

impl CanvasWidget {
    fn reset_render_progress(&mut self) {
        self.rendered_segments.clear();
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
        for (index, handle) in self.geometry.generator_points.iter().enumerate() {
            if distance(point, (*handle).into()) <= HIT_RADIUS {
                return Some(DragTarget::GeneratorPoint(index));
            }
        }
        for (index, handle) in self.geometry.baseline().iter().enumerate() {
            if distance(point, (*handle).into()) <= HIT_RADIUS {
                return Some(DragTarget::BasePoint(index));
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
            pending_segments: VecDeque::new(),
            rendered_segments: Vec::new(),
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
        let needs_reset = element.widget.geometry.generator_points
            != app_state.geometry.generator_points
            || element.widget.depth != app_state.depth;
        element.widget.geometry = app_state.geometry.clone();
        element.widget.depth = app_state.depth;
        element.widget.show_guides = app_state.show_guides;
        if needs_reset {
            element.widget.reset_render_progress();
            element.ctx.request_anim_frame();
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
            label(format!("Preset: {}", data.preset_name)),
            label(format!("Depth: {}", data.depth)),
            label(data.estimated_segments_label()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap(18.0.px()),
        flex_row((
            text_button("Koch-ish", |data: &mut InteractivePaintApp| {
                data.set_preset("Koch-ish", FractalGeometry::koch());
            }),
            text_button("Lightning", |data: &mut InteractivePaintApp| {
                data.set_preset("Lightning", FractalGeometry::lightning());
            }),
            text_button("Canyon", |data: &mut InteractivePaintApp| {
                data.set_preset("Canyon", FractalGeometry::canyon());
            }),
            text_button("Reset", |data: &mut InteractivePaintApp| {
                let preset = data.preset_name;
                let geometry = match preset {
                    "Lightning" => FractalGeometry::lightning(),
                    "Canyon" => FractalGeometry::canyon(),
                    _ => FractalGeometry::koch(),
                };
                data.set_preset(preset, geometry);
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap(12.0.px()),
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

fn apply_drag(
    start_geometry: &FractalGeometry,
    target: &DragTarget,
    delta: Vec2,
) -> FractalGeometry {
    let mut geometry = start_geometry.clone();
    let baseline = start_geometry.baseline();
    match *target {
        DragTarget::GeneratorPoint(index) => {
            if index == 0 || index == geometry.generator_points.len() - 1 {
                let mut new_baseline = baseline;
                new_baseline[if index == 0 { 0 } else { 1 }] = (
                    baseline[if index == 0 { 0 } else { 1 }].0 + delta.x,
                    baseline[if index == 0 { 0 } else { 1 }].1 + delta.y,
                );
                geometry.generator_points = transform_points_between_baselines(
                    &start_geometry.generator_points,
                    baseline,
                    new_baseline,
                );
            } else if let Some(point) = geometry.generator_points.get_mut(index) {
                point.0 += delta.x;
                point.1 += delta.y;
            }
        }
        DragTarget::BasePoint(index) => {
            let mut new_baseline = baseline;
            new_baseline[index] = (baseline[index].0 + delta.x, baseline[index].1 + delta.y);
            geometry.generator_points = transform_points_between_baselines(
                &start_geometry.generator_points,
                baseline,
                new_baseline,
            );
        }
        DragTarget::GeneratorShape | DragTarget::Baseline => {
            for point in &mut geometry.generator_points {
                point.0 += delta.x;
                point.1 += delta.y;
            }
        }
    }
    geometry
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

fn normalized_points(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    if points.len() < 2 {
        return vec![(0.0, 0.0), (1.0, 0.0)];
    }

    normalized_points_from_baseline(points, [points[0], points[points.len() - 1]])
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

fn paint_generator_guides(scene: &mut Scene, points: &[(f64, f64)]) {
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

    for &(x, y) in points {
        fill(
            scene,
            &Circle::new((x, y), HANDLE_RADIUS),
            Color::from_rgb8(215, 83, 63),
        );
    }
}

fn paint_baseline(scene: &mut Scene, base_line: [(f64, f64); 2]) {
    scene.stroke(
        &Stroke::new(3.0),
        Affine::IDENTITY,
        Color::from_rgb8(55, 108, 171),
        None,
        &Line::new(base_line[0], base_line[1]),
    );
    for point in base_line {
        fill(
            scene,
            &Circle::new(point, HANDLE_RADIUS + 1.5),
            Color::from_rgb8(87, 140, 201),
        );
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
    fn normalized_points_keep_endpoints_at_unit_interval() {
        let points = vec![(10.0, 10.0), (20.0, 30.0), (40.0, 10.0)];
        let local = normalized_points(&points);

        assert!((local[0].0 - 0.0).abs() < 1e-9);
        assert!((local[0].1 - 0.0).abs() < 1e-9);
        assert!((local[2].0 - 1.0).abs() < 1e-9);
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
        let geometry = FractalGeometry::koch();
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
        let geometry = FractalGeometry::koch();
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
}
