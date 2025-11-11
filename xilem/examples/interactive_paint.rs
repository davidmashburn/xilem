// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Interactive custom paint example with clickable circles

use masonry::core::*;
use masonry::dpi::LogicalSize;
use masonry::properties::types::AsUnit;
use masonry::util::{fill, stroke};
use masonry_winit::app::{EventLoop, EventLoopBuilder};
use vello::{kurbo::Line, Scene};
use winit::error::EventLoopError;
use xilem::core::{Arg, MessageContext, Mut, View, ViewMarker};
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
                
                // Prioritize closer circle if both are hit
                if dist1 <= self.circle1_radius && (dist2 > self.circle2_radius || dist1 <= dist2) {
                    ctx.capture_pointer();
                    self.has_dragged = false;
                    self.initial_click_pos = Some(local_pos);
                    self.initial_circle_pos = self.circle1_pos;
                    self.dragging_circle = Some(1);
                } else if dist2 <= self.circle2_radius {
                    ctx.capture_pointer();
                    self.has_dragged = false;
                    self.initial_click_pos = Some(local_pos);
                    self.initial_circle_pos = self.circle2_pos;
                    self.dragging_circle = Some(2);
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
                        ctx.submit_action::<(u8, (f64, f64))>((circle_id, new_pos));
                    }
                }
                // Always reset state
                self.initial_click_pos = None;
                self.dragging_circle = None;
                self.has_dragged = false;
            }
            _ => {}
        }
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
            None => MessageResult::Stale
        }
    }
}

fn app_logic(data: &mut InteractivePaintApp) -> impl WidgetView<Edit<InteractivePaintApp>> + use<> {
    flex_col((
        label(format!("Interactive Paint - Clicks: {}", data.click_count)),
        sized_box(CanvasView).width(400.px()).height(300.px()),
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