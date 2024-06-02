use std::ops::{Add, Sub};

use anyhow::Result;
use femtovg::{Paint, Path};

use relm4::gtk::gdk::{Key, ModifierType};

use crate::{
    configuration::APP_CONFIG,
    math::{self, Vec2D},
    sketch_board::{MouseEventMsg, MouseEventType},
    style::Style,
    tools::DrawableClone,
};

use super::{Drawable, Tool, ToolUpdateResult};

const HIGHLIGHT_OPACITY: f64 = 0.4;

#[derive(Clone, Debug)]
struct BlockHighlight {
    top_left: Vec2D,
    size: Option<Vec2D>,
}

#[derive(Clone, Debug)]
struct LineHighlight {
    points: Vec<Vec2D>,
    shift_pressed: bool,
}

#[derive(Clone, Debug)]
struct Highlighter<T> {
    data: T,
    style: Style,
}

trait Highlight {
    fn highlight(&self, canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>) -> Result<()>;
}

impl Highlight for Highlighter<LineHighlight> {
    fn highlight(&self, canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>) -> Result<()> {
        canvas.save();

        let mut path = Path::new();
        let first = self
            .data
            .points
            .first()
            .expect("should exist atleast one point in highlight instance.");

        path.move_to(first.x, first.y);
        for p in self.data.points.iter().skip(1) {
            path.line_to(first.x + p.x, first.y + p.y);
        }

        let mut paint = Paint::color(femtovg::Color::rgba(
            self.style.color.r,
            self.style.color.g,
            self.style.color.b,
            (255.0 * HIGHLIGHT_OPACITY) as u8,
        ));
        paint.set_line_width(self.style.size.to_highlight_width());
        paint.set_line_join(femtovg::LineJoin::Round);
        paint.set_line_cap(femtovg::LineCap::Square);

        canvas.stroke_path(&path, &paint);
        canvas.restore();
        Ok(())
    }
}

impl Highlight for Highlighter<BlockHighlight> {
    fn highlight(&self, canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>) -> Result<()> {
        let size = match self.data.size {
            Some(s) => s,
            None => return Ok(()), // early exit if size is none
        };

        let (pos, size) = math::rect_ensure_positive_size(self.data.top_left, size);

        let mut shadow_path = Path::new();
        shadow_path.rect(pos.x, pos.y, size.x, size.y);

        let shadow_paint = Paint::color(femtovg::Color::rgba(
            self.style.color.r,
            self.style.color.g,
            self.style.color.b,
            (255.0 * HIGHLIGHT_OPACITY) as u8,
        ));

        canvas.fill_path(&shadow_path, &shadow_paint);
        Ok(())
    }
}

#[derive(Clone, Debug)]
enum HighlightKind {
    Block(Highlighter<BlockHighlight>),
    Line(Highlighter<LineHighlight>),
}
impl HighlightKind {
    fn highlight(&self, canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>) {
        let _ = match self {
            HighlightKind::Block(highlighter) => highlighter.highlight(canvas),
            HighlightKind::Line(highlighter) => highlighter.highlight(canvas),
        };
    }
}

#[derive(Default, Clone, Debug)]
pub struct HighlightTool {
    highlighter: Option<HighlightKind>,
    style: Style,
}

impl Drawable for HighlightKind {
    fn draw(
        &self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        _font: femtovg::FontId,
    ) -> Result<()> {
        self.highlight(canvas);
        Ok(())
    }
}

