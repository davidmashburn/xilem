//! Vello graphics - colored rectangles

use masonry::core::*;
use masonry::dpi::LogicalSize;
use masonry::properties::types::AsUnit;
use masonry::util::stroke;
use vello::kurbo::Line;
use vello::Scene;
use winit::error::EventLoopError;
use xilem::style::Style as _;
use xilem::view::{label, sized_box, flex_col};
use xilem::{Color, EventLoop, WidgetView, WindowOptions, Xilem};
use xilem_core::Edit;
use xilem::core::{Arg, MessageContext, Mut, View, ViewArgument, ViewMarker};
use xilem::{Pod, ViewCtx};
use xilem_core::MessageResult;

// Custom widget that draws lines with Scene::stroke
struct LineWidget;

impl Widget for LineWidget {
    type Action = ();
    
    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}
    fn accessibility_role(&self) -> masonry::accesskit::Role { masonry::accesskit::Role::Image }
    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut masonry::accesskit::Node) {}
    fn children_ids(&self) -> ChildrenIds { ChildrenIds::from_slice(&[]) }
    
    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let angle = 30.0_f64.to_radians();
        let length = size.width - 40.0;
        let end_x = 20.0 + length * angle.cos();
        let end_y = size.height / 2.0 - length * angle.sin();
        let line = Line::new((20.0, size.height / 2.0), (end_x, end_y));
        stroke(scene, &line, Color::new([0.0, 1.0, 0.0, 1.0]), 3.0);
    }
    
    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &mut PropertiesMut<'_>, bc: &BoxConstraints) -> vello::kurbo::Size {
        bc.constrain((200.0, 100.0))
    }
}

struct LineView;

impl ViewMarker for LineView {}
impl<State: ViewArgument, Action> View<State, Action, ViewCtx> for LineView {
    type Element = Pod<LineWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: Arg<'_, State>) -> (Self::Element, Self::ViewState) {
        let pod = ctx.create_pod(LineWidget);
        (pod, ())
    }

    fn rebuild(&self, _: &Self, _: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>, _: Arg<'_, State>) {}
    fn teardown(&self, _: &mut Self::ViewState, _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}
    fn message(&self, _: &mut Self::ViewState, _: &mut MessageContext, _: Mut<'_, Self::Element>, _: Arg<'_, State>) -> MessageResult<Action> {
        MessageResult::Stale
    }
}

fn line_widget() -> LineView {
    LineView
}

struct VelloGraphics;

impl VelloGraphics {
    fn view(&mut self) -> impl WidgetView<Edit<Self>> + use<> {
        flex_col((
            // Circle 1
            sized_box(label("Circle 1"))
                .width(100.px())
                .height(100.px())
                .background(masonry::properties::Background::Color(Color::new([1.0, 0.0, 0.0, 1.0])))
                .corner_radius(50.0),
            
            // Circle 2
            sized_box(label("Circle 2"))
                .width(120.px())
                .height(120.px())
                .background(masonry::properties::Background::Color(Color::new([0.0, 0.0, 1.0, 1.0])))
                .corner_radius(60.0),
            
            // True line using Scene::stroke
            sized_box(line_widget())
                .width(200.px())
                .height(100.px()),
            
            // Triangle (rotated square)
            sized_box(label("△"))
                .width(80.px())
                .height(80.px())
                .background(masonry::properties::Background::Color(Color::new([1.0, 1.0, 0.0, 1.0])))
        ))
    }
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new_simple(
        VelloGraphics,
        VelloGraphics::view,
        WindowOptions::new("Vello Graphics")
            .with_initial_inner_size(LogicalSize::new(400.0, 600.0)),
    );
    app.run_in(EventLoop::with_user_event())?;
    Ok(())
}