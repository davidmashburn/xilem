// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Interactive custom paint example with clickable circles

use masonry::core::*;
use masonry::dpi::LogicalSize;
use masonry::properties::types::AsUnit;
use masonry::util::{fill, stroke};
use masonry_winit::app::{EventLoop, EventLoopBuilder};
use vello::{kurbo::{BezPath, Line}, Scene};
use winit::error::EventLoopError;
use xilem::core::{Arg, MessageContext, Mut, View, ViewArgument, ViewMarker};
use xilem::view::{flex_col, label, sized_box};
use xilem::{Color, Pod, ViewCtx, WidgetView, WindowOptions, Xilem};
use xilem_core::{Edit, MessageResult};

struct InteractivePaintApp {
    circle1_clicked: bool,
    circle2_clicked: bool,
    click_count: i32,
    circle1_pos: (f64, f64),
    circle2_pos: (f64, f64),
}

impl Default for InteractivePaintApp {
    fn default() -> Self {
        Self {
            circle1_clicked: false,
            circle2_clicked: false,
            click_count: 0,
            circle1_pos: (55.0, 55.0),
            circle2_pos: (65.0, 65.0),
        }
    }
}



struct CircleWidget {
    color: Color,
    radius: f64,
    pos: (f64, f64),
    has_dragged: bool,
    initial_click_pos: Option<vello::kurbo::Point>,
    initial_circle_pos: (f64, f64),
}

impl Widget for CircleWidget {
    type Action = ();

    fn register_children(&mut self, _: &mut RegisterCtx<'_>) {}
    
    fn accessibility_role(&self) -> masonry::accesskit::Role {
        masonry::accesskit::Role::Button
    }
    
    fn accessibility(&mut self, _: &mut AccessCtx<'_>, _: &PropertiesRef<'_>, _: &mut masonry::accesskit::Node) {}
    
    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[])
    }
    
    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _: &PropertiesRef<'_>, scene: &mut Scene) {
        let circle = vello::kurbo::Circle::new(self.pos, self.radius);
        fill(scene, &circle, self.color);
    }
    
    fn layout(&mut self, _: &mut LayoutCtx<'_>, _: &mut PropertiesMut<'_>, bc: &BoxConstraints) -> vello::kurbo::Size {
        let size = self.radius * 2.0 + 10.0;
        bc.constrain((size, size))
    }
    
    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Down(e) => {
                ctx.capture_pointer();
                self.has_dragged = false;
                let local_pos = ctx.local_position(e.state.position);
                self.initial_click_pos = Some(local_pos);
                self.initial_circle_pos = self.pos;
            }
            PointerEvent::Move(e) => {
                if ctx.is_active() {
                    if let Some(initial_click) = self.initial_click_pos {
                        let current_pos = ctx.local_position(e.current.position);
                        let delta_x = current_pos.x - initial_click.x;
                        let delta_y = current_pos.y - initial_click.y;
                        self.pos = (self.initial_circle_pos.0 + delta_x, self.initial_circle_pos.1 + delta_y);
                        self.has_dragged = true;
                        ctx.request_paint_only();
                    }
                }
            }
            PointerEvent::Up(_) => {
                if ctx.is_active() {
                    ctx.release_pointer();
                    if self.has_dragged {
                        ctx.submit_action::<()>(());
                    }
                    self.initial_click_pos = None;
                }
            }
            _ => {}
        }
    }
}

#[derive(Debug)]
struct Circle {
    color: Color,
    radius: f64,
}

impl Circle {
    fn new(color: Color, radius: f64) -> Self {
        Self { color, radius }
    }
}

impl ViewMarker for Circle {}

impl<State: ViewArgument, Action> View<State, Action, ViewCtx> for Circle {
    type Element = Pod<CircleWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: Arg<'_, State>) -> (Self::Element, Self::ViewState) {
        (ctx.create_pod(CircleWidget { color: self.color, radius: self.radius, pos: (50.0, 50.0), has_dragged: false, initial_click_pos: None, initial_circle_pos: (50.0, 50.0) }), ())
    }

    fn rebuild(&self, prev: &Self, _: &mut Self::ViewState, _: &mut ViewCtx, mut element: Mut<'_, Self::Element>, _: Arg<'_, State>) {
        if prev.color != self.color {
            element.widget.color = self.color;
            element.ctx.request_paint_only();
        }
        if prev.radius != self.radius {
            element.widget.radius = self.radius;
            element.ctx.request_layout();
        }
    }
    
    fn teardown(&self, _: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}
    
    fn message(&self, _: &mut Self::ViewState, _: &mut MessageContext, _: Mut<'_, Self::Element>, _: Arg<'_, State>) -> MessageResult<Action> {
        MessageResult::Stale
    }
}

