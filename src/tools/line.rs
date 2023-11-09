use anyhow::Result;
use pangocairo::cairo::{Context, ImageSurface};
use relm4::gtk::gdk::Key;

use crate::{math::Vec2D, sketch_board::MouseEventMsg, style::Style};

use super::{Drawable, DrawableClone, Tool, ToolUpdateResult};

#[derive(Default)]
pub struct LineTool {
    line: Option<Line>,
    style: Style,
}

#[derive(Clone, Copy, Debug)]
pub struct Line {
    start: Vec2D,
    direction: Option<Vec2D>,
    style: Style,
}

impl Drawable for Line {
    fn draw(&self, cx: &Context, _surface: &ImageSurface) -> Result<()> {
        let direction = match self.direction {
            Some(d) => d,
            None => return Ok(()), // exit early if no direction
        };

        let (r, g, b, a) = self.style.color.to_rgba_f64();

        cx.save()?;

        cx.set_line_width(self.style.size.to_line_width());
        cx.set_source_rgba(r, g, b, a);
        cx.move_to(self.start.x, self.start.y);
        cx.rel_line_to(direction.x, direction.y);
        cx.stroke()?;

        cx.restore()?;

        Ok(())
    }
}

impl Tool for LineTool {
    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        match event {
            MouseEventMsg::BeginDrag(pos) => {
                // start new
                self.line = Some(Line {
                    start: pos,
                    direction: None,
                    style: self.style,
                });

                ToolUpdateResult::Redraw
            }
            MouseEventMsg::EndDrag(dir) => {
                if let Some(a) = &mut self.line {
                    if dir == Vec2D::zero() {
                        self.line = None;

                        ToolUpdateResult::Redraw
                    } else {
                        a.direction = Some(dir);
                        let result = a.clone_box();
                        self.line = None;

                        ToolUpdateResult::Commit(result)
                    }
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            MouseEventMsg::UpdateDrag(dir) => {
                if let Some(r) = &mut self.line {
                    r.direction = Some(dir);
                    ToolUpdateResult::Redraw
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            _ => ToolUpdateResult::Unmodified,
        }
    }

    fn handle_key_event(&mut self, event: crate::sketch_board::KeyEventMsg) -> ToolUpdateResult {
        if event.key == Key::Escape && self.line.is_some() {
            self.line = None;
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
        match &self.line {
            Some(d) => Some(d),
            None => None,
        }
    }
}
