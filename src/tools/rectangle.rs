use anyhow::Result;
use pangocairo::cairo::{Context, ImageSurface};
use relm4::gtk::gdk::Key;

use crate::{math::Vec2D, sketch_board::MouseEventMsg, style::Style};

use super::{Drawable, DrawableClone, Tool, ToolUpdateResult};

#[derive(Clone, Copy, Debug)]
pub struct Rectangle {
    top_left: Vec2D,
    size: Option<Vec2D>,
    style: Style,
}

impl Drawable for Rectangle {
    fn draw(&self, cx: &Context, _surface: &ImageSurface) -> Result<()> {
        let size = match self.size {
            Some(s) => s,
            None => return Ok(()), // early exit if none
        };

        let (r, g, b) = self.style.color.to_rgb_f64();

        cx.save()?;

        // set style
        cx.set_line_width(self.style.size.to_line_width());
        cx.set_source_rgb(r, g, b);

        // make rect
        cx.rectangle(self.top_left.x, self.top_left.y, size.x, size.y);

        // draw
        cx.stroke()?;

        cx.restore()?;

        Ok(())
    }
}

#[derive(Default)]
pub struct RectangleTool {
    rectangle: Option<Rectangle>,
    style: Style,
}

impl Tool for RectangleTool {
    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        match event {
            MouseEventMsg::BeginDrag(pos) => {
                // start new
                self.rectangle = Some(Rectangle {
                    top_left: pos,
                    size: None,
                    style: self.style,
                });

                ToolUpdateResult::Redraw
            }
            MouseEventMsg::EndDrag(dir) => {
                if let Some(a) = &mut self.rectangle {
                    if dir == Vec2D::zero() {
                        self.rectangle = None;

                        ToolUpdateResult::Redraw
                    } else {
                        a.size = Some(dir);
                        let result = a.clone_box();
                        self.rectangle = None;

                        ToolUpdateResult::Commit(result)
                    }
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            MouseEventMsg::UpdateDrag(dir) => {
                if let Some(a) = &mut self.rectangle {
                    if dir == Vec2D::zero() {
                        return ToolUpdateResult::Unmodified;
                    }
                    a.size = Some(dir);

                    ToolUpdateResult::Redraw
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            _ => ToolUpdateResult::Unmodified,
        }
    }

    fn handle_key_event(&mut self, event: crate::sketch_board::KeyEventMsg) -> ToolUpdateResult {
        if event.key == Key::Escape && self.rectangle.is_some() {
            self.rectangle = None;
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
        match &self.rectangle {
            Some(d) => Some(d),
            None => None,
        }
    }
}
