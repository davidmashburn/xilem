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
            PointerEvent::Down(_) => {
                ctx.capture_pointer();
                ctx.submit_action::<()>(());
            }
            PointerEvent::Move(e) => {
                if ctx.is_active() {
                    let local_pos = ctx.local_position(e.current.position);
                    self.pos = (local_pos.x, local_pos.y);
                    ctx.request_paint_only();
                }
            }
            PointerEvent::Up(_) => {
                if ctx.is_active() {
                    ctx.release_pointer();
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
        (ctx.create_pod(CircleWidget { color: self.color, radius: self.radius, pos: (50.0, 50.0) }), ())
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

struct LineWidget;

impl Widget for LineWidget {
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
        let start = (20.0, size.height / 2.0);
        let end = (size.width - 20.0, size.height / 2.0 - 50.0);
        stroke(scene, &Line::new(start, end), Color::from_rgb8(0, 255, 0), 3.0);
    }
    
    fn layout(&mut self, _: &mut LayoutCtx<'_>, _: &mut PropertiesMut<'_>, bc: &BoxConstraints) -> vello::kurbo::Size {
        bc.constrain((200.0, 100.0))
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

struct LineView;

impl ViewMarker for LineView {}

impl<State: ViewArgument, Action> View<State, Action, ViewCtx> for LineView {
    type Element = Pod<LineWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: Arg<'_, State>) -> (Self::Element, Self::ViewState) {
        (ctx.create_pod(LineWidget), ())
    }

    fn rebuild(&self, _: &Self, _: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>, _: Arg<'_, State>) {}
    
    fn teardown(&self, _: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}
    
    fn message(&self, _: &mut Self::ViewState, _: &mut MessageContext, _: Mut<'_, Self::Element>, _: Arg<'_, State>) -> MessageResult<Action> {
        MessageResult::Stale
    }
}

fn app_logic(data: &mut InteractivePaintApp) -> impl WidgetView<Edit<InteractivePaintApp>> + use<> {
    flex_col((
        label(format!("Interactive Paint - Clicks: {}", data.click_count)),
        
        // Draggable red circle
        sized_box(draggable_circle(
            if data.circle1_clicked { Color::from_rgb8(255, 100, 100) } else { Color::from_rgb8(255, 0, 0) },
            50.0,
            data.circle1_pos,
            |data: &mut InteractivePaintApp, pos: (f64, f64)| {
                data.click_count += 1;
                data.circle1_clicked = !data.circle1_clicked;
                data.circle1_pos = pos;
            },
        )).width(400.px()).height(300.px()),
        
        // Draggable blue circle
        sized_box(draggable_circle(
            if data.circle2_clicked { Color::from_rgb8(100, 100, 255) } else { Color::from_rgb8(0, 0, 255) },
            60.0,
            data.circle2_pos,
            |data: &mut InteractivePaintApp, pos: (f64, f64)| {
                data.click_count += 1;
                data.circle2_clicked = !data.circle2_clicked;
                data.circle2_pos = pos;
            },
        )).width(400.px()).height(300.px()),
        
        // Custom painted line
        sized_box(LineView)
            .width(200.px())
            .height(100.px()),
        
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