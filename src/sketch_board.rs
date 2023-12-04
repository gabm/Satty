use std::cell::RefCell;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use std::rc::Rc;

use gdk_pixbuf::Pixbuf;
use gtk::prelude::*;

use relm4::drawing::DrawHandler;
use relm4::gtk::gdk::{DisplayManager, Key, ModifierType};
use relm4::{gtk, Component, ComponentParts, ComponentSender};

use crate::math::Vec2D;
use crate::renderer::Renderer;
use crate::style::Style;
use crate::tools::{Tool, ToolEvent, ToolUpdateResult, Tools, ToolsManager};
use crate::ui::toolbars::ToolbarEvent;

#[derive(Debug, Clone, Copy)]
pub enum SketchBoardInput {
    InputEvent(InputEvent),
    Resize(Vec2D),
    ToolbarEvent(ToolbarEvent),
}

#[derive(Debug, Clone)]
pub enum SketchBoardOutput {
    ShowToast(String),
}

#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    MouseEvent(MouseEventMsg),
    KeyEvent(KeyEventMsg),
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

#[derive(Debug, Clone, Copy)]
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
        SketchBoardInput::InputEvent(InputEvent::MouseEvent(MouseEventMsg {
            type_: event_type,
            button: button.into(),
            modifier,
            pos,
        }))
    }
    pub fn new_key_event(event: KeyEventMsg) -> SketchBoardInput {
        SketchBoardInput::InputEvent(InputEvent::KeyEvent(event))
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
    fn screen2image(p: &mut Vec2D, scale: f64) {
        p.x /= scale;
        p.y /= scale;
    }

    fn remap_event_coordinates(&mut self, scale: f64) {
        if let InputEvent::MouseEvent(me) = self {
            Self::screen2image(&mut me.pos, scale)
        };
    }
}

pub struct SketchBoardConfig {
    pub original_image: Pixbuf,
    pub output_filename: Option<String>,
    pub copy_command: Option<String>,
    pub early_exit: bool,
    pub init_tool: Tools,
}

pub struct SketchBoard {
    handler: DrawHandler,
    active_tool: Rc<RefCell<dyn Tool>>,
    tools: ToolsManager,
    style: Style,
    config: SketchBoardConfig,
    renderer: Renderer,
    scale_factor: f64,
}

impl SketchBoard {
    pub fn calculate_scale_factor(&mut self, new_dimensions: Vec2D) {
        let aspect_ratio =
            self.config.original_image.width() as f64 / self.config.original_image.height() as f64;
        self.scale_factor = if new_dimensions.x / aspect_ratio <= new_dimensions.y {
            new_dimensions.x / aspect_ratio / self.config.original_image.height() as f64
        } else {
            new_dimensions.y * aspect_ratio / self.config.original_image.width() as f64
        };
    }
    fn refresh_screen(&mut self) {
        let cx = self.handler.get_context();
        if let Err(e) = self
            .renderer
            .render_to_window(&cx, self.scale_factor, &self.active_tool)
        {
            println!("Error drawing: {:?}", e);
        }
    }

    fn handle_save(&self, sender: ComponentSender<Self>) {
        let output_filename = match &self.config.output_filename {
            None => {
                println!("No Output filename specified!");
                return;
            }
            Some(o) => o,
        };

        if !output_filename.ends_with(".png") {
            let msg = "The only supported format is png, but the filename does not end in png";
            println!("{msg}");
            sender
                .output_sender()
                .emit(SketchBoardOutput::ShowToast(msg.to_string()));
            return;
        }

        let texture = match self.renderer.render_to_texture(&self.active_tool) {
            Ok(t) => t,
            Err(e) => {
                println!("Error while creating texture: {e}");
                return;
            }
        };

        let data = texture.save_to_png_bytes();

        let msg = match fs::write(output_filename, data) {
            Err(e) => format!("Error while saving file: {e}"),
            Ok(_) => format!("File saved to '{}'.", output_filename),
        };

        sender
            .output_sender()
            .emit(SketchBoardOutput::ShowToast(msg));
    }

