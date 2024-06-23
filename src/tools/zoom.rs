use femtovg::FontId;

use super::{Drawable, Tool, ToolUpdateResult};
use crate::sketch_board::{MouseButton, MouseEventMsg, MouseEventType};
use relm4::gtk::gdk::ModifierType;

#[derive(Clone, Copy, Debug)]
pub struct Zoom {
    factor: f32,
}

impl Drawable for Zoom {
    fn draw(
        &self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        _font: FontId,
    ) -> anyhow::Result<()> {
        canvas.scale(self.factor, self.factor);
        Ok(())
    }
}

#[derive(Default)]
pub struct ZoomTool {
    zoom: Option<Zoom>,
}

impl Tool for ZoomTool {
    fn get_drawable(&self) -> Option<&dyn Drawable> {
        match &self.zoom {
            Some(d) => Some(d),
            None => None,
        }
    }

    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        match event.type_ {
            MouseEventType::Click => {
                if event.button == MouseButton::Primary {
                    if let Some(zoom) = &mut self.zoom {
                        if event.modifier.intersects(ModifierType::CONTROL_MASK) {
                            zoom.factor -= 0.1;
                        } else {
                            zoom.factor += 0.1;
                        }
                        return ToolUpdateResult::Redraw;
                    }
                }
                return ToolUpdateResult::Unmodified;
            }
            _ => ToolUpdateResult::Unmodified,
        }
    }
}
