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
use xilem::view::{button, flex_col, label, sized_box};
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



struct CircleWidget {
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
        let center = (size.width / 2.0, size.height / 2.0);
        let circle = vello::kurbo::Circle::new(center, self.radius);
        fill(scene, &circle, self.color);
    }
    
    fn layout(&mut self, _: &mut LayoutCtx<'_>, _: &mut PropertiesMut<'_>, bc: &BoxConstraints) -> vello::kurbo::Size {
        let size = self.radius * 2.0 + 10.0;
        bc.constrain((size, size))
    }
}

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
        (ctx.create_pod(CircleWidget { color: self.color, radius: self.radius }), ())
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
        button(
            Circle::new(
                if data.circle1_clicked { Color::from_rgb8(255, 100, 100) } else { Color::from_rgb8(200, 0, 0) },
                30.0
            ),
            |data: &mut InteractivePaintApp| {
                data.click_count += 1;
                data.circle1_clicked = !data.circle1_clicked;
            },
        ),
        
        // Clickable blue circle
        button(
            Circle::new(
                if data.circle2_clicked { Color::from_rgb8(100, 100, 255) } else { Color::from_rgb8(0, 0, 200) },
                30.0
            ),
            |data: &mut InteractivePaintApp| {
                data.click_count += 1;
                data.circle2_clicked = !data.circle2_clicked;
            },
        ),
        
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