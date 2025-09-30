use std::cell::Cell;

use anyhow::Result;
use femtovg::{FontId, Paint, Path};
use relm4::gtk::{
    gdk::{Key, ModifierType},
    TextBuffer,
};

use relm4::gtk::prelude::*;

use crate::{
    math::Vec2D,
    sketch_board::{KeyEventMsg, MouseButton, MouseEventMsg, MouseEventType, TextEventMsg},
    style::Style,
};

use super::{Drawable, DrawableClone, Tool, ToolUpdateResult};

#[derive(Clone, Debug)]
pub struct Text {
    pos: Vec2D,
    editing: bool,
    text_buffer: TextBuffer,
    style: Style,
    preedit_text: String,
    preedit_start_offset: Option<i32>,
    preedit_in_progress: bool,
    cursor_rect: Cell<Option<(f32, f32, f32)>>,
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
            preedit_text: String::new(),
            preedit_start_offset: None,
            preedit_in_progress: false,
            cursor_rect: Cell::new(None),
        }
    }

    fn begin_preedit(&mut self) {
        if !self.preedit_in_progress {
            self.preedit_start_offset = Some(self.text_buffer.cursor_position());
            self.text_buffer.begin_user_action();
            self.preedit_in_progress = true;
        }
        self.preedit_text.clear();
    }

    fn update_preedit(&mut self, text: &str, cursor_pos: i32) {
        if self.preedit_start_offset.is_none() {
            self.begin_preedit();
        }

        if let Some(offset) = self.preedit_start_offset {
            let existing_len = self.preedit_text.chars().count() as i32;
            if existing_len > 0 {
                let mut start_iter = self.text_buffer.iter_at_offset(offset);
                let mut end_iter = self.text_buffer.iter_at_offset(offset + existing_len);
                let _ = self
                    .text_buffer
                    .delete_interactive(&mut start_iter, &mut end_iter, true);
            }

            if text.is_empty() {
                self.preedit_text.clear();
                let cursor_iter = self.text_buffer.iter_at_offset(offset);
                self.text_buffer.place_cursor(&cursor_iter);
                return;
            }

            let mut insert_iter = self.text_buffer.iter_at_offset(offset);
            self.text_buffer.insert(&mut insert_iter, text);
            self.preedit_text.clear();
            self.preedit_text.push_str(text);

            let preedit_len = self.preedit_text.chars().count() as i32;
            let target_cursor = if cursor_pos < 0 {
                preedit_len
            } else {
                cursor_pos.clamp(0, preedit_len)
            };

            let cursor_iter = self.text_buffer.iter_at_offset(offset + target_cursor);
            self.text_buffer.place_cursor(&cursor_iter);
        }
    }

    fn end_preedit(&mut self) {
        if let Some(offset) = self.preedit_start_offset {
            let existing_len = self.preedit_text.chars().count() as i32;
            if existing_len > 0 {
                let mut start_iter = self.text_buffer.iter_at_offset(offset);
                let mut end_iter = self.text_buffer.iter_at_offset(offset + existing_len);
                if self
                    .text_buffer
                    .delete_interactive(&mut start_iter, &mut end_iter, true)
                {
                    self.text_buffer.place_cursor(&start_iter);
                }
            } else {
                let cursor_iter = self.text_buffer.iter_at_offset(offset);
                self.text_buffer.place_cursor(&cursor_iter);
            }
        }

        if self.preedit_in_progress {
            self.text_buffer.end_user_action();
            self.preedit_in_progress = false;
        }

        self.preedit_text.clear();
        self.preedit_start_offset = None;
    }

    fn commit_preedit(&mut self, text: &str) {
        self.end_preedit();
        if !text.is_empty() {
            self.text_buffer.insert_at_cursor(text);
        }
    }

    fn ime_cursor_rect(&self) -> Option<(f32, f32, f32)> {
        self.cursor_rect.get()
    }

    fn draw_cursor(
        canvas: &mut femtovg::Canvas<femtovg::renderer::OpenGl>,
        paint: &Paint,
        x: f32,
        top: f32,
        height: f32,
    ) -> (f32, f32) {
        let extra_height = height * 0.1;

        let mut path = Path::new();
        path.move_to(x, top - extra_height);
        path.line_to(x, top + height + 2.0 * extra_height);
        canvas.fill_path(&path, paint);

        let cursor_top = top - extra_height;
        let cursor_bottom = top + height + 2.0 * extra_height;
        (cursor_top, cursor_bottom)
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
        let text = gtext.as_str();

        let mut paint: Paint = self.style.into();
        paint.set_font(&[font]);

        // get some metrics
        let canva_scale = canvas.transform().average_scale();
        let canvas_offset_x = canvas.transform()[4];
        let canvas_width = canvas.width() as f32;

        let width = canvas_width / canva_scale - self.pos.x - canvas_offset_x;
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
            line_height = (font_metrics.ascender() + font_metrics.descender()).abs() / canva_scale;
        }
        if line_height <= 0.0 {
            line_height = font_metrics.height() / canva_scale;
        }
        let mut ascender = font_metrics.ascender() / canva_scale;
        if ascender <= 0.0 {
            ascender = line_height;
        }
        let cursor_top_offset = measured_cursor
            .as_ref()
            .map(|metrics| metrics.y - self.pos.y)
            .unwrap_or(-ascender);
        let inferred_cursor_top = self.pos.y + cursor_top_offset;

        struct LineLayout<'a> {
            baseline: f32,
            cursor_top: f32,
            draw_text: &'a str,
            has_newline: bool,
            char_start: usize,
            char_count: usize,
        }

        let mut line_layouts = Vec::new();
        let mut y = self.pos.y;
        let mut char_offset = 0usize;

        for line_range in lines {
            let baseline = y;
            let cursor_top = baseline + cursor_top_offset;
            let full_slice = &text[line_range.clone()];
            let (draw_slice, has_newline) = if let Some(stripped) = full_slice.strip_suffix('\n') {
                (stripped, true)
            } else {
                (full_slice, false)
            };

            if !draw_slice.is_empty() {
                let _ = canvas.fill_text(self.pos.x, baseline, draw_slice, &paint);
            }

            let line_char_count = draw_slice.chars().count();

            line_layouts.push(LineLayout {
                baseline,
                cursor_top,
                draw_text: draw_slice,
                has_newline,
                char_start: char_offset,
                char_count: line_char_count,
            });

            char_offset += line_char_count;
            if has_newline {
                char_offset += 1;
            }

            y += line_height;
        }

        if line_layouts.is_empty() {
            line_layouts.push(LineLayout {
                baseline: self.pos.y,
                cursor_top: inferred_cursor_top,
                draw_text: "",
                has_newline: false,
                char_start: 0,
                char_count: 0,
            });
        }
        if self.editing {
            self.cursor_rect.set(None);

            let cursor_char_pos = self.text_buffer.cursor_position() as usize;

            let mut cursor_drawn = false;

            for (idx, line) in line_layouts.iter().enumerate() {
                let line_char_count = line.char_count;
                let line_start = line.char_start;
                let line_end = line_start + line_char_count;

                if cursor_char_pos < line_start {
                    continue;
                }

                if cursor_char_pos < line_end {
                    let local_index = cursor_char_pos - line_start;
                    let byte_offset = line
                        .draw_text
                        .char_indices()
                        .nth(local_index)
                        .map(|(i, _)| i)
                        .unwrap_or_else(|| line.draw_text.len());

                    let substring = &line.draw_text[..byte_offset];
                    let cursor_x = canvas
                        .measure_text(self.pos.x, line.baseline, substring, &paint)
                        .ok()
                        .map(|m| m.x + m.width())
                        .unwrap_or(self.pos.x);

                    let (top, bottom) =
                        Text::draw_cursor(canvas, &paint, cursor_x, line.cursor_top, line_height);
                    self.cursor_rect.set(Some((cursor_x, top, bottom - top)));
                    cursor_drawn = true;
                    break;
                }

                if cursor_char_pos == line_end {
                    if line.has_newline {
                        let next_baseline = line_layouts
                            .get(idx + 1)
                            .map(|next| next.baseline)
                            .unwrap_or(line.baseline + line_height);
                        let next_cursor_top = line_layouts
                            .get(idx + 1)
                            .map(|next| next.cursor_top)
                            .unwrap_or(next_baseline + cursor_top_offset);
                        let (top, bottom) = Text::draw_cursor(
                            canvas,
                            &paint,
                            self.pos.x,
                            next_cursor_top,
                            line_height,
                        );
                        self.cursor_rect.set(Some((self.pos.x, top, bottom - top)));
                        cursor_drawn = true;
                        break;
                    } else {
                        let cursor_x = canvas
                            .measure_text(self.pos.x, line.baseline, line.draw_text, &paint)
                            .ok()
                            .map(|m| m.x + m.width())
                            .unwrap_or(self.pos.x);
                        let (top, bottom) = Text::draw_cursor(
                            canvas,
                            &paint,
                            cursor_x,
                            line.cursor_top,
                            line_height,
                        );
                        self.cursor_rect.set(Some((cursor_x, top, bottom - top)));
                        cursor_drawn = true;
                        break;
                    }
                }
            }

            if !cursor_drawn {
                if let Some(last_line) = line_layouts.last() {
                    let (cursor_top, cursor_x) = if last_line.has_newline {
                        (
                            last_line.baseline + line_height + cursor_top_offset,
                            self.pos.x,
                        )
                    } else {
                        let cursor_x = canvas
                            .measure_text(
                                self.pos.x,
                                last_line.baseline,
                                last_line.draw_text,
                                &paint,
                            )
                            .ok()
                            .map(|m| m.x + m.width())
                            .unwrap_or(self.pos.x);
                        (last_line.cursor_top, cursor_x)
                    };

                    let (top, bottom) =
                        Text::draw_cursor(canvas, &paint, cursor_x, cursor_top, line_height);
                    self.cursor_rect.set(Some((cursor_x, top, bottom - top)));
                } else {
                    let (top, bottom) = Text::draw_cursor(
                        canvas,
                        &paint,
                        self.pos.x,
                        inferred_cursor_top,
                        line_height,
                    );
                    self.cursor_rect.set(Some((self.pos.x, top, bottom - top)));
                }
            }
        } else {
            self.cursor_rect.set(None);
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct TextTool {
    text: Option<Text>,
    style: Style,
}

impl Tool for TextTool {
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
                    t.commit_preedit(&text);
                    ToolUpdateResult::Redraw
                }
                TextEventMsg::PreeditStart => {
                    t.begin_preedit();
                    ToolUpdateResult::Unmodified
                }
                TextEventMsg::PreeditChanged { text, cursor_pos } => {
                    t.update_preedit(&text, cursor_pos);
                    ToolUpdateResult::Redraw
                }
                TextEventMsg::PreeditEnd => {
                    t.end_preedit();
                    ToolUpdateResult::Redraw
                }
            }
        } else {
            ToolUpdateResult::Unmodified
        }
    }

    fn handle_key_event(&mut self, event: KeyEventMsg) -> ToolUpdateResult {
        if let Some(t) = &mut self.text {
            if matches!(event.key, Key::Return | Key::KP_Enter) {
                let modifiers = event.modifier;
                let shift_pressed = modifiers.contains(ModifierType::SHIFT_MASK);

                if shift_pressed {
                    t.end_preedit();
                    t.text_buffer.insert_at_cursor("\n");
                    return ToolUpdateResult::Redraw;
                } else {
                    t.end_preedit();
                    t.editing = false;
                    let result = t.clone_box();
                    self.text = None;
                    return ToolUpdateResult::Commit(result);
                }
            } else if event.key == Key::Escape {
                t.end_preedit();
                self.text = None;
                return ToolUpdateResult::Redraw;
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
                let mut cursor_itr = t.text_buffer.iter_at_mark(&t.text_buffer.get_insert());
                cursor_itr.backward_line();
                t.text_buffer.place_cursor(&cursor_itr);
                return ToolUpdateResult::Redraw;
            } else if event.key == Key::End {
                let mut cursor_itr = t.text_buffer.iter_at_mark(&t.text_buffer.get_insert());
                cursor_itr.forward_line();
                t.text_buffer.place_cursor(&cursor_itr);
                return ToolUpdateResult::Redraw;
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
                            l.end_preedit();
                            l.editing = false;
                            ToolUpdateResult::Commit(l.clone_box())
                        }
                        None => ToolUpdateResult::Redraw,
                    };

                    // create a new Text
                    self.text = Some(Text::new(event.pos, self.style));

                    return_value
                } else {
                    ToolUpdateResult::Unmodified
                }
            }
            _ => ToolUpdateResult::Unmodified,
        }
    }

    fn handle_deactivated(&mut self) -> ToolUpdateResult {
        if let Some(t) = &mut self.text {
            t.end_preedit();
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

    fn ime_cursor_rect(&self) -> Option<(f32, f32, f32)> {
        self.text.as_ref().and_then(Text::ime_cursor_rect)
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
    ForwardWord,
    BackwardWord,
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
                    ActionScope::ForwardWord => cursor_itr.forward_word_end(),
                    ActionScope::BackwardWord => cursor_itr.backward_word_start(),
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
