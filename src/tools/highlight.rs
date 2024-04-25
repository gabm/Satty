use anyhow::Result;
use femtovg::{Paint, Path};

use relm4::gtk::gdk::{Key, ModifierType};

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
    points: Option<Vec<Vec2D>>,
    shift_pressed: bool,
}

impl Highlight {
    // This is triggered when a user does not press shift before highlighting.
    fn draw_free_hand(
        &self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
    ) -> Result<()> {
        canvas.save();
        let mut path = Path::new();
        if let Some(points) = &self.points {
            let first = points.first().expect("atleast one point");
            path.move_to(first.x, first.y);
            for p in points.iter().skip(1) {
                path.line_to(first.x + p.x, first.y + p.y);
            }

            let mut paint = Paint::color(femtovg::Color::rgba(
                self.style.color.r,
                self.style.color.g,
                self.style.color.b,
                (255.0 * 0.4) as u8,
            ));
            paint.set_line_width(self.style.size.to_highlight_width());

            canvas.stroke_path(&path, &paint);
        }
        canvas.restore();
        Ok(())
    }

    /// This is triggered when the user presses shift *before* highlighting.
    fn draw_aligned(&self, canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>) -> Result<()> {
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
            (255.0 * 0.4) as u8,
        ));

        canvas.fill_path(&shadow_path, &shadow_paint);
        Ok(())
    }
}

impl Drawable for Highlight {
    fn draw(
        &self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        _font: femtovg::FontId,
    ) -> Result<()> {
        if self.points.is_some() {
            self.draw_free_hand(canvas)?;
        } else {
            self.draw_aligned(canvas)?;
        }
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
        let shift_pressed = event.modifier.intersects(ModifierType::SHIFT_MASK);
        let ctrl_pressed = event.modifier.intersects(ModifierType::CONTROL_MASK);
        match event.type_ {
            MouseEventType::BeginDrag => {
                self.highlight = Some(Highlight {
                    top_left: event.pos,
                    size: None,
                    style: self.style,
                    editing: true,
                    points: if !ctrl_pressed {
                        Some(vec![event.pos])
                    } else {
                        None
                    },
                    shift_pressed,
                });

                ToolUpdateResult::Redraw
            }
            MouseEventType::EndDrag => {
                if let Some(highlight) = &mut self.highlight {
                    if event.pos == Vec2D::zero() {
                        self.highlight = None;

                        ToolUpdateResult::Redraw
                    } else {
                        if let Some(points) = &mut highlight.points {
                            if shift_pressed {
                                let last = points.last().expect("should have atleast one point");
                                points.push(Vec2D::new(event.pos.x, last.y));
                            } else {
                                points.push(event.pos);
                            }
                        }

                        highlight.shift_pressed = shift_pressed;
                        highlight.editing = false;

                        let result = highlight.clone_box();
                        self.highlight = None;

                        ToolUpdateResult::Commit(result)
                    }
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            MouseEventType::UpdateDrag => {
                if let Some(highlight) = &mut self.highlight {
                    if event.pos == Vec2D::zero() {
                        return ToolUpdateResult::Unmodified;
                    }
                    if let Some(points) = &mut highlight.points {
                        if shift_pressed {
                            let last = points.last().expect("should have atleast one point");
                            points.push(Vec2D::new(event.pos.x, last.y));
                        } else {
                            points.push(event.pos);
                        }
                    }
                    highlight.size = Some(event.pos);
                    highlight.shift_pressed = shift_pressed;

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
