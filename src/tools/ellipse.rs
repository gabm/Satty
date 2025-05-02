use anyhow::Result;
use femtovg::{FontId, Path};
use relm4::gtk::gdk::{Key, ModifierType};

use crate::{
    math::Vec2D,
    sketch_board::{MouseEventMsg, MouseEventType},
    style::Style,
};

use super::{Drawable, DrawableClone, Tool, ToolUpdateResult};

#[derive(Clone, Copy, Debug)]
pub struct Ellipse {
    origin: Vec2D,
    middle: Vec2D,
    radii: Option<Vec2D>,
    style: Style,
}

impl Drawable for Ellipse {
    fn draw(
        &self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        _font: FontId,
    ) -> Result<()> {
        let radii = match self.radii {
            Some(s) => s,
            None => return Ok(()), // early exit if none
        };

        canvas.save();
        let mut path = Path::new();
        path.ellipse(self.middle.x, self.middle.y, radii.x, radii.y);

        if self.style.fill {
            canvas.fill_path(&path, &self.style.into());
        } else {
            canvas.stroke_path(&path, &self.style.into());
        }
        canvas.restore();

        Ok(())
    }
}

#[derive(Default)]
pub struct EllipseTool {
    ellipse: Option<Ellipse>,
    style: Style,
}

impl Tool for EllipseTool {
    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        match event.type_ {
            MouseEventType::BeginDrag => {
                // start new
                self.ellipse = Some(Ellipse {
                    origin: event.pos,
                    middle: event.pos,
                    radii: None,
                    style: self.style,
                });

                ToolUpdateResult::Redraw
            }
            MouseEventType::EndDrag => {
                if let Some(ellipse) = &mut self.ellipse {
                    if event.pos == Vec2D::zero() {
                        self.ellipse = None;

                        ToolUpdateResult::Redraw
                    } else {
                        EllipseTool::calculate_shape(ellipse, &event);
                        let result = ellipse.clone_box();
                        self.ellipse = None;
                        ToolUpdateResult::Commit(result)
                    }
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            MouseEventType::UpdateDrag => {
                if let Some(ellipse) = &mut self.ellipse {
                    if event.pos == Vec2D::zero() {
                        return ToolUpdateResult::Unmodified;
                    }
                    EllipseTool::calculate_shape(ellipse, &event);
                    ToolUpdateResult::Redraw
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            _ => ToolUpdateResult::Unmodified,
        }
    }

    fn handle_key_event(&mut self, event: crate::sketch_board::KeyEventMsg) -> ToolUpdateResult {
        if event.key == Key::Escape && self.ellipse.is_some() {
            self.ellipse = None;
            ToolUpdateResult::Redraw
        } else {
            ToolUpdateResult::Unmodified
        }
    }

    fn handle_style_event(&mut self, style: Style) -> ToolUpdateResult {
        self.style = style;
        ToolUpdateResult::Unmodified
    }

    fn get_drawable(&self) -> Option<&dyn Drawable> {
        match &self.ellipse {
            Some(d) => Some(d),
            None => None,
        }
    }
}

impl EllipseTool {
    fn calculate_shape(ellipse: &mut Ellipse, event: &MouseEventMsg) {
        match event.modifier & (ModifierType::CONTROL_MASK | ModifierType::SHIFT_MASK) {
            v if v == ModifierType::CONTROL_MASK | ModifierType::SHIFT_MASK => {
                let max_size = (event.pos.x / 2.0).abs().max((event.pos.y / 2.0).abs());
                ellipse.radii = Some(Vec2D {
                    x: max_size * event.pos.x.signum(),
                    y: max_size * event.pos.y.signum(),
                });
                ellipse.middle.x = ellipse.origin.x + max_size * event.pos.x.signum();
                ellipse.middle.y = ellipse.origin.y + max_size * event.pos.y.signum();
            }
            ModifierType::CONTROL_MASK => {
                ellipse.radii = Some(Vec2D {
                    x: event.pos.x / 2.0,
                    y: event.pos.y / 2.0,
                });
                ellipse.middle.x = ellipse.origin.x + event.pos.x / 2.0;
                ellipse.middle.y = ellipse.origin.y + event.pos.y / 2.0;
            }
            ModifierType::SHIFT_MASK => {
                ellipse.middle = ellipse.origin;
                let max_size = event.pos.x.abs().max(event.pos.y.abs());
                ellipse.radii = Some(Vec2D {
                    x: max_size * event.pos.x.signum(),
                    y: max_size * event.pos.y.signum(),
                });
            }
            _ => {
                ellipse.middle = ellipse.origin;
                ellipse.radii = Some(event.pos);
            }
        }
    }
}