impl PartialEq for Circle {
    fn eq(&self, other: &Self) -> bool {
        self.color == other.color && self.radius == other.radius
    }
}

#[derive(Debug)]
struct DraggableCircle<State, Action, F> {
    circle: Circle,
    pos: (f64, f64),
    on_click: F,
    phantom: std::marker::PhantomData<fn(State) -> Action>,
}

fn draggable_circle<F>(
    color: Color,
    radius: f64,
    pos: (f64, f64),
    on_click: F,
) -> DraggableCircle<Edit<InteractivePaintApp>, (), F>
where
    F: Fn(&mut InteractivePaintApp, (f64, f64)) -> () + Send + Sync + 'static,
{
    DraggableCircle {
        circle: Circle::new(color, radius),
        pos,
        on_click,
        phantom: std::marker::PhantomData,
    }
}

impl<State, Action, F> ViewMarker for DraggableCircle<State, Action, F> {}

impl<F> View<Edit<InteractivePaintApp>, (), ViewCtx> for DraggableCircle<Edit<InteractivePaintApp>, (), F>
where
    F: Fn(&mut InteractivePaintApp, (f64, f64)) -> () + Send + Sync + 'static,
{
    type Element = Pod<CircleWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: Arg<'_, Edit<InteractivePaintApp>>) -> (Self::Element, Self::ViewState) {
        let widget = CircleWidget {
            color: self.circle.color,
            radius: self.circle.radius,
            pos: self.pos,
            has_dragged: false,
            initial_click_pos: None,
            initial_circle_pos: self.pos,
        };
        (ctx.with_action_widget(|ctx| ctx.create_pod(widget)), ())
    }

    fn rebuild(&self, prev: &Self, _: &mut Self::ViewState, _: &mut ViewCtx, mut element: Mut<'_, Self::Element>, _: Arg<'_, Edit<InteractivePaintApp>>) {
        if prev.circle.color != self.circle.color {
            element.widget.color = self.circle.color;
            element.ctx.request_paint_only();
        }
        if prev.circle.radius != self.circle.radius {
            element.widget.radius = self.circle.radius;
            element.ctx.request_layout();
        }
        if prev.pos != self.pos {
            element.widget.pos = self.pos;
            element.ctx.request_paint_only();
        }
    }
    
    fn teardown(&self, _: &mut Self::ViewState, ctx: &mut ViewCtx, element: Mut<'_, Self::Element>) {
        ctx.teardown_leaf(element);
    }
    
    fn message(&self, _: &mut Self::ViewState, message: &mut MessageContext, element: Mut<'_, Self::Element>, app_state: Arg<'_, Edit<InteractivePaintApp>>) -> MessageResult<()> {
        if message.take_first().is_some() {
            tracing::warn!("Got unexpected id path in DraggableCircle::message");
            return MessageResult::Stale;
        }
        match message.take_message::<()>() {
            Some(_) => {
                let new_pos = element.widget.pos;
                (self.on_click)(app_state, new_pos);
                MessageResult::Action(())
            },
            None => {
                tracing::error!("Wrong message type in DraggableCircle::message: {message:?}, expected ()");
                MessageResult::Stale
            }
        }
    }
}

struct CanvasWidget {
    circle1_pos: (f64, f64),
    circle2_pos: (f64, f64),
    circle1_color: Color,
    circle2_color: Color,
    circle1_radius: f64,
    circle2_radius: f64,
    has_dragged: bool,
    initial_click_pos: Option<vello::kurbo::Point>,
    initial_circle_pos: (f64, f64),
    dragging_circle: Option<u8>,
}

impl Widget for CanvasWidget {
    type Action = (u8, (f64, f64));

