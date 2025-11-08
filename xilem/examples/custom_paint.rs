// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Custom paint example

use masonry::core::*;
use masonry::dpi::LogicalSize;
use masonry::properties::{types::AsUnit, Background};
use masonry::util::stroke;
use vello::{kurbo::Line, Scene};
use winit::error::EventLoopError;
use xilem::style::Style as _;
use xilem::view::{flex_col, label, sized_box};
use xilem::{Color, EventLoop, WidgetView, WindowOptions, Xilem};
use xilem_core::{Edit, MessageResult};
use xilem::core::{Arg, MessageContext, Mut, View, ViewArgument, ViewMarker};
use xilem::{Pod, ViewCtx};

struct LineWidget;

impl Widget for LineWidget {
    type Action = ();
    fn register_children(&mut self, _: &mut RegisterCtx<'_>) {}
    fn accessibility_role(&self) -> masonry::accesskit::Role { masonry::accesskit::Role::Image }
    fn accessibility(&mut self, _: &mut AccessCtx<'_>, _: &PropertiesRef<'_>, _: &mut masonry::accesskit::Node) {}
    fn children_ids(&self) -> ChildrenIds { ChildrenIds::from_slice(&[]) }
    
    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        stroke(scene, &Line::new((20.0, size.height / 2.0), (size.width - 20.0, size.height / 2.0 - 50.0)), Color::from_rgb8(0, 255, 0), 3.0);
    }
    
    fn layout(&mut self, _: &mut LayoutCtx<'_>, _: &mut PropertiesMut<'_>, bc: &BoxConstraints) -> vello::kurbo::Size {
        bc.constrain((200.0, 100.0))
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
        sized_box(label("Circle 1"))
            .width(100.px())
            .height(100.px())
            .background(Background::Color(Color::from_rgb8(255, 0, 0)))
            .corner_radius(50.0),
        sized_box(label("Circle 2"))
            .width(120.px())
            .height(120.px())
            .background(Background::Color(Color::from_rgb8(0, 0, 255)))
            .corner_radius(60.0),
        sized_box(LineView)
            .width(200.px())
            .height(100.px()),
        sized_box(label("△"))
            .width(80.px())
            .height(80.px())
            .background(Background::Color(Color::from_rgb8(255, 255, 0))),
    ))
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new_simple(
        CustomPaintApp,
        app_logic,
        WindowOptions::new("Custom Paint")
            .with_initial_inner_size(LogicalSize::new(400.0, 600.0)),
    );
    app.run_in(EventLoop::with_user_event())
}