    fn handle_copy_clipboard(&self, sender: ComponentSender<Self>) {
        let texture = match self.renderer.render_to_texture(&self.active_tool) {
            Ok(t) => t,
            Err(e) => {
                println!("Error while creating texture: {e}");
                return;
            }
        };

        if let Some(command) = &self.config.copy_command {
            let mut child = Command::new(command)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .spawn()
                .unwrap();

            let child_stdin = child.stdin.as_mut().unwrap();
            child_stdin
                .write_all(texture.save_to_png_bytes().as_ref())
                .unwrap();
            if child.wait().unwrap().success() {
                sender.output_sender().emit(SketchBoardOutput::ShowToast(
                    "Copied to clipboard.".to_string(),
                ));
            }
        } else {
            match DisplayManager::get().default_display() {
                Some(display) => {
                    display.clipboard().set_texture(&texture);
                    sender.output_sender().emit(SketchBoardOutput::ShowToast(
                        "Copied to clipboard.".to_string(),
                    ));
                }
                None => {
                    println!("Cannot save to clipboard");
                }
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

    fn handle_toolbar_event(
        &mut self,
        toolbar_event: ToolbarEvent,
        sender: ComponentSender<Self>,
    ) -> ToolUpdateResult {
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
                self.handle_save(sender);
                if self.config.early_exit {
                    relm4::main_application().quit();
                }
                ToolUpdateResult::Unmodified
            }
            ToolbarEvent::CopyClipboard => {
                self.handle_copy_clipboard(sender);
                if self.config.early_exit {
                    relm4::main_application().quit();
                }
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
    type Init = SketchBoardConfig;

    view! {
        gtk::Box {
            #[local_ref]
            area -> gtk::DrawingArea {
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
                                Vec2D::new(x, y)));

                        },
                        connect_drag_update[sender] => move |controller, x, y| {
                            sender.input(SketchBoardInput::new_mouse_event(
                                MouseEventType::UpdateDrag,
                                controller.current_button(),
                                controller.current_event_state(),
                                Vec2D::new(x, y)));
                        },
                        connect_drag_end[sender] => move |controller, x, y| {
                            sender.input(SketchBoardInput::new_mouse_event(
                                MouseEventType::EndDrag,
                                controller.current_button(),
                                controller.current_event_state(),
                                Vec2D::new(x, y)
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
                            Vec2D::new(x, y)));
                    }
                },

                connect_resize[sender] => move |_, x, y| {
                    sender.input(SketchBoardInput::Resize(Vec2D::new(x as f64,y as f64)));
                }
            }
        },
    }

    fn update(&mut self, msg: SketchBoardInput, sender: ComponentSender<Self>, _root: &Self::Root) {
        // handle resize ourselves, pass everything else to tool
        let result = match msg {
            SketchBoardInput::Resize(dim) => {
                self.calculate_scale_factor(dim);
                ToolUpdateResult::Redraw
            }

            SketchBoardInput::InputEvent(mut ie) => {
                if let InputEvent::KeyEvent(ke) = ie {
                    if ke.key == Key::z && ke.modifier == ModifierType::CONTROL_MASK {
                        self.handle_undo()
                    } else if ke.key == Key::y && ke.modifier == ModifierType::CONTROL_MASK {
                        self.handle_redo()
                    } else if ke.key == Key::s && ke.modifier == ModifierType::CONTROL_MASK {
                        self.handle_save(sender);
                        if self.config.early_exit {
                            relm4::main_application().quit();
                        }
                        ToolUpdateResult::Unmodified
                    } else if ke.key == Key::c && ke.modifier == ModifierType::CONTROL_MASK {
                        self.handle_copy_clipboard(sender);
                        if self.config.early_exit {
                            relm4::main_application().quit();
                        }
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
                    ie.remap_event_coordinates(self.scale_factor);
                    self.active_tool
                        .borrow_mut()
                        .handle_event(ToolEvent::Input(ie))
                }
            }
            SketchBoardInput::ToolbarEvent(toolbar_event) => {
                self.handle_toolbar_event(toolbar_event, sender)
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
        config: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let tools = ToolsManager::new();

        let model = Self {
            handler: DrawHandler::new(),
            active_tool: tools.get(&config.init_tool),
            style: Style::default(),
            renderer: Renderer::new(config.original_image.clone(), tools.get_crop_tool()),
            scale_factor: 1.0,
            config,
            tools,
        };

        let area = model.handler.drawing_area();
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
