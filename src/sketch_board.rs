use anyhow::anyhow;

use femtovg::imgref::Img;
use femtovg::rgb::{ComponentBytes, RGBA};
use gdk_pixbuf::glib::Bytes;
use gdk_pixbuf::Pixbuf;
use keycode::{KeyMap, KeyMappingId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::rc::Rc;

use gtk::prelude::*;

use relm4::gtk::gdk::{DisplayManager, Key, ModifierType, Texture};
use relm4::{gtk, Component, ComponentParts, ComponentSender};

use crate::configuration::{Action, APP_CONFIG};
use crate::femtovg_area::FemtoVGArea;
use crate::math::Vec2D;
use crate::notification::log_result;
use crate::style::Style;
use crate::tools::{Tool, ToolEvent, ToolUpdateResult, Tools, ToolsManager};
use crate::ui::toolbars::ToolbarEvent;

type RenderedImage = Img<Vec<RGBA<u8>>>;

#[derive(Debug, Clone)]
pub enum SketchBoardInput {
    InputEvent(InputEvent),
    ToolbarEvent(ToolbarEvent),
    RenderResult(RenderedImage, Action),
    CommitEvent(TextEventMsg),
}

#[derive(Debug, Clone)]
pub enum SketchBoardOutput {
    ToggleToolbarsDisplay,
    ToolSwitchShortcut(Tools),
}

#[derive(Debug, Clone)]
pub enum InputEvent {
    Mouse(MouseEventMsg),
    Key(KeyEventMsg),
    KeyRelease(KeyEventMsg),
    Text(TextEventMsg),
}

// from https://flatuicolors.com/palette/au

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum MouseButton {
    Primary,
    Secondary,
    Middle,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyEventMsg {
    pub key: Key,
    pub code: u32,
    pub modifier: ModifierType,
}
#[derive(Debug, Clone)]
pub enum TextEventMsg {
    Commit(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseEventType {
    BeginDrag,
    EndDrag,
    UpdateDrag,
    Click,
    //Motion(Vec2D),
}

#[derive(Debug, Clone, Copy)]
pub struct MouseEventMsg {
    pub type_: MouseEventType,
    pub button: MouseButton,
    pub modifier: ModifierType,
    pub pos: Vec2D,
}

impl SketchBoardInput {
    pub fn new_mouse_event(
        event_type: MouseEventType,
        button: u32,
        modifier: ModifierType,
        pos: Vec2D,
    ) -> SketchBoardInput {
        SketchBoardInput::InputEvent(InputEvent::Mouse(MouseEventMsg {
            type_: event_type,
            button: button.into(),
            modifier,
            pos,
        }))
    }
    pub fn new_key_event(event: KeyEventMsg) -> SketchBoardInput {
        SketchBoardInput::InputEvent(InputEvent::Key(event))
    }

    pub fn new_key_release_event(event: KeyEventMsg) -> SketchBoardInput {
        SketchBoardInput::InputEvent(InputEvent::KeyRelease(event))
    }

    pub fn new_text_event(event: TextEventMsg) -> SketchBoardInput {
        SketchBoardInput::InputEvent(InputEvent::Text(event))
    }

    pub fn new_commit_event(event: TextEventMsg) -> SketchBoardInput {
        SketchBoardInput::CommitEvent(event)
    }
}

impl From<u32> for MouseButton {
    fn from(value: u32) -> Self {
        match value {
            gtk::gdk::BUTTON_PRIMARY => MouseButton::Primary,
            gtk::gdk::BUTTON_MIDDLE => MouseButton::Middle,
            gtk::gdk::BUTTON_SECONDARY => MouseButton::Secondary,
            _ => MouseButton::Primary,
        }
    }
}

impl InputEvent {
    fn remap_event_coordinates(&mut self, renderer: &FemtoVGArea) {
        if let InputEvent::Mouse(me) = self {
            match me.type_ {
                MouseEventType::BeginDrag | MouseEventType::Click => {
                    me.pos = renderer.abs_canvas_to_image_coordinates(me.pos)
                }
                MouseEventType::EndDrag | MouseEventType::UpdateDrag => {
                    me.pos = renderer.rel_canvas_to_image_coordinates(me.pos)
                }
            }
        };
    }
}

pub struct SketchBoard {
    renderer: FemtoVGArea,
    active_tool: Rc<RefCell<dyn Tool>>,
    tools: ToolsManager,
    style: Style,
}

impl SketchBoard {
    fn refresh_screen(&mut self) {
        self.renderer.queue_render();
    }

    fn image_to_pixbuf(image: RenderedImage) -> Pixbuf {
        let (buf, w, h) = image.into_contiguous_buf();

        Pixbuf::from_bytes(
            &Bytes::from(buf.as_bytes()),
            gdk_pixbuf::Colorspace::Rgb,
            true,
            8,
            w as i32,
            h as i32,
            w as i32 * 4,
        )
    }

    fn handle_render_result(&self, image: RenderedImage, action: Action) {
        match action {
            Action::SaveToClipboard => self.handle_copy_clipboard(Self::image_to_pixbuf(image)),
            Action::SaveToFile => self.handle_save(Self::image_to_pixbuf(image)),
        };
        if APP_CONFIG.read().early_exit() {
            relm4::main_application().quit();
        }
    }

    fn handle_save(&self, image: Pixbuf) {
        let output_filename = match APP_CONFIG.read().output_filename() {
            None => {
                println!("No Output filename specified!");
                return;
            }
            Some(o) => o.clone(),
        };

        // run the output filename by "chrono date format"
        let output_filename = format!("{}", chrono::Local::now().format(&output_filename));

        // TODO: we could support more data types
        if !output_filename.ends_with(".png") {
            log_result(
                "The only supported format is png, but the filename does not end in png",
                !APP_CONFIG.read().disable_notifications(),
            );
            return;
        }

        let data = match image.save_to_bufferv("png", &Vec::new()) {
            Ok(d) => d,
            Err(e) => {
                println!("Error serializing image: {e}");
                return;
            }
        };

        match fs::write(&output_filename, data) {
            Err(e) => log_result(
                &format!("Error while saving file: {e}"),
                !APP_CONFIG.read().disable_notifications(),
            ),
            Ok(_) => log_result(
                &format!("File saved to '{}'.", &output_filename),
                !APP_CONFIG.read().disable_notifications(),
            ),
        };
    }

    fn save_to_clipboard(&self, texture: &impl IsA<Texture>) -> anyhow::Result<()> {
        let display = DisplayManager::get()
            .default_display()
            .ok_or(anyhow!("Cannot open default display for clipboard."))?;
        display.clipboard().set_texture(texture);

        Ok(())
    }

    fn save_to_external_process(
        &self,
        texture: &impl IsA<Texture>,
        command: &str,
    ) -> anyhow::Result<()> {
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()?;

        let child_stdin = child.stdin.as_mut().unwrap();
        child_stdin.write_all(texture.save_to_png_bytes().as_ref())?;

        if !child.wait()?.success() {
            return Err(anyhow!("Writing to process '{command}' failed."));
        }

        Ok(())
    }

    fn handle_copy_clipboard(&self, image: Pixbuf) {
        let texture = Texture::for_pixbuf(&image);

        let result = if let Some(command) = APP_CONFIG.read().copy_command() {
            self.save_to_external_process(&texture, command)
        } else {
            self.save_to_clipboard(&texture)
        };

        match result {
            Err(e) => println!("Error saving {e}"),
            Ok(()) => {
                log_result(
                    "Copied to clipboard.",
                    !APP_CONFIG.read().disable_notifications(),
                );

                // TODO: rethink order and messaging patterns
                if APP_CONFIG.read().save_after_copy() {
                    self.handle_save(image);
                };
            }
        }
    }

    fn handle_undo(&mut self) -> ToolUpdateResult {
        if self.active_tool.borrow().active() {
            self.active_tool.borrow_mut().handle_undo()
        } else if self.renderer.undo() {
            ToolUpdateResult::Redraw
        } else {
            ToolUpdateResult::Unmodified
        }
    }

    fn handle_redo(&mut self) -> ToolUpdateResult {
        if self.active_tool.borrow().active() {
            self.active_tool.borrow_mut().handle_redo()
        } else if self.renderer.redo() {
            ToolUpdateResult::Redraw
        } else {
            ToolUpdateResult::Unmodified
        }
    }

    // Toolbars = Tools Toolbar + Style Toolbar
    fn handle_toggle_toolbars_display(
        &mut self,
        sender: ComponentSender<Self>,
    ) -> ToolUpdateResult {
        sender
            .output_sender()
            .emit(SketchBoardOutput::ToggleToolbarsDisplay);
        ToolUpdateResult::Unmodified
    }

    fn handle_toolbar_event(&mut self, toolbar_event: ToolbarEvent) -> ToolUpdateResult {
        match toolbar_event {
            ToolbarEvent::ToolSelected(tool) => {
                // deactivate old tool and save drawable, if any
                let mut deactivate_result = self
                    .active_tool
                    .borrow_mut()
                    .handle_event(ToolEvent::Deactivated);

                if let ToolUpdateResult::Commit(d) = deactivate_result {
                    self.renderer.commit(d);
                    // we handle commit directly and "downgrade" to a simple redraw result
                    deactivate_result = ToolUpdateResult::Redraw;
                }

                // change active tool
                self.active_tool = self.tools.get(&tool);
                self.renderer.set_active_tool(self.active_tool.clone());

                // send style event
                self.active_tool
                    .borrow_mut()
                    .handle_event(ToolEvent::StyleChanged(self.style));

                // send activated event
                let activate_result = self
                    .active_tool
                    .borrow_mut()
                    .handle_event(ToolEvent::Activated);

                match activate_result {
                    ToolUpdateResult::Unmodified => deactivate_result,
                    _ => activate_result,
                }
            }
            ToolbarEvent::ColorSelected(color) => {
                self.style.color = color;
                self.active_tool
                    .borrow_mut()
                    .handle_event(ToolEvent::StyleChanged(self.style))
            }
            ToolbarEvent::SizeSelected(size) => {
                self.style.size = size;
                self.active_tool
                    .borrow_mut()
                    .handle_event(ToolEvent::StyleChanged(self.style))
            }
            ToolbarEvent::SaveFile => {
                self.renderer.request_render(Action::SaveToFile);
                ToolUpdateResult::Unmodified
            }
            ToolbarEvent::CopyClipboard => {
                self.renderer.request_render(Action::SaveToClipboard);
                ToolUpdateResult::Unmodified
            }
            ToolbarEvent::Undo => self.handle_undo(),
            ToolbarEvent::Redo => self.handle_redo(),
            ToolbarEvent::ToggleFill => {
                self.style.fill = !self.style.fill;
                self.active_tool
                    .borrow_mut()
                    .handle_event(ToolEvent::StyleChanged(self.style))
            }
        }
    }

    fn handle_text_commit(
        &self,
        event: TextEventMsg,
        sender: ComponentSender<Self>,
    ) -> ToolUpdateResult {
        let tool_shortcuts = HashMap::from([
            (("p", ""), Tools::Pointer),
            (("c", "1"), Tools::Crop),
            (("b", "2"), Tools::Brush),
            (("l", "3"), Tools::Line),
            (("a", "4"), Tools::Arrow),
            (("r", "5"), Tools::Rectangle),
            (("e", "6"), Tools::Ellipse),
            (("t", "7"), Tools::Text),
            (("m", "8"), Tools::Marker),
            (("u", "9"), Tools::Blur),
            (("h", "0"), Tools::Highlight),
        ]);
        match event {
            TextEventMsg::Commit(txt) => {
                // NOTE:
                // If there's an IMContext binded to the controller, single letter-key events will
                // always go through it first, denying a bypass, so the only way we can do single-key
                // bindings is to act upon the IMMulticontext's commit event itself.
                // NOTE:
                // Here we're basically bypassing the IMMulticontext. If the text tool is active
                // and wants text inputs, we're interested in the single-letter keypress as a text character.
                // If not, we parse it as a shortcut event.
                if self.active_tool_type() == Tools::Text
                    && self.active_tool.borrow().input_enabled()
                {
                    sender.input(SketchBoardInput::new_text_event(TextEventMsg::Commit(
                        txt.to_string(),
                    )));
                    ToolUpdateResult::Unmodified
                } else {
                    let key = txt.as_str();
                    if let Some(tool) = tool_shortcuts
                        .iter()
                        .find(|item| item.0.0 == key || item.0.1 == key)
                        .map(|(_, tool)| tool)
                    {
                        sender.input(SketchBoardInput::ToolbarEvent(ToolbarEvent::ToolSelected(
                            *tool,
                        )));
                        sender
                            .output_sender()
                            .emit(SketchBoardOutput::ToolSwitchShortcut(*tool));

                        ToolUpdateResult::Unmodified
                    } else {
                        ToolUpdateResult::Unmodified
                    }
                }
            }
        }
    }

    pub fn active_tool_type(&self) -> Tools {
        self.active_tool.borrow().get_tool_type()
    }
}

#[relm4::component(pub)]
impl Component for SketchBoard {
    type CommandOutput = ();
    type Input = SketchBoardInput;
    type Output = SketchBoardOutput;
    type Init = Pixbuf;

    view! {
        gtk::Box {
            #[local_ref]
            area -> FemtoVGArea {
                set_vexpand: true,
                set_hexpand: true,
                set_can_focus: true,
                set_focusable: true,
                grab_focus: (),

                add_controller = gtk::GestureDrag {
                        set_button: 0,
                        connect_drag_begin[sender] => move |controller, x, y| {
                            sender.input(SketchBoardInput::new_mouse_event(
                                MouseEventType::BeginDrag,
                                controller.current_button(),
                                controller.current_event_state(),
                                Vec2D::new(x as f32, y as f32)));

                        },
                        connect_drag_update[sender] => move |controller, x, y| {
                            sender.input(SketchBoardInput::new_mouse_event(
                                MouseEventType::UpdateDrag,
                                controller.current_button(),
                                controller.current_event_state(),
                                Vec2D::new(x as f32, y as f32)));
                        },
                        connect_drag_end[sender] => move |controller, x, y| {
                            sender.input(SketchBoardInput::new_mouse_event(
                                MouseEventType::EndDrag,
                                controller.current_button(),
                                controller.current_event_state(),
                                Vec2D::new(x as f32, y as f32)
                            ));
                        }
                },
                add_controller = gtk::GestureClick {
                    set_button: 0,
                    connect_pressed[sender] => move |controller, _, x, y| {
                        sender.input(SketchBoardInput::new_mouse_event(
                            MouseEventType::Click,
                            controller.current_button(),
                            controller.current_event_state(),
                            Vec2D::new(x as f32, y as f32)));
                    }
                },

                add_controller = gtk::EventControllerKey {
                    connect_key_pressed[sender] => move |controller, key, code, modifier | {
                        if let Some(im_context) = controller.im_context() {
                            im_context.focus_in();
                            if !im_context.filter_keypress(controller.current_event().unwrap()) {
                                sender.input(SketchBoardInput::new_key_event(KeyEventMsg::new(key, code, modifier)));
                            }
                        } else {
                            sender.input(SketchBoardInput::new_key_event(KeyEventMsg::new(key, code, modifier)));
                        }
                        glib::Propagation::Stop
                    },

                    connect_key_released[sender] => move |controller, key, code, modifier | {
                        if let Some(im_context) = controller.im_context() {
                            im_context.focus_in();
                            if !im_context.filter_keypress(controller.current_event().unwrap()) {
                                sender.input(SketchBoardInput::new_key_release_event(KeyEventMsg::new(key, code, modifier)));
                            }
                        } else {
                            sender.input(SketchBoardInput::new_key_release_event(KeyEventMsg::new(key, code, modifier)));
                        }
                    },

                    #[wrap(Some)]
                    set_im_context = &gtk::IMMulticontext {
                        connect_commit[sender] => move |_cx, txt| {
                            sender.input(SketchBoardInput::new_commit_event(TextEventMsg::Commit(txt.to_string())));
                        },
                    },
                }
            }
        },
    }

    fn update(&mut self, msg: SketchBoardInput, sender: ComponentSender<Self>, _root: &Self::Root) {
        // handle resize ourselves, pass everything else to tool
        let result = match msg {
            SketchBoardInput::InputEvent(mut ie) => {
                if let InputEvent::Key(ke) = ie {
                    match (true, ke.modifier) {
                        (z, ModifierType::CONTROL_MASK)
                            if z == ke.is_one_of(Key::z, KeyMappingId::UsZ) =>
                        {
                            self.handle_undo()
                        }
                        (y, ModifierType::CONTROL_MASK)
                            if y == ke.is_one_of(Key::y, KeyMappingId::UsY) =>
                        {
                            self.handle_redo()
                        }
                        (t, ModifierType::CONTROL_MASK)
                            if t == ke.is_one_of(Key::t, KeyMappingId::UsT) =>
                        {
                            self.handle_toggle_toolbars_display(sender)
                        }
                        (s, ModifierType::CONTROL_MASK)
                            if s == ke.is_one_of(Key::s, KeyMappingId::UsS) =>
                        {
                            self.renderer.request_render(Action::SaveToFile);
                            ToolUpdateResult::Unmodified
                        }
                        (c, ModifierType::CONTROL_MASK)
                            if c == ke.is_one_of(Key::c, KeyMappingId::UsC) =>
                        {
                            self.renderer.request_render(Action::SaveToClipboard);
                            ToolUpdateResult::Unmodified
                        }
                        (esc, _) if esc == ke.is_one_of(Key::Escape, KeyMappingId::Escape) => {
                            relm4::main_application().quit();
                            // this is only here to make rust happy. The application should exit with the previous call
                            ToolUpdateResult::Unmodified
                        }
                        (enter, _)
                            if enter
                                == ke.is_one_of(Key::Return, KeyMappingId::Enter)
                                    | ke.is_one_of(Key::KP_Enter, KeyMappingId::Enter) =>
                        {
                            // First, let the tool handle the event. If the tool does nothing, we can do our thing (otherwise require a second Enter)
                            // Relying on ToolUpdateResult::Unmodified is probably not a good idea, but it's the only way at the moment. See discussion in #144
                            let result: ToolUpdateResult = self
                                .active_tool
                                .borrow_mut()
                                .handle_event(ToolEvent::Input(ie));
                            if let ToolUpdateResult::Unmodified = result {
                                self.renderer
                                    .request_render(APP_CONFIG.read().action_on_enter());
                            }
                            result
                        }
                        _ => {
                            self.active_tool
                            .borrow_mut()
                            .handle_event(ToolEvent::Input(ie))
                        },
                    }
                } else {
                    ie.remap_event_coordinates(&self.renderer);
                    self.active_tool
                        .borrow_mut()
                        .handle_event(ToolEvent::Input(ie))
                }
            }
            SketchBoardInput::ToolbarEvent(toolbar_event) => {
                self.handle_toolbar_event(toolbar_event)
            }
            SketchBoardInput::RenderResult(img, action) => {
                self.handle_render_result(img, action);
                ToolUpdateResult::Unmodified
            }
            SketchBoardInput::CommitEvent(txt) => {
                self.handle_text_commit(txt, sender);
                ToolUpdateResult::Unmodified
            }
        };

        //println!("Event={:?} Result={:?}", msg, result);
        match result {
            ToolUpdateResult::Commit(drawable) => {
                self.renderer.commit(drawable);
                self.refresh_screen();
            }
            ToolUpdateResult::Unmodified => (),
            ToolUpdateResult::Redraw => self.refresh_screen(),
        };
    }

    fn init(
        image: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let config = APP_CONFIG.read();
        let tools = ToolsManager::new();

        let mut model = Self {
            renderer: FemtoVGArea::default(),
            active_tool: tools.get(&config.initial_tool()),
            style: Style::default(),
            tools,
        };

        let area = &mut model.renderer;
        area.init(
            sender.input_sender().clone(),
            model.tools.get_crop_tool(),
            model.active_tool.clone(),
            image,
        );

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}

impl KeyEventMsg {
    pub fn new(key: Key, code: u32, modifier: ModifierType) -> Self {
        Self {
            key,
            code,
            modifier,
        }
    }

    /// Matches one of providen keys. The modifier is not considered.
    /// And the key has more priority over keycode.
    fn is_one_of(&self, key: Key, code: KeyMappingId) -> bool {
        // INFO: on linux the keycode from gtk4 is evdev keycode, so need to match by him if need
        // to use layout-independent shortcuts. And notice that there is substraction by 8, it's
        // because of x11 compatibility in which the keycodes are in range [8,255]. So need shift
        // them to get correct evdev keycode.
        let keymap = KeyMap::from(code);
        self.key == key || self.code as u16 - 8 == keymap.evdev
    }
}
