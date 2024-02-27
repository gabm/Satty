use std::f32::consts::PI;

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
pub struct Arrow {
    start: Vec2D,
    end: Option<Vec2D>,
    style: Style,
}

#[derive(Default)]
pub struct ArrowTool {
    arrow: Option<Arrow>,
    style: Style,
}

impl Tool for ArrowTool {
    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        match event.type_ {
            MouseEventType::BeginDrag => {
                // start new
                self.arrow = Some(Arrow {
                    start: event.pos,
                    end: None,
                    style: self.style,
                });

                ToolUpdateResult::Redraw
            }
            MouseEventType::EndDrag => {
                if let Some(a) = &mut self.arrow {
                    if event.pos == Vec2D::zero() {
                        self.arrow = None;

                        ToolUpdateResult::Redraw
                    } else {
                        if event.modifier.intersects(ModifierType::SHIFT_MASK) {
                            a.end = Some(a.start + event.pos.snapped_vector_15deg());
                        } else {
                            a.end = Some(a.start + event.pos);
                        }
                        let result = a.clone_box();
                        self.arrow = None;

                        ToolUpdateResult::Commit(result)
                    }
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            MouseEventType::UpdateDrag => {
                if let Some(a) = &mut self.arrow {
                    if event.pos == Vec2D::zero() {
                        return ToolUpdateResult::Unmodified;
                    }
                    if event.modifier.intersects(ModifierType::SHIFT_MASK) {
                        a.end = Some(a.start + event.pos.snapped_vector_15deg());
                    } else {
                        a.end = Some(a.start + event.pos);
                    }

                    ToolUpdateResult::Redraw
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            _ => ToolUpdateResult::Unmodified,
        }
    }

    fn handle_key_event(&mut self, event: crate::sketch_board::KeyEventMsg) -> ToolUpdateResult {
        if event.key == Key::Escape && self.arrow.is_some() {
            self.arrow = None;
            ToolUpdateResult::Redraw
        } else {
            ToolUpdateResult::Unmodified
        }
    }

    fn get_drawable(&self) -> Option<&dyn Drawable> {
        match &self.arrow {
            Some(d) => Some(d),
            None => None,
        }
    }

    fn handle_style_event(&mut self, style: Style) -> ToolUpdateResult {
        self.style = style;
        ToolUpdateResult::Unmodified
    }
}

impl Arrow {
    fn get_arrow_head_points(&self) -> (Vec2D, Vec2D) {
        let end = match self.end {
            Some(e) => e,
            None => return (Vec2D::zero(), Vec2D::zero()), // exit if no end
        };

        // borrowed from: https://math.stackexchange.com/questions/1314006/drawing-an-arrow
        let delta = self.start - end;
        let l1 = delta.norm();
        const L2: f32 = 30.0;
        const PHI: f32 = PI / 6.0;
        let (sin_phi, cos_phi) = PHI.sin_cos();

        let x3 = end.x + L2 / l1 * (delta.x * cos_phi + delta.y * sin_phi);
        let y3 = end.y + L2 / l1 * (delta.y * cos_phi - delta.x * sin_phi);

        let x4 = end.x + L2 / l1 * (delta.x * cos_phi - delta.y * sin_phi);
        let y4 = end.y + L2 / l1 * (delta.y * cos_phi + delta.x * sin_phi);

        (Vec2D::new(x3, y3), Vec2D::new(x4, y4))
    }
}

impl Drawable for Arrow {
    fn draw(
        &self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        _font: FontId,
    ) -> Result<()> {
        let end = match self.end {
            Some(e) => e,
            None => return Ok(()), // exit if no end
        };
        let (p1, p2) = self.get_arrow_head_points();

        canvas.save();

        let mut path = Path::new();
        path.move_to(self.start.x, self.start.y);
        path.line_to(end.x, end.y);

        path.move_to(p1.x, p1.y);
        path.line_to(end.x, end.y);
        path.line_to(p2.x, p2.y);

        canvas.stroke_path(&path, &self.style.into());

        canvas.restore();
        Ok(())
    }
}