    fn register_children(&mut self, _: &mut RegisterCtx<'_>) {}
    
    fn accessibility_role(&self) -> masonry::accesskit::Role {
        masonry::accesskit::Role::Canvas
    }
    
    fn accessibility(&mut self, _: &mut AccessCtx<'_>, _: &PropertiesRef<'_>, _: &mut masonry::accesskit::Node) {}
    
    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[])
    }
    
    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _: &PropertiesRef<'_>, scene: &mut Scene) {
        // Draw line between circles
        stroke(scene, &Line::new(self.circle1_pos, self.circle2_pos), Color::from_rgb8(0, 255, 0), 3.0);
        
        // Draw circles
        let circle1 = vello::kurbo::Circle::new(self.circle1_pos, self.circle1_radius);
        fill(scene, &circle1, self.circle1_color);
        
        let circle2 = vello::kurbo::Circle::new(self.circle2_pos, self.circle2_radius);
        fill(scene, &circle2, self.circle2_color);
    }
    
    fn layout(&mut self, _: &mut LayoutCtx<'_>, _: &mut PropertiesMut<'_>, bc: &BoxConstraints) -> vello::kurbo::Size {
        bc.constrain((400.0, 300.0))
    }
    
    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Down(e) => {
                let local_pos = ctx.local_position(e.state.position);
                
                // Check which circle was clicked
                let dist1 = ((local_pos.x - self.circle1_pos.0).powi(2) + (local_pos.y - self.circle1_pos.1).powi(2)).sqrt();
                let dist2 = ((local_pos.x - self.circle2_pos.0).powi(2) + (local_pos.y - self.circle2_pos.1).powi(2)).sqrt();
                
                tracing::debug!("Click at ({}, {}), dist1={}, dist2={}, r1={}, r2={}", local_pos.x, local_pos.y, dist1, dist2, self.circle1_radius, self.circle2_radius);
                
                // Prioritize closer circle if both are hit
                if dist1 <= self.circle1_radius && (dist2 > self.circle2_radius || dist1 <= dist2) {
                    ctx.capture_pointer();
                    self.has_dragged = false;
                    self.initial_click_pos = Some(local_pos);
                    self.initial_circle_pos = self.circle1_pos;
                    self.dragging_circle = Some(1);
                    tracing::debug!("Started dragging circle 1");
                } else if dist2 <= self.circle2_radius {
                    ctx.capture_pointer();
                    self.has_dragged = false;
                    self.initial_click_pos = Some(local_pos);
                    self.initial_circle_pos = self.circle2_pos;
                    self.dragging_circle = Some(2);
                    tracing::debug!("Started dragging circle 2");
                } else {
                    tracing::debug!("Click missed both circles");
                }
            }
            PointerEvent::Move(e) => {
                if ctx.is_active() {
                    if let (Some(initial_click), Some(circle_id)) = (self.initial_click_pos, self.dragging_circle) {
                        let current_pos = ctx.local_position(e.current.position);
                        let delta_x = current_pos.x - initial_click.x;
                        let delta_y = current_pos.y - initial_click.y;
                        let new_pos = (self.initial_circle_pos.0 + delta_x, self.initial_circle_pos.1 + delta_y);
                        
                        if circle_id == 1 {
                            self.circle1_pos = new_pos;
                        } else {
                            self.circle2_pos = new_pos;
                        }
                        
                        self.has_dragged = true;
                        ctx.request_paint_only();
                    }
                }
            }
            PointerEvent::Up(_) => {
                if ctx.is_active() {
                    ctx.release_pointer();
                    if let Some(circle_id) = self.dragging_circle {
                        let new_pos = if circle_id == 1 { self.circle1_pos } else { self.circle2_pos };
                        tracing::debug!("Releasing circle {}, dragged={}, pos=({}, {})", circle_id, self.has_dragged, new_pos.0, new_pos.1);
                        ctx.submit_action::<(u8, (f64, f64))>((circle_id, new_pos));
                    }
                }
                // Always reset state regardless of active status
                self.initial_click_pos = None;
                self.dragging_circle = None;
                self.has_dragged = false;
            }
            _ => {}
        }
    }
}

struct TriangleWidget;

impl Widget for TriangleWidget {
    type Action = ();

    fn register_children(&mut self, _: &mut RegisterCtx<'_>) {}
    
    fn accessibility_role(&self) -> masonry::accesskit::Role {
        masonry::accesskit::Role::Image
    }
    
    fn accessibility(&mut self, _: &mut AccessCtx<'_>, _: &PropertiesRef<'_>, _: &mut masonry::accesskit::Node) {}
    
    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[])
    }
    
    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let mut path = BezPath::new();
        path.move_to((size.width / 2.0, 10.0));
        path.line_to((10.0, size.height - 10.0));
        path.line_to((size.width - 10.0, size.height - 10.0));
        path.close_path();
        fill(scene, &path, Color::from_rgb8(255, 255, 0));
    }
    
    fn layout(&mut self, _: &mut LayoutCtx<'_>, _: &mut PropertiesMut<'_>, bc: &BoxConstraints) -> vello::kurbo::Size {
        bc.constrain((80.0, 80.0))
    }
}

struct TriangleView;

impl ViewMarker for TriangleView {}

impl<State: ViewArgument, Action> View<State, Action, ViewCtx> for TriangleView {
    type Element = Pod<TriangleWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: Arg<'_, State>) -> (Self::Element, Self::ViewState) {
        (ctx.create_pod(TriangleWidget), ())
    }

    fn rebuild(&self, _: &Self, _: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>, _: Arg<'_, State>) {}
    
    fn teardown(&self, _: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}
    
    fn message(&self, _: &mut Self::ViewState, _: &mut MessageContext, _: Mut<'_, Self::Element>, _: Arg<'_, State>) -> MessageResult<Action> {
        MessageResult::Stale
    }
}

