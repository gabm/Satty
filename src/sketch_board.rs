use anyhow::anyhow;

use femtovg::imgref::Img;
use femtovg::rgb::{ComponentBytes, RGBA};
use gdk_pixbuf::glib::Bytes;
use gdk_pixbuf::Pixbuf;
use std::cell::RefCell;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::rc::Rc;

use gtk::prelude::*;

use relm4::gtk::gdk::{DisplayManager, Key, ModifierType, Texture};
use relm4::{gtk, Component, ComponentParts, ComponentSender};

use crate::configuration::APP_CONFIG;
use crate::femtovg_area::FemtoVGArea;
use crate::math::Vec2D;
use crate::style::Style;
use crate::tools::{Tool, ToolEvent, ToolUpdateResult, ToolsManager};
use crate::ui::toolbars::ToolbarEvent;

type RenderedImage = Img<Vec<RGBA<u8>>>;

#[derive(Debug, Clone)]
pub enum SketchBoardInput {
    InputEvent(InputEvent),
    ToolbarEvent(ToolbarEvent),
    RenderResult(RenderedImage, Action),
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    SaveToClipboard,
    SaveToFile,
}

#[derive(Debug, Clone)]
pub enum SketchBoardOutput {
    ShowToast(String),
    ToggleToolbarsDisplay,
}

#[derive(Debug, Clone)]
pub enum InputEvent {
    Mouse(MouseEventMsg),
    Key(KeyEventMsg),
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

    pub fn new_text_event(event: TextEventMsg) -> SketchBoardInput {
        SketchBoardInput::InputEvent(InputEvent::Text(event))
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

    fn handle_render_result(
        &self,
        sender: ComponentSender<Self>,
        image: RenderedImage,
        action: Action,
    ) {
        match action {
            Action::SaveToClipboard => {
                self.handle_copy_clipboard(sender, Self::image_to_pixbuf(image))
            }
            Action::SaveToFile => self.handle_save(sender, Self::image_to_pixbuf(image)),
        };
        if APP_CONFIG.read().early_exit() {
            relm4::main_application().quit();
        }
    }

    fn handle_save(&self, sender: ComponentSender<Self>, image: Pixbuf) {
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
            let msg = "The only supported format is png, but the filename does not end in png";
            println!("{msg}");
            sender
                .output_sender()
                .emit(SketchBoardOutput::ShowToast(msg.to_string()));
            return;
        }

        let data = match image.save_to_bufferv("png", &Vec::new()) {
            Ok(d) => d,
            Err(e) => {
                println!("Error serializing image: {e}");
                return;
            }
        };

        let msg = match fs::write(&output_filename, data) {
            Err(e) => format!("Error while saving file: {e}"),
            Ok(_) => format!("File saved to '{}'.", &output_filename),
        };

        sender
            .output_sender()
            .emit(SketchBoardOutput::ShowToast(msg));
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
        let mut child = Command::new(command)
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

    fn handle_copy_clipboard(&self, sender: ComponentSender<Self>, image: Pixbuf) {
        let texture = Texture::for_pixbuf(&image);

        let result = if let Some(command) = APP_CONFIG.read().copy_command() {
            self.save_to_external_process(&texture, command)
        } else {
            self.save_to_clipboard(&texture)
        };

        match result {
            Err(e) => println!("Error saving {e}"),
            Ok(()) => {
                sender.output_sender().emit(SketchBoardOutput::ShowToast(
                    "Copied to clipboard.".to_string(),
                ));

                // TODO: rethink order and messaging patterns
                if APP_CONFIG.read().save_after_copy() {
                    self.handle_save(sender, image);
                };
            }
        }
    }

    fn handle_undo(&mut self) -> ToolUpdateResult {
        if self.renderer.undo() {
            ToolUpdateResult::Redraw
        } else {
            ToolUpdateResult::Unmodified
        }
    }

    fn handle_redo(&mut self) -> ToolUpdateResult {
        if self.renderer.redo() {
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
        }
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
            }
        },
    }

    fn update(&mut self, msg: SketchBoardInput, sender: ComponentSender<Self>, _root: &Self::Root) {
        // handle resize ourselves, pass everything else to tool
        let result = match msg {
            SketchBoardInput::InputEvent(mut ie) => {
                if let InputEvent::Key(ke) = ie {
                    if ke.key == Key::z && ke.modifier == ModifierType::CONTROL_MASK {
                        self.handle_undo()
                    } else if ke.key == Key::y && ke.modifier == ModifierType::CONTROL_MASK {
                        self.handle_redo()
                    } else if ke.key == Key::t && ke.modifier == ModifierType::CONTROL_MASK {
                        self.handle_toggle_toolbars_display(sender)
                    } else if ke.key == Key::s && ke.modifier == ModifierType::CONTROL_MASK {
                        self.renderer.request_render(Action::SaveToFile);
                        ToolUpdateResult::Unmodified
                    } else if ke.key == Key::c && ke.modifier == ModifierType::CONTROL_MASK {
                        self.renderer.request_render(Action::SaveToClipboard);
                        ToolUpdateResult::Unmodified
                    } else if ke.key == Key::Escape {
                        relm4::main_application().quit();
                        // this is only here to make rust happy. The application should exit with the previous call
                        ToolUpdateResult::Unmodified
                    } else {
                        self.active_tool
                            .borrow_mut()
                            .handle_event(ToolEvent::Input(ie))
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
                self.handle_render_result(sender, img, action);
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
}
