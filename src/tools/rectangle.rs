use anyhow::Result;
use femtovg::{FontId, Path};
use relm4::gtk::gdk::{Key, ModifierType};

use crate::{
    configuration::APP_CONFIG,
    math::Vec2D,
    sketch_board::{MouseEventMsg, MouseEventType},
    style::Style,
};

use super::{Drawable, DrawableClone, Tool, ToolUpdateResult, Tools};

#[derive(Clone, Copy, Debug)]
pub struct Rectangle {
    top_left: Vec2D,
    size: Option<Vec2D>,
    style: Style,
}

impl Drawable for Rectangle {
    fn draw(
        &self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        _font: FontId,
    ) -> Result<()> {
        let size = match self.size {
            Some(s) => s,
            None => return Ok(()), // early exit if none
        };

        canvas.save();
        let mut path = Path::new();
        path.rounded_rect(
            self.top_left.x,
            self.top_left.y,
            size.x,
            size.y,
            APP_CONFIG.read().corner_roundness(),
        );

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
pub struct RectangleTool {
    rectangle: Option<Rectangle>,
    style: Style,
    input_enabled: bool,
}

impl Tool for RectangleTool {
    fn input_enabled(&self) -> bool {
        self.input_enabled
    }

    fn set_input_enabled(&mut self, value: bool) {
        self.input_enabled = value;
    }

    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        let shift_pressed = event.modifier.intersects(ModifierType::SHIFT_MASK);
        match event.type_ {
            MouseEventType::BeginDrag => {
                // start new
                self.rectangle = Some(Rectangle {
                    top_left: event.pos,
                    size: None,
                    style: self.style,
                });

                ToolUpdateResult::Redraw
            }
            MouseEventType::EndDrag => {
                if let Some(rectangle) = &mut self.rectangle {
                    if event.pos == Vec2D::zero() {
                        self.rectangle = None;

                        ToolUpdateResult::Redraw
                    } else {
                        if shift_pressed {
                            let max_size = event.pos.x.abs().max(event.pos.y.abs());
                            rectangle.size = Some(Vec2D {
                                x: max_size * event.pos.x.signum(),
                                y: max_size * event.pos.y.signum(),
                            });
                        } else {
                            rectangle.size = Some(event.pos);
                        }
                        let result = rectangle.clone_box();
                        self.rectangle = None;

                        ToolUpdateResult::Commit(result)
                    }
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            MouseEventType::UpdateDrag => {
                if let Some(rectangle) = &mut self.rectangle {
                    if event.pos == Vec2D::zero() {
                        return ToolUpdateResult::Unmodified;
                    }
                    if shift_pressed {
                        let max_size = event.pos.x.abs().max(event.pos.y.abs());
                        rectangle.size = Some(Vec2D {
                            x: max_size * event.pos.x.signum(),
                            y: max_size * event.pos.y.signum(),
                        });
                    } else {
                        rectangle.size = Some(event.pos);
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

    fn get_tool_type(&self) -> super::Tools {
        Tools::Rectangle
    }
}
