use anyhow::Result;
use femtovg::{Paint, Path};

use relm4::gtk::gdk::Key;

use crate::{
    math::{self, Vec2D},
    sketch_board::{MouseEventMsg, MouseEventType},
    style::{Size, Style},
};

use super::{Drawable, DrawableClone, Tool, ToolUpdateResult};

#[derive(Clone, Debug)]
pub struct Highlight {
    top_left: Vec2D,
    size: Option<Vec2D>,
    style: Style,
    editing: bool,
}

impl Drawable for Highlight {
    fn draw(
        &self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        _font: femtovg::FontId,
    ) -> Result<()> {
        let size = match self.size {
            Some(s) => s,
            None => return Ok(()), // early exit if size is none
        };

        let (pos, size) = math::rect_ensure_positive_size(self.top_left, size);

        if self.editing {
            // include a border when selecting an area.
            let border_paint =
                Paint::color(self.style.color.into()).with_line_width(Size::Small.to_line_width());
            let mut border_path = Path::new();
            border_path.rect(pos.x, pos.y, size.x, size.y);
            canvas.stroke_path(&border_path, &border_paint);
        }

        let mut shadow_path = Path::new();
        shadow_path.rect(pos.x, pos.y, size.x, size.y);

        let shadow_paint = Paint::color(femtovg::Color::rgba(
            self.style.color.r,
            self.style.color.g,
            self.style.color.b,
            self.style.size.to_highlight_opacity(),
        ));

        canvas.fill_path(&shadow_path, &shadow_paint);
        Ok(())
    }
}

#[derive(Default)]
pub struct HighlightTool {
    highlight: Option<Highlight>,
    style: Style,
}

impl Tool for HighlightTool {
    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        match event.type_ {
            MouseEventType::BeginDrag => {
                self.highlight = Some(Highlight {
                    top_left: event.pos,
                    size: None,
                    style: self.style,
                    editing: true,
                });

                ToolUpdateResult::Redraw
            }
            MouseEventType::EndDrag => {
                if let Some(a) = &mut self.highlight {
                    if event.pos == Vec2D::zero() {
                        self.highlight = None;

                        ToolUpdateResult::Redraw
                    } else {
                        a.size = Some(event.pos);
                        a.editing = false;

                        let result = a.clone_box();
                        self.highlight = None;

                        ToolUpdateResult::Commit(result)
                    }
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            MouseEventType::UpdateDrag => {
                if let Some(a) = &mut self.highlight {
                    if event.pos == Vec2D::zero() {
                        return ToolUpdateResult::Unmodified;
                    }
                    a.size = Some(event.pos);

                    ToolUpdateResult::Redraw
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            _ => ToolUpdateResult::Unmodified,
        }
    }

    fn handle_key_event(&mut self, event: crate::sketch_board::KeyEventMsg) -> ToolUpdateResult {
        if event.key == Key::Escape && self.highlight.is_some() {
            self.highlight = None;
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
        match &self.highlight {
            Some(d) => Some(d),
            None => None,
        }
    }
}
