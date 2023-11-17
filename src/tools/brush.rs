use std::f64::consts::PI;

use crate::{
    math::Vec2D,
    sketch_board::{MouseButton, MouseEventMsg, MouseEventType},
    style::Style,
};

use super::{Drawable, DrawableClone, Tool, ToolUpdateResult};

#[derive(Default)]
pub struct BrushTool {
    drawable: Option<BrushDrawable>,
    style: Style,
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
        cx: &pangocairo::cairo::Context,
        _surface: &pangocairo::cairo::ImageSurface,
    ) -> anyhow::Result<()> {
        let (r, g, b, a) = self.style.color.to_rgba_f64();

        cx.save()?;

        cx.set_line_width(self.style.size.to_line_width());
        cx.set_source_rgba(r, g, b, a);
        cx.set_line_join(pangocairo::cairo::LineJoin::Bevel);

        if self.points.len() == 0 {
            cx.arc(
                self.start.x,
                self.start.y,
                self.style.size.to_line_width(),
                0.0,
                2.0 * PI,
            );
            cx.fill()?;
        } else if self.points.len() > 0 {
            cx.move_to(self.start.x, self.start.y);

            for p in &self.points {
                cx.line_to(self.start.x + p.x, self.start.y + p.y);
            }
            cx.stroke()?;
        }

        cx.restore()?;

        Ok(())
    }
}

impl Tool for BrushTool {
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
