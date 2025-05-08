use femtovg::{FontId, Path};

use crate::{
    math::Vec2D,
    sketch_board::{MouseButton, MouseEventMsg, MouseEventType},
    style::Style,
};

use super::{Drawable, DrawableClone, Tool, ToolUpdateResult, Tools};

#[derive(Default)]
pub struct BrushTool {
    drawable: Option<BrushDrawable>,
    style: Style,
    input_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct BrushDrawable {
    start: Vec2D,
    points: Vec<Vec2D>,
    style: Style,
}

impl Drawable for BrushDrawable {
    fn draw(
        &self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        _font: FontId,
    ) -> anyhow::Result<()> {
        canvas.save();
        let mut path = Path::new();

        if !self.points.is_empty() {
            path.move_to(self.start.x, self.start.y);
            for p in &self.points {
                path.line_to(self.start.x + p.x, self.start.y + p.y);
            }

            canvas.stroke_path(&path, &self.style.into());
        }
        canvas.restore();
        Ok(())
    }
}

impl Tool for BrushTool {
    fn input_enabled(&self) -> bool {
        self.input_enabled
    }

    fn set_input_enabled(&mut self, value: bool) {
        self.input_enabled = value;
    }

    fn get_tool_type(&self) -> super::Tools {
        Tools::Brush
    }

    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        match event.type_ {
            MouseEventType::BeginDrag => {
                self.drawable = Some(BrushDrawable {
                    start: event.pos,
                    points: Vec::new(),
                    style: self.style,
                });

                ToolUpdateResult::Redraw
            }
            MouseEventType::EndDrag => {
                if let Some(brush) = &mut self.drawable {
                    // add last point
                    brush.points.push(event.pos);

                    // commit
                    let result = brush.clone_box();
                    self.drawable = None;

                    ToolUpdateResult::Commit(result)
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            MouseEventType::UpdateDrag => {
                if let Some(brush) = &mut self.drawable {
                    // add point
                    brush.points.push(event.pos);

                    ToolUpdateResult::Redraw
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            MouseEventType::Click => {
                if event.button == MouseButton::Primary {
                    let brush = Box::new(BrushDrawable {
                        start: event.pos,
                        points: Vec::new(),
                        style: self.style,
                    });
                    ToolUpdateResult::Commit(brush)
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
        }
    }

    fn get_drawable(&self) -> Option<&dyn Drawable> {
        match &self.drawable {
            Some(d) => Some(d),
            None => None,
        }
    }

    fn handle_style_event(&mut self, style: Style) -> ToolUpdateResult {
        self.style = style;
        ToolUpdateResult::Unmodified
    }
}
