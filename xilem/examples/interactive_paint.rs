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
}

impl Default for InteractivePaintApp {
    fn default() -> Self {
        Self {
            circle1_clicked: false,
            circle2_clicked: false,
            click_count: 0,
        }
    }
}

pub struct CircleWidget {
    color: Color,
    radius: f64,
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
    
    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let center_x = size.width / 2.0;
        let center_y = size.height / 2.0;
        let circle = vello::kurbo::Circle::new((center_x, center_y), self.radius);
        fill(scene, &circle, self.color);
    }
    
    fn layout(&mut self, _: &mut LayoutCtx<'_>, _: &mut PropertiesMut<'_>, bc: &BoxConstraints) -> vello::kurbo::Size {
        let size = self.radius * 2.0 + 10.0;
        bc.constrain((size, size))
    }
    
    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if matches!(event, PointerEvent::Down { .. }) {
            ctx.submit_action::<()>(());
        }
    }
}



#[derive(Debug)]
pub struct ClickableCircle<State, Action, F> {
    color: Color,
    radius: f64,
    on_click: F,
    phantom: std::marker::PhantomData<fn(State) -> Action>,
}

/// Creates a clickable circle widget
pub fn clickable_circle<
    State: ViewArgument,
    Action,
    F: Fn(Arg<'_, State>) -> Action + Send + Sync + 'static,
>(
    color: Color,
    radius: f64,
    on_click: F,
) -> ClickableCircle<State, Action, F> {
    ClickableCircle {
        color,
        radius,
        on_click,
        phantom: std::marker::PhantomData,
    }
}

impl<State, Action, F> ViewMarker for ClickableCircle<State, Action, F> {}

impl<State, Action, F> View<State, Action, ViewCtx> for ClickableCircle<State, Action, F>
where
    State: ViewArgument,
    Action: 'static,
    F: Fn(Arg<'_, State>) -> Action + Send + Sync + 'static,
{
    type Element = Pod<CircleWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: Arg<'_, State>) -> (Self::Element, Self::ViewState) {
        let widget = CircleWidget {
            color: self.color,
            radius: self.radius,
        };
        (ctx.with_action_widget(|ctx| ctx.create_pod(widget)), ())
    }

    fn rebuild(&self, _: &Self, _: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>, _: Arg<'_, State>) {}
    
    fn teardown(&self, _: &mut Self::ViewState, ctx: &mut ViewCtx, element: Mut<'_, Self::Element>) {
        ctx.teardown_leaf(element);
    }
    
    fn message(&self, _: &mut Self::ViewState, message: &mut MessageContext, _: Mut<'_, Self::Element>, app_state: Arg<'_, State>) -> MessageResult<Action> {
        if message.take_first().is_some() {
            tracing::warn!("Got unexpected id path in ClickableCircle::message");
            return MessageResult::Stale;
        }
        match message.take_message::<()>() {
            Some(_) => MessageResult::Action((self.on_click)(app_state)),
            None => {
                tracing::error!("Wrong message type in ClickableCircle::message: {message:?}, expected ()");
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
        
        // Clickable red circle
        sized_box(clickable_circle(
            if data.circle1_clicked { Color::from_rgb8(255, 100, 100) } else { Color::from_rgb8(255, 0, 0) },
            50.0,
            |data: &mut InteractivePaintApp| {
                data.click_count += 1;
                data.circle1_clicked = !data.circle1_clicked;
            },
        )).width(110.px()).height(110.px()),
        
        // Clickable blue circle
        sized_box(clickable_circle(
            if data.circle2_clicked { Color::from_rgb8(100, 100, 255) } else { Color::from_rgb8(0, 0, 255) },
            60.0,
            |data: &mut InteractivePaintApp| {
                data.click_count += 1;
                data.circle2_clicked = !data.circle2_clicked;
            },
        )).width(130.px()).height(130.px()),
        
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