struct CanvasView;

impl ViewMarker for CanvasView {}

impl View<Edit<InteractivePaintApp>, (), ViewCtx> for CanvasView {
    type Element = Pod<CanvasWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, app_state: Arg<'_, Edit<InteractivePaintApp>>) -> (Self::Element, Self::ViewState) {
        let widget = CanvasWidget {
            circle1_pos: app_state.circle1_pos,
            circle2_pos: app_state.circle2_pos,
            circle1_color: if app_state.circle1_clicked { Color::from_rgb8(255, 100, 100) } else { Color::from_rgb8(255, 0, 0) },
            circle2_color: if app_state.circle2_clicked { Color::from_rgb8(100, 100, 255) } else { Color::from_rgb8(0, 0, 255) },
            circle1_radius: 25.0,
            circle2_radius: 30.0,
            has_dragged: false,
            initial_click_pos: None,
            initial_circle_pos: (0.0, 0.0),
            dragging_circle: None,
        };
        (ctx.with_action_widget(|ctx| ctx.create_pod(widget)), ())
    }

    fn rebuild(&self, _prev: &Self, _: &mut Self::ViewState, _: &mut ViewCtx, mut element: Mut<'_, Self::Element>, app_state: Arg<'_, Edit<InteractivePaintApp>>) {
        // Always update colors
        element.widget.circle1_color = if app_state.circle1_clicked { Color::from_rgb8(255, 100, 100) } else { Color::from_rgb8(255, 0, 0) };
        element.widget.circle2_color = if app_state.circle2_clicked { Color::from_rgb8(100, 100, 255) } else { Color::from_rgb8(0, 0, 255) };
        
        // Only reset state and update positions if not actively dragging
        if element.widget.dragging_circle.is_none() {
            element.widget.circle1_pos = app_state.circle1_pos;
            element.widget.circle2_pos = app_state.circle2_pos;
            element.widget.has_dragged = false;
            element.widget.initial_click_pos = None;
        }
        
        element.ctx.request_paint_only();
    }
    
    fn teardown(&self, _: &mut Self::ViewState, ctx: &mut ViewCtx, element: Mut<'_, Self::Element>) {
        ctx.teardown_leaf(element);
    }
    
    fn message(&self, _: &mut Self::ViewState, message: &mut MessageContext, _element: Mut<'_, Self::Element>, mut app_state: Arg<'_, Edit<InteractivePaintApp>>) -> MessageResult<()> {
        if message.take_first().is_some() {
            tracing::warn!("Got unexpected id path in CanvasView::message");
            return MessageResult::Stale;
        }
        match message.take_message::<(u8, (f64, f64))>() {
            Some(boxed_msg) => {
                let (circle_id, new_pos) = *boxed_msg;
                app_state.click_count += 1;
                if circle_id == 1 {
                    app_state.circle1_clicked = !app_state.circle1_clicked;
                    app_state.circle1_pos = new_pos;
                } else {
                    app_state.circle2_clicked = !app_state.circle2_clicked;
                    app_state.circle2_pos = new_pos;
                }
                MessageResult::Action(())
            },
            None => {
                tracing::error!("Wrong message type in CanvasView::message: {message:?}, expected (u8, (f64, f64))");
                MessageResult::Stale
            }
        }
    }
}

fn app_logic(data: &mut InteractivePaintApp) -> impl WidgetView<Edit<InteractivePaintApp>> + use<> {
    flex_col((
        label(format!("Interactive Paint - Clicks: {}", data.click_count)),
        
        // Canvas with both circles and connecting line
        sized_box(CanvasView)
            .width(400.px())
            .height(300.px()),
        
        // Yellow triangle
        sized_box(TriangleView)
            .width(80.px())
            .height(80.px()),
    ))
}

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let data = InteractivePaintApp::default();
    let app = Xilem::new_simple(
        data,
        app_logic,
        WindowOptions::new("Interactive Paint").with_initial_inner_size(LogicalSize::new(400.0, 700.0)),
    );
    app.run_in(event_loop)
}

#[expect(clippy::allow_attributes, reason = "No way to specify the condition")]
#[allow(dead_code, reason = "False positive: needed in not-_android version")]
fn main() -> Result<(), EventLoopError> {
    run(EventLoop::with_user_event())
}

#[cfg(target_os = "android")]
#[expect(
    unsafe_code,
    reason = "We believe that there are no other declarations using this name in the compiled objects here"
)]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let mut event_loop = EventLoop::with_user_event();
    event_loop.with_android_app(app);

    run(event_loop).expect("Can create app");
}