impl Tool for HighlightTool {
    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        let shift_pressed = event.modifier.intersects(ModifierType::SHIFT_MASK);
        let ctrl_pressed = event.modifier.intersects(ModifierType::CONTROL_MASK);
        let default_highlight_block = APP_CONFIG.read().default_block_highlight();
        match event.type_ {
            MouseEventType::BeginDrag => {
                match (ctrl_pressed, default_highlight_block) {
                    (false, true) | (true, false) => {
                        self.highlighter =
                            Some(HighlightKind::Block(Highlighter::<BlockHighlight> {
                                data: BlockHighlight {
                                    top_left: event.pos,
                                    size: None,
                                },
                                style: self.style,
                            }))
                    }
                    (false, false) | (true, true) => {
                        self.highlighter = Some(HighlightKind::Line(Highlighter::<LineHighlight> {
                            data: LineHighlight {
                                points: vec![event.pos],
                                shift_pressed,
                            },
                            style: self.style,
                        }))
                    }
                }

                ToolUpdateResult::Redraw
            }
            MouseEventType::UpdateDrag | MouseEventType::EndDrag => {
                if self.highlighter.is_none() {
                    return ToolUpdateResult::Unmodified;
                }
                let mut highlighter_kind = self.highlighter.as_mut().unwrap();
                let update: ToolUpdateResult = match &mut highlighter_kind {
                    HighlightKind::Block(highlighter) => {
                        if shift_pressed {
                            let max_size = event.pos.x.abs().max(event.pos.y.abs());
                            highlighter.data.size = Some(Vec2D {
                                x: max_size * event.pos.x.signum(),
                                y: max_size * event.pos.y.signum(),
                            });
                        } else {
                            highlighter.data.size = Some(event.pos);
                        };
                        ToolUpdateResult::Redraw
                    }
                    HighlightKind::Line(highlighter) => {
                        if event.pos == Vec2D::zero() {
                            return ToolUpdateResult::Unmodified;
                        };

                        if shift_pressed {
                            // if shift was pressed before we remove an extra point which would
                            // have been the previous aligned point. However ignore if there is
                            // only one point which means the highlight has just started.
                            if highlighter.data.shift_pressed && highlighter.data.points.len() >= 2
                            {
                                highlighter
                                    .data
                                    .points
                                    .pop()
                                    .expect("atleast 2 points in highlight path.");
                            };
                            // use the last point to position the snapping guide, or 0 if the point
                            // is the first one.
                            let last = if highlighter.data.points.len() == 1 {
                                Vec2D::zero()
                            } else {
                                *highlighter
                                    .data
                                    .points
                                    .last_mut()
                                    .expect("atleast one point")
                            };
                            let snapped_pos = event.pos.sub(last).snapped_vector_15deg().add(last);
                            highlighter.data.points.push(snapped_pos);
                        } else {
                            highlighter.data.points.push(event.pos);
                        }

                        highlighter.data.shift_pressed = shift_pressed;
                        ToolUpdateResult::Redraw
                    }
                };
                if event.type_ == MouseEventType::UpdateDrag {
                    return update;
                };
                let result = highlighter_kind.clone_box();
                self.highlighter = None;
                ToolUpdateResult::Commit(result)
            }

            _ => ToolUpdateResult::Unmodified,
        }
    }

    fn handle_key_event(&mut self, event: crate::sketch_board::KeyEventMsg) -> ToolUpdateResult {
        if event.key == Key::Escape && self.highlighter.is_some() {
            self.highlighter = None;
            return ToolUpdateResult::Redraw;
        }
        ToolUpdateResult::Unmodified
    }

    fn handle_key_release_event(
        &mut self,
        event: crate::sketch_board::KeyEventMsg,
    ) -> ToolUpdateResult {
        // add an extra point when shift is unheld, this allows for users to make sharper turns.
        // press (aka: release) shift a second time to remove the added point.
        if event.key == Key::Shift_L || event.key == Key::Shift_R {
            if let Some(HighlightKind::Line(highlighter)) = &mut self.highlighter {
                let points = &mut highlighter.data.points;
                let last = points
                    .last()
                    .expect("line highlight must have atleast one point");
                if points.len() >= 2 {
                    if *last == points[points.len() - 2] {
                        points.pop();
                    } else {
                        points.push(*last);
                    }
                    return ToolUpdateResult::Redraw;
                };
            };
        }
        ToolUpdateResult::Unmodified
    }

    fn handle_style_event(&mut self, style: Style) -> ToolUpdateResult {
        self.style = style;
        ToolUpdateResult::Unmodified
    }

    fn get_drawable(&self) -> Option<&dyn Drawable> {
        match &self.highlighter {
            Some(d) => Some(d),
            None => None,
        }
    }
}
