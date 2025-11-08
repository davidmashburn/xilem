// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Custom paint example

use masonry::core::*;
use masonry::dpi::LogicalSize;
use masonry::properties::{types::AsUnit, Background};
use masonry::util::{fill, stroke};
use masonry_winit::app::{EventLoop, EventLoopBuilder};
use vello::{kurbo::{BezPath, Line}, Scene};
use winit::error::EventLoopError;
use xilem::core::{Arg, MessageContext, Mut, View, ViewArgument, ViewMarker};
use xilem::style::Style as _;
use xilem::view::{flex_col, label, sized_box};
use xilem::{Color, Pod, ViewCtx, WidgetView, WindowOptions, Xilem};
use xilem_core::{Edit, MessageResult};

struct LineWidget;

struct TriangleWidget;

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

struct CustomPaintApp;

fn app_logic(_data: &mut CustomPaintApp) -> impl WidgetView<Edit<CustomPaintApp>> + use<> {
    flex_col((
        // Red circle
        sized_box(label("Circle 1"))
            .width(100.px())
            .height(100.px())
            .background(Background::Color(Color::from_rgb8(255, 0, 0)))
            .corner_radius(50.0),
        
        // Blue circle
        sized_box(label("Circle 2"))
            .width(120.px())
            .height(120.px())
            .background(Background::Color(Color::from_rgb8(0, 0, 255)))
            .corner_radius(60.0),
        
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
    let data = CustomPaintApp;
    let app = Xilem::new_simple(
        data,
        app_logic,
        WindowOptions::new("Custom Paint").with_initial_inner_size(LogicalSize::new(400.0, 600.0)),
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