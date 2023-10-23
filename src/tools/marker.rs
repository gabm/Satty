use pangocairo::pango::FontDescription;

use crate::sketch_board::MouseButton;
use crate::style::Style;
use crate::{math::Vec2D, sketch_board::MouseEventMsg};

use super::{Drawable, DrawableClone, Tool, ToolUpdateResult};

pub struct MarkerTool {
    style: Style,
    next_number: u16,
}

#[derive(Clone, Debug)]
pub struct Marker {
    pos: Vec2D,
    number: u16,
    style: Style,
}

impl Drawable for Marker {
    fn draw(
        &self,
        cx: &pangocairo::cairo::Context,
        surface: &pangocairo::cairo::ImageSurface,
    ) -> anyhow::Result<()> {
        let layout = pangocairo::create_layout(cx);

        layout.set_text(format!("{}", self.number).as_str());

        let mut desc = FontDescription::from_string("Sans,Times new roman");
        desc.set_size(self.style.size.to_text_size());
        layout.set_font_description(Some(&desc));

        let (r, g, b) = self.style.color.to_rgb_f64();

        cx.save()?;
        cx.set_source_rgb(r, g, b);

        cx.move_to(self.pos.x, self.pos.y);
        pangocairo::show_layout(cx, &layout);

        cx.restore()?;

        Ok(())
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
                        number: self.next_number,
                        style: self.style,
                    };

                    // increment for next
                    self.next_number += 1;

                    ToolUpdateResult::Commit(marker.clone_box())
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            _ => ToolUpdateResult::Unmodified,
        }
    }
}

impl MarkerTool {
    pub fn new(style: Style) -> Self {
        Self {
            next_number: 1,
            style,
        }
    }
}
