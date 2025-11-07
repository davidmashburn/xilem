//! Vello graphics - colored rectangles

use masonry::dpi::LogicalSize;
use masonry::properties::types::AsUnit;
use winit::error::EventLoopError;
use xilem::style::Style as _;
use xilem::view::{label, sized_box, flex_col, text_button};
use xilem::{Color, EventLoop, WidgetView, WindowOptions, Xilem};
use xilem_core::Edit;

struct VelloGraphics {
    red: bool,
}

impl VelloGraphics {
    fn view(&mut self) -> impl WidgetView<Edit<Self>> + use<> {
        flex_col((
            // Rectangle
            sized_box(label("Rectangle"))
                .width(200.px())
                .height(200.px())
                .background(masonry::properties::Background::Color(if self.red {
                    Color::new([1.0, 0.0, 0.0, 1.0])
                } else {
                    Color::new([0.0, 0.0, 1.0, 1.0])
                })),
            
            // Circle
            sized_box(label("Circle"))
                .width(150.px())
                .height(150.px())
                .background(masonry::properties::Background::Color(Color::new([0.0, 1.0, 0.0, 1.0])))
                .corner_radius(75.0),
            
            text_button("Toggle Color", |data: &mut Self| {
                data.red = !data.red;
            })
        ))
    }
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new_simple(
        VelloGraphics { red: true },
        VelloGraphics::view,
        WindowOptions::new("Vello Graphics")
            .with_initial_inner_size(LogicalSize::new(400.0, 500.0)),
    );
    app.run_in(EventLoop::with_user_event())?;
    Ok(())
}