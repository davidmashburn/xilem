// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Interactive line fractal explorer built with a custom paint widget.

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
use xilem::view::{checkbox, flex_col, flex_row, label, sized_box, slider, text_button};
use xilem::{Color, Pod, ViewCtx, WidgetView, WindowOptions, Xilem};
use xilem_core::{Edit, MessageResult};

const CANVAS_WIDTH: f64 = 920.0;
const CANVAS_HEIGHT: f64 = 620.0;
const HANDLE_RADIUS: f64 = 8.0;
const HIT_RADIUS: f64 = 14.0;
const MIN_SEGMENT_LENGTH: f64 = 2.5;
const MAX_SEGMENTS: usize = 40_000;

#[derive(Clone, Debug)]
struct FractalGeometry {
    generator_points: Vec<(f64, f64)>,
    base_line: [(f64, f64); 2],
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
            base_line: [(120.0, 490.0), (800.0, 490.0)],
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
            base_line: [(120.0, 500.0), (810.0, 470.0)],
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
            base_line: [(110.0, 505.0), (820.0, 505.0)],
        }
    }
}

#[derive(Debug)]
struct InteractivePaintApp {
    depth: usize,
    show_guides: bool,
    geometry: FractalGeometry,
    preset_name: &'static str,
}

impl Default for InteractivePaintApp {
    fn default() -> Self {
        Self {
            depth: 5,
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

    fn branch_factor(&self) -> usize {
        self.geometry.generator_points.len().saturating_sub(1)
    }

    fn estimated_segments(&self) -> usize {
        let mut total = 1usize;
        let branch_factor = self.branch_factor().max(1);
        for _ in 0..self.depth {
            total = total.saturating_mul(branch_factor);
            if total >= MAX_SEGMENTS {
                return MAX_SEGMENTS;
            }
        }
        total
    }
}

#[derive(Clone, Debug)]
enum CanvasAction {
    Geometry(FractalGeometry),
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
}

impl Widget for CanvasWidget {
    type Action = CanvasAction;

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
                    ctx.submit_action::<CanvasAction>(CanvasAction::Geometry(
                        self.geometry.clone(),
                    ));
                    ctx.request_paint_only();
                }
            }
            PointerEvent::Up(_) | PointerEvent::Cancel(_) => {
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

        let local_points = normalized_points(&self.geometry.generator_points);
        let mut remaining_segments = MAX_SEGMENTS;
        draw_fractal(
            scene,
            &local_points,
            self.geometry.base_line[0].into(),
            self.geometry.base_line[1].into(),
            self.depth,
            &mut remaining_segments,
        );

        if self.show_guides {
            paint_generator_guides(scene, &self.geometry.generator_points);
            paint_baseline(scene, self.geometry.base_line);
        } else {
            paint_baseline(scene, self.geometry.base_line);
        }
    }
}

impl CanvasWidget {
    fn hit_test(&self, point: Point) -> Option<DragTarget> {
        for (index, handle) in self.geometry.generator_points.iter().enumerate() {
            if distance(point, (*handle).into()) <= HIT_RADIUS {
                return Some(DragTarget::GeneratorPoint(index));
            }
        }
        for (index, handle) in self.geometry.base_line.iter().enumerate() {
            if distance(point, (*handle).into()) <= HIT_RADIUS {
                return Some(DragTarget::BasePoint(index));
            }
        }
        if point_near_polyline(point, &self.geometry.generator_points, HIT_RADIUS) {
            return Some(DragTarget::GeneratorShape);
        }
        if point_to_segment_distance(
            point,
            self.geometry.base_line[0].into(),
            self.geometry.base_line[1].into(),
        ) <= HIT_RADIUS
        {
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
        element.widget.geometry = app_state.geometry.clone();
        element.widget.depth = app_state.depth;
        element.widget.show_guides = app_state.show_guides;
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
    let estimated_segments = data.estimated_segments();
    let segment_label = if estimated_segments >= MAX_SEGMENTS {
        format!("Segments: {}+", MAX_SEGMENTS)
    } else {
        format!("Segments: {estimated_segments}")
    };

    flex_col((
        label("Line Fractal Explorer").text_size(26.0),
        flex_row((
            label(format!("Preset: {}", data.preset_name)),
            label(format!("Depth: {}", data.depth)),
            label(segment_label),
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
            slider(0.0, 7.0, data.depth as f64, |data: &mut InteractivePaintApp, value| {
                data.depth = value.round().clamp(0.0, 7.0) as usize;
            })
            .step(1.0),
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
    match *target {
        DragTarget::GeneratorPoint(index) => {
            if let Some(point) = geometry.generator_points.get_mut(index) {
                point.0 += delta.x;
                point.1 += delta.y;
            }
        }
        DragTarget::BasePoint(index) => {
            geometry.base_line[index].0 += delta.x;
            geometry.base_line[index].1 += delta.y;
        }
        DragTarget::GeneratorShape => {
            for point in &mut geometry.generator_points {
                point.0 += delta.x;
                point.1 += delta.y;
            }
        }
        DragTarget::Baseline => {
            for point in &mut geometry.base_line {
                point.0 += delta.x;
                point.1 += delta.y;
            }
        }
    }
    geometry
}

fn normalized_points(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    if points.len() < 2 {
        return vec![(0.0, 0.0), (1.0, 0.0)];
    }

    let start = Point::new(points[0].0, points[0].1);
    let end = Point::new(points[points.len() - 1].0, points[points.len() - 1].1);
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

fn draw_fractal(
    scene: &mut Scene,
    local_points: &[(f64, f64)],
    start: Point,
    end: Point,
    depth: usize,
    remaining_segments: &mut usize,
) {
    if *remaining_segments == 0 {
        return;
    }

    if depth == 0 || distance(start, end) <= MIN_SEGMENT_LENGTH || local_points.len() < 2 {
        scene.stroke(
            &Stroke::new(1.15),
            Affine::IDENTITY,
            Color::from_rgb8(28, 96, 99),
            None,
            &Line::new(start, end),
        );
        *remaining_segments = remaining_segments.saturating_sub(1);
        return;
    }

    for pair in local_points.windows(2) {
        if *remaining_segments == 0 {
            return;
        }
        let child_start = map_local(start, end, pair[0]);
        let child_end = map_local(start, end, pair[1]);
        draw_fractal(
            scene,
            local_points,
            child_start,
            child_end,
            depth - 1,
            remaining_segments,
        );
    }
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
        assert_eq!(moved.base_line, geometry.base_line);
    }
}
