use anyhow::Result;
use femtovg::{FontId, Paint, Path, TextMetrics};
use relm4::gtk::{
    gdk::{Key, ModifierType},
    TextBuffer,
};
use std::borrow::Cow;

use relm4::gtk::prelude::*;

use crate::{
    math::Vec2D,
    sketch_board::{KeyEventMsg, MouseButton, MouseEventMsg, MouseEventType, TextEventMsg},
    style::Style,
};

use super::{Drawable, DrawableClone, Tool, ToolUpdateResult, Tools};

#[derive(Clone, Debug)]
pub struct Text {
    pos: Vec2D,
    editing: bool,
    text_buffer: TextBuffer,
    style: Style,
    preedit: Option<Preedit>,
}

#[derive(Clone, Debug)]
struct Preedit {
    text: String,
    cursor: Option<usize>,
}

impl Text {
    fn new(pos: Vec2D, style: Style) -> Self {
        let text_buffer = TextBuffer::new(None);
        text_buffer.set_enable_undo(true);

        Self {
            pos,
            text_buffer,
            editing: true,
            style,
            preedit: None,
        }
    }

    fn byte_index_from_char_index(text: &str, char_index: usize) -> usize {
        text.char_indices()
            .nth(char_index)
            .map(|(idx, _)| idx)
            .unwrap_or_else(|| text.len())
    }

    fn display_text<'a>(&self, base_text: &'a str) -> (Cow<'a, str>, usize) {
        let cursor_char_index = self.text_buffer.cursor_position() as usize;
        let base_cursor_byte = Self::byte_index_from_char_index(base_text, cursor_char_index);

        if self.editing {
            if let Some(preedit) = &self.preedit {
                if preedit.text.is_empty() {
                    return (Cow::Borrowed(base_text), base_cursor_byte);
                }

                let mut composed = String::with_capacity(base_text.len() + preedit.text.len());
                composed.push_str(&base_text[..base_cursor_byte]);
                composed.push_str(&preedit.text);
                composed.push_str(&base_text[base_cursor_byte..]);

                let preedit_char_len = preedit.text.chars().count();
                let cursor_chars = preedit
                    .cursor
                    .map(|value| value.min(preedit_char_len))
                    .unwrap_or(preedit_char_len);
                let preedit_cursor_byte =
                    Self::byte_index_from_char_index(&preedit.text, cursor_chars);
                let composed_cursor_byte = base_cursor_byte + preedit_cursor_byte;

                (Cow::Owned(composed), composed_cursor_byte)
            } else {
                (Cow::Borrowed(base_text), base_cursor_byte)
            }
        } else {
            (Cow::Borrowed(base_text), base_cursor_byte)
        }
    }
}

