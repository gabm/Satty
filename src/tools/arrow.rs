use std::f64::consts::PI;

use anyhow::Result;
use pangocairo::cairo::{Context, ImageSurface};
use relm4::gtk::gdk::Key;

use crate::{math::Vec2D, sketch_board::MouseEventMsg, style::Style};

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
        match event {
            MouseEventMsg::BeginDrag(pos) => {
                // start new
                self.arrow = Some(Arrow {
                    start: pos,
                    end: None,
                    style: self.style,
                });

                ToolUpdateResult::Redraw
            }
            MouseEventMsg::EndDrag(dir) => {
                if let Some(a) = &mut self.arrow {
                    if dir == Vec2D::zero() {
                        self.arrow = None;

                        ToolUpdateResult::Redraw
                    } else {
                        a.end = Some(a.start + dir);
                        let result = a.clone_box();
                        self.arrow = None;

                        ToolUpdateResult::Commit(result)
                    }
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            MouseEventMsg::UpdateDrag(dir) => {
                if let Some(a) = &mut self.arrow {
                    if dir == Vec2D::zero() {
                        return ToolUpdateResult::Unmodified;
                    }
                    a.end = Some(a.start + dir);

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
        const L2: f64 = 30.0;
        const PHI: f64 = PI / 6.0;
        let (sin_phi, cos_phi) = PHI.sin_cos();

        let x3 = end.x + L2 / l1 * (delta.x * cos_phi + delta.y * sin_phi);
        let y3 = end.y + L2 / l1 * (delta.y * cos_phi - delta.x * sin_phi);

        let x4 = end.x + L2 / l1 * (delta.x * cos_phi - delta.y * sin_phi);
        let y4 = end.y + L2 / l1 * (delta.y * cos_phi + delta.x * sin_phi);

        (Vec2D::new(x3, y3), Vec2D::new(x4, y4))
    }
}

impl Drawable for Arrow {
    fn draw(&self, cx: &Context, _surface: &ImageSurface) -> Result<()> {
        let end = match self.end {
            Some(e) => e,
            None => return Ok(()), // exit if no end
        };

        let (p1, p2) = self.get_arrow_head_points();
        let (r, g, b) = self.style.color.to_rgb_f64();

        cx.save()?;

        cx.set_line_width(self.style.size.to_line_width());
        cx.set_source_rgb(r, g, b);

        // base line
        cx.move_to(self.start.x, self.start.y);
        cx.line_to(end.x, end.y);

        // arrow-arms
        cx.move_to(p1.x, p1.y);
        cx.line_to(end.x, end.y);
        cx.line_to(p2.x, p2.y);

        // draw!
        cx.stroke()?;

        cx.restore()?;

        Ok(())
    }
}
