use std::cell::RefCell;
use std::f64::consts::PI;
use std::rc::Rc;

use pangocairo::pango::{FontDescription, SCALE};

use crate::sketch_board::MouseButton;
use crate::style::Style;
use crate::{math::Vec2D, sketch_board::MouseEventMsg};

use super::{Drawable, DrawableClone, Tool, ToolUpdateResult};

pub struct MarkerTool {
    style: Style,
    next_number: Rc<RefCell<u16>>,
}

#[derive(Clone, Debug)]
pub struct Marker {
    pos: Vec2D,
    number: u16,
    style: Style,
    tool_next_number: Rc<RefCell<u16>>,
}

impl Drawable for Marker {
    fn draw(
        &self,
        cx: &pangocairo::cairo::Context,
        _surface: &pangocairo::cairo::ImageSurface,
    ) -> anyhow::Result<()> {
        let layout = pangocairo::create_layout(cx);

        // set text
        let mut desc = FontDescription::from_string("Sans,Times new roman");
        desc.set_size(self.style.size.to_text_size());
        layout.set_font_description(Some(&desc));
        layout.set_alignment(pangocairo::pango::Alignment::Center);
        layout.set_text(format!("{}", self.number).as_str());

        // calculate circle positon and size
        let (_, rect) = layout.extents();
        let circle_pos_x = self.pos.x + (rect.x() / SCALE + rect.width() / SCALE / 2) as f64;
        let circle_pos_y = self.pos.y + (rect.y() / SCALE + rect.height() / SCALE / 2) as f64;
        let circle_radius = ((rect.width() / SCALE * rect.width() / SCALE) as f64
            + (rect.height() / SCALE * rect.height() / SCALE) as f64)
            .sqrt();

        let (r, g, b) = self.style.color.to_rgb_f64();

        cx.save()?;

        // draw a circle background
        cx.arc(
            circle_pos_x,
            circle_pos_y,
            circle_radius * 0.8,
            0.0,
            2.0 * PI,
        ); // full circle
        cx.set_source_rgb(r, g, b);
        cx.fill()?;

        // draw a circle around
        cx.arc(circle_pos_x, circle_pos_y, circle_radius, 0.0, 2.0 * PI); // full circle
        cx.set_source_rgb(r, g, b);
        cx.set_line_width(self.style.size.to_line_width() * 2.0);
        cx.stroke()?;

        // render text on top
        cx.set_source_rgb(1.0, 1.0, 1.0);
        cx.move_to(self.pos.x, self.pos.y);
        pangocairo::show_layout(cx, &layout);

        cx.restore()?;

        Ok(())
    }

    fn handle_undo(&mut self) {
        *self.tool_next_number.borrow_mut() = self.number;
    }

    fn handle_redo(&mut self) {
        *self.tool_next_number.borrow_mut() = self.number + 1;
    }
}

impl Tool for MarkerTool {
    fn get_drawable(&self) -> Option<&dyn Drawable> {
        None
    }

    fn handle_style_event(&mut self, style: Style) -> ToolUpdateResult {
        self.style = style;
        ToolUpdateResult::Unmodified
    }

    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        match event {
            MouseEventMsg::Click(pos, button) => {
                if button == MouseButton::Primary {
                    let marker = Marker {
                        pos,
                        number: *self.next_number.borrow(),
                        style: self.style,
                        tool_next_number: self.next_number.clone(),
                    };

                    // increment for next
                    *self.next_number.borrow_mut() += 1;

                    ToolUpdateResult::Commit(marker.clone_box())
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            _ => ToolUpdateResult::Unmodified,
        }
    }
}

impl Default for MarkerTool {
    fn default() -> Self {
        Self {
            style: Default::default(),
            next_number: Rc::new(RefCell::new(1)),
        }
    }
}