impl Drawable for Text {
    fn draw(
        &self,
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        font: FontId,
        _bounds: (Vec2D, Vec2D),
    ) -> Result<()> {
        let gtext = self.text_buffer.text(
            &self.text_buffer.start_iter(),
            &self.text_buffer.end_iter(),
            false,
        );
        let base_text = gtext.as_str();
        let (display_text, cursor_byte_pos) = self.display_text(base_text);
        let text = display_text.as_ref();

        let mut paint: Paint = self.style.into();
        paint.set_font(&[font]);

        // get some metrics
        let canva_scale = canvas.transform().average_scale();
        let canvas_offset_x = canvas.transform()[4];
        let canvas_width = canvas.width() as f32;

        let width = canvas_width / canva_scale - self.pos.x - canvas_offset_x;
        let mut y = self.pos.y;
        let mut metrics = Vec::<TextMetrics>::new();

        let lines = canvas.break_text_vec(width, text, &paint)?;

        let font_metrics = canvas.measure_font(&paint)?;
        let measured_cursor = canvas
            .measure_text(self.pos.x, self.pos.y, "|", &paint)
            .ok();

        let mut line_height = measured_cursor
            .as_ref()
            .map(|metrics| metrics.height())
            .unwrap_or(0.0);
        if line_height <= 0.0 {
            let ascender_plus_descender = font_metrics.ascender() + font_metrics.descender();
            if ascender_plus_descender.abs() > f32::EPSILON {
                line_height = ascender_plus_descender.abs() / canva_scale;
            }
        }
        if line_height <= 0.0 {
            line_height = font_metrics.height() / canva_scale;
        }

        let cursor_top_offset = measured_cursor
            .as_ref()
            .map(|metrics| metrics.y - self.pos.y)
            .unwrap_or(-font_metrics.ascender() / canva_scale);
        let cursor_height = if line_height.abs() > f32::EPSILON {
            line_height.abs()
        } else {
            // reasonable default when all metrics fail
            (font_metrics.height() / canva_scale).abs()
        };

        for line_range in lines {
            if let Ok(text_metrics) = canvas.fill_text(self.pos.x, y, &text[line_range], &paint) {
                y += line_height;
                metrics.push(text_metrics);
            }
        }
        if self.editing {
            // function to draw a cursor
            let mut draw_cursor = |x, y: f32, height| {
                // 20% extra height for cursor w.r.t. font height
                let extra_height = height * 0.1;

                let mut path = Path::new();
                path.move_to(x, y - extra_height);
                path.line_to(x, y + height + 2.0 * extra_height);
                canvas.fill_path(&path, &paint);
            };

            // find cursor pos in broken text
            let mut previous_lines_bytes_offset = 0;
            let mut cursor_drawn = false;

            for m in metrics.iter() {
                for g in &m.glyphs {
                    if previous_lines_bytes_offset + g.byte_index == cursor_byte_pos {
                        // cursor is before this glyph, draw here!
                        draw_cursor(g.x - g.bearing_x, m.y, m.height());
                        cursor_drawn = true;
                        break;
                    }
                }

                let last_byte = match m.glyphs.last() {
                    Some(g) => g.byte_index + 1,
                    None => 0,
                };

                previous_lines_bytes_offset += last_byte;
            }

            if !cursor_drawn {
                // cursor is after last char, draw there!
                if let Some(m) = metrics.last() {
                    if let Some(g) = m.glyphs.last() {
                        if g.c == '\n' {
                            // if last char is a manual wrap -> draw cursor on next line
                            let baseline = self.pos.y + metrics.len() as f32 * line_height;
                            let next_cursor_top = baseline + cursor_top_offset;
                            draw_cursor(self.pos.x, next_cursor_top, cursor_height);
                        } else {
                            // on the same line as last glyph
                            draw_cursor(g.x + g.bearing_x + g.width, m.y, m.height());
                        }
                    }
                } else {
                    // no text rendered so far
                    draw_cursor(self.pos.x, self.pos.y + cursor_top_offset, cursor_height);
                }
            }
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct TextTool {
    text: Option<Text>,
    style: Style,
    input_enabled: bool,
}

impl Tool for TextTool {
    fn get_tool_type(&self) -> super::Tools {
        Tools::Text
    }

    fn input_enabled(&self) -> bool {
        self.input_enabled
    }

    fn set_input_enabled(&mut self, value: bool) {
        self.input_enabled = value;
    }

    fn get_drawable(&self) -> Option<&dyn Drawable> {
        match &self.text {
            Some(d) => Some(d),
            None => None,
        }
    }

    fn handle_style_event(&mut self, style: Style) -> ToolUpdateResult {
        self.style = style;
        if let Some(t) = &mut self.text {
            t.style = style;
            ToolUpdateResult::Redraw
        } else {
            ToolUpdateResult::Unmodified
        }
    }

    fn handle_text_event(&mut self, event: crate::sketch_board::TextEventMsg) -> ToolUpdateResult {
        if let Some(t) = &mut self.text {
            match event {
                TextEventMsg::Commit(text) => {
                    t.preedit = None;
                    t.text_buffer.insert_at_cursor(&text);
                    ToolUpdateResult::Redraw
                }
                TextEventMsg::Preedit { text, cursor } => {
                    let cursor = cursor.map(|value| value as usize);
                    if text.is_empty() {
                        if t.preedit.take().is_some() {
                            ToolUpdateResult::Redraw
                        } else {
                            ToolUpdateResult::Unmodified
                        }
                    } else {
                        t.preedit = Some(Preedit { text, cursor });
                        ToolUpdateResult::Redraw
                    }
                }
                TextEventMsg::PreeditEnd => {
                    if t.preedit.take().is_some() {
                        ToolUpdateResult::Redraw
                    } else {
                        ToolUpdateResult::Unmodified
                    }
                }
            }
        } else {
            ToolUpdateResult::Unmodified
        }
    }

    fn handle_key_event(&mut self, event: KeyEventMsg) -> ToolUpdateResult {
        if let Some(t) = &mut self.text {
            if event.key == Key::Return {
                if event.modifier == ModifierType::SHIFT_MASK {
                    t.text_buffer.insert_at_cursor("\n");
                    return ToolUpdateResult::Redraw;
                } else {
                    t.preedit = None;
                    t.editing = false;
                    let result = t.clone_box();
                    self.text = None;
                    self.input_enabled = false;
                    return ToolUpdateResult::Commit(result);
                }
            } else if event.key == Key::Escape {
                return self.handle_deactivated();
            } else if event.key == Key::BackSpace {
                if event.modifier == ModifierType::CONTROL_MASK {
                    return Self::handle_text_buffer_action(
                        &mut t.text_buffer,
                        Action::Delete,
                        ActionScope::BackwardWord,
                    );
                } else {
                    return Self::handle_text_buffer_action(
                        &mut t.text_buffer,
                        Action::Delete,
                        ActionScope::BackwardChar,
                    );
                }
            } else if event.key == Key::Delete {
                if event.modifier == ModifierType::CONTROL_MASK {
                    return Self::handle_text_buffer_action(
                        &mut t.text_buffer,
                        Action::Delete,
                        ActionScope::ForwardWord,
                    );
                } else {
                    return Self::handle_text_buffer_action(
                        &mut t.text_buffer,
                        Action::Delete,
                        ActionScope::ForwardChar,
                    );
                }
            } else if event.key == Key::Left {
                if event.modifier == ModifierType::CONTROL_MASK {
                    return Self::handle_text_buffer_action(
                        &mut t.text_buffer,
                        Action::MoveCursor,
                        ActionScope::BackwardWord,
                    );
                } else {
                    return Self::handle_text_buffer_action(
                        &mut t.text_buffer,
                        Action::MoveCursor,
                        ActionScope::BackwardChar,
                    );
                }
            } else if event.key == Key::Right {
                if event.modifier == ModifierType::CONTROL_MASK {
                    return Self::handle_text_buffer_action(
                        &mut t.text_buffer,
                        Action::MoveCursor,
                        ActionScope::ForwardWord,
                    );
                } else {
                    return Self::handle_text_buffer_action(
                        &mut t.text_buffer,
                        Action::MoveCursor,
                        ActionScope::ForwardChar,
                    );
                }
            } else if event.key == Key::Home {
                if event.modifier == ModifierType::CONTROL_MASK {
                    return Self::handle_text_buffer_action(
                        &mut t.text_buffer,
                        Action::MoveCursor,
                        ActionScope::BufferStart,
                    );
                } else {
                    return Self::handle_text_buffer_action(
                        &mut t.text_buffer,
                        Action::MoveCursor,
                        ActionScope::BackwardLine,
                    );
                }
            } else if event.key == Key::End {
                if event.modifier == ModifierType::CONTROL_MASK {
                    return Self::handle_text_buffer_action(
                        &mut t.text_buffer,
                        Action::MoveCursor,
                        ActionScope::BufferEnd,
                    );
                } else {
                    return Self::handle_text_buffer_action(
                        &mut t.text_buffer,
                        Action::MoveCursor,
                        ActionScope::ForwardLine,
                    );
                }
            }
        };
        ToolUpdateResult::Unmodified
    }

    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        match event.type_ {
            MouseEventType::Click => {
                if event.button == MouseButton::Primary {
                    // create commit message if necessary
                    let return_value = match &mut self.text {
                        Some(l) => {
                            l.preedit = None;
                            l.editing = false;
                            ToolUpdateResult::Commit(l.clone_box())
                        }
                        None => ToolUpdateResult::Redraw,
                    };

                    // create a new Text
                    self.text = Some(Text::new(event.pos, self.style));

                    self.set_input_enabled(true);

                    return_value
                } else {
                    self.set_input_enabled(false);
                    ToolUpdateResult::Unmodified
                }
            }
            _ => ToolUpdateResult::Unmodified,
        }
    }

    fn handle_deactivated(&mut self) -> ToolUpdateResult {
        self.input_enabled = false;
        if let Some(t) = &mut self.text {
            t.preedit = None;
            t.editing = false;
            let result = t.clone_box();
            self.text = None;
            ToolUpdateResult::Commit(result)
        } else {
            ToolUpdateResult::Unmodified
        }
    }

    fn active(&self) -> bool {
        self.text.is_some()
    }

    fn handle_undo(&mut self) -> ToolUpdateResult {
        if let Some(t) = &self.text {
            t.text_buffer.undo();
            ToolUpdateResult::Redraw
        } else {
            ToolUpdateResult::Unmodified
        }
    }

    fn handle_redo(&mut self) -> ToolUpdateResult {
        if let Some(t) = &self.text {
            t.text_buffer.redo();
            ToolUpdateResult::Redraw
        } else {
            ToolUpdateResult::Unmodified
        }
    }
}
enum ActionScope {
    ForwardChar,
    BackwardChar,
    ForwardLine,
    BackwardLine,
    ForwardWord,
    BackwardWord,
    BufferStart,
    BufferEnd,
}

enum Action {
    Delete,
    MoveCursor,
}

impl TextTool {
    fn handle_text_buffer_action(
        text_buffer: &mut TextBuffer,
        action: Action,
        action_scope: ActionScope,
    ) -> ToolUpdateResult {
        let mut start_cursor_itr = text_buffer.iter_at_mark(&text_buffer.get_insert());

        match action {
            Action::Delete => {
                let mut end_cursor_itr = start_cursor_itr;

                match action_scope {
                    ActionScope::ForwardChar => end_cursor_itr.forward_char(),
                    ActionScope::BackwardChar => end_cursor_itr.backward_char(),
                    ActionScope::ForwardWord => end_cursor_itr.forward_word_end(),
                    ActionScope::BackwardWord => end_cursor_itr.backward_word_start(),
                    _ => false, // should normally be whether movement was possible, but it's not used anyway
                };

                if text_buffer.delete_interactive(&mut start_cursor_itr, &mut end_cursor_itr, true)
                {
                    ToolUpdateResult::Redraw
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            Action::MoveCursor => {
                let mut cursor_itr = start_cursor_itr;
                match action_scope {
                    ActionScope::ForwardChar => cursor_itr.forward_char(),
                    ActionScope::BackwardChar => cursor_itr.backward_char(),
                    ActionScope::ForwardLine => cursor_itr.forward_to_line_end(),
                    ActionScope::ForwardWord => cursor_itr.forward_word_end(),
                    ActionScope::BackwardWord => cursor_itr.backward_word_start(),
                    ActionScope::BackwardLine => {
                        if cursor_itr.starts_line() {
                            cursor_itr.backward_line()
                        } else {
                            while !cursor_itr.starts_line() {
                                cursor_itr.backward_char();
                            }
                            false
                        }
                    }
                    ActionScope::BufferEnd => {
                        cursor_itr.forward_to_end();
                        false
                    }
                    ActionScope::BufferStart => {
                        while !cursor_itr.is_start() {
                            cursor_itr.backward_line();
                        }
                        false
                    }
                };

                text_buffer.place_cursor(&cursor_itr);
                let new_cursor_itr = text_buffer.iter_at_mark(&text_buffer.get_insert());

                if new_cursor_itr != start_cursor_itr {
                    ToolUpdateResult::Redraw
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
        }
    }
}
