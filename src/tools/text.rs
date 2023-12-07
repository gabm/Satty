use anyhow::Result;
use pangocairo::{
    cairo::{Context, ImageSurface},
    pango::{FontDescription, SCALE},
};
use relm4::gtk::{
    gdk::{Key, ModifierType},
    TextBuffer,
};

use relm4::gtk::prelude::*;

use crate::{
    math::Vec2D,
    sketch_board::{KeyEventMsg, MouseButton, MouseEventMsg, MouseEventType},
    style::Style,
};

use super::{Drawable, DrawableClone, Tool, ToolUpdateResult};

#[derive(Clone, Debug)]
pub struct Text {
    pos: Vec2D,
    editing: bool,
    text_buffer: TextBuffer,
    style: Style,
}

impl Drawable for Text {
    fn draw(&self, cx: &Context, _surface: &ImageSurface) -> Result<()> {
        let layout = pangocairo::create_layout(cx);

        let text = self.text_buffer.text(
            &self.text_buffer.start_iter(),
            &self.text_buffer.end_iter(),
            false,
        );
        layout.set_text(text.as_str());

        let mut desc = FontDescription::from_string("Sans,Times new roman");
        desc.set_size(self.style.size.to_text_size());
        layout.set_font_description(Some(&desc));

        let (r, g, b, a) = self.style.color.to_rgba_f64();

        cx.save()?;
        cx.set_source_rgba(r, g, b, a);

        cx.move_to(self.pos.x, self.pos.y);
        pangocairo::show_layout(cx, &layout);

        if self.editing {
            // GTK is working with UTF-8 and character positions, pango is working with UTF-8 but byte positions.
            // here we transform one into the other!
            let (cursor_byte_pos, _) = text
                .char_indices()
                .nth((self.text_buffer.cursor_position()) as usize)
                .unwrap_or((text.bytes().count(), 'X'));

            let (cursor, _) = layout.cursor_pos(cursor_byte_pos as i32);

            let cursor_pos =
                self.pos + Vec2D::new((cursor.x() / SCALE) as f64, (cursor.y() / SCALE) as f64);
            let cursor_height = (cursor.height() / SCALE) as f64;

            cx.set_line_width(1.0);
            cx.move_to(cursor_pos.x, cursor_pos.y);
            cx.rel_line_to(0.0, cursor_height);
            cx.stroke()?;
        }

        cx.restore()?;

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

    fn handle_key_event(&mut self, event: KeyEventMsg) -> ToolUpdateResult {
        if let Some(t) = &mut self.text {
            if event.key == Key::Return {
                if event.modifier == ModifierType::SHIFT_MASK {
                    t.text_buffer.insert_at_cursor("\n");
                    return ToolUpdateResult::Redraw;
                } else {
                    t.editing = false;
                    let result = t.clone_box();
                    self.text = None;
                    return ToolUpdateResult::Commit(result);
                }
            } else if event.key == Key::Escape {
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
            } else if let Some(c) = event.key.to_unicode() {
                let mut buf = [0; 4];
                t.text_buffer.insert_at_cursor(c.encode_utf8(&mut buf));

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
                            l.editing = false;
                            ToolUpdateResult::Commit(l.clone_box())
                        }
                        None => ToolUpdateResult::Redraw,
                    };

                    // create a new Text
                    self.text = Some(Text {
                        pos: event.pos,
                        text_buffer: TextBuffer::new(None),
                        editing: true,
                        style: self.style,
                    });

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
            t.editing = false;
            let result = t.clone_box();
            self.text = None;
            ToolUpdateResult::Commit(result)
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
