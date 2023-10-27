use std::cell::RefCell;
use std::rc::Rc;

use gdk_pixbuf::glib::Bytes;
use gdk_pixbuf::Pixbuf;
use gtk::prelude::*;

use anyhow::Result;

use pangocairo::cairo::{Filter, Format, ImageSurface, Pattern};
use relm4::drawing::DrawHandler;
use relm4::gtk::cairo::{Context, Operator};
use relm4::gtk::gdk::{DisplayManager, Key, MemoryFormat, MemoryTexture, ModifierType};
use relm4::{gtk, Component, ComponentParts, ComponentSender};

use crate::math::Vec2D;
use crate::style::{Color, Size, Style};
use crate::tools::{Drawable, Tool, ToolEvent, ToolUpdateResult, Tools, ToolsManager};

#[derive(Debug, Clone, Copy)]
pub enum SketchBoardMessage {
    InputEvent(InputEvent),
    ToolSelected(Tools),
    ColorSelected(Color),
    SizeSelected(Size),
    Resize(Vec2D),
    SaveFile,
    CopyClipboard,
    Undo,
    Redo,
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
pub enum MouseEventMsg {
    BeginDrag(Vec2D),
    EndDrag(Vec2D),
    UpdateDrag(Vec2D),
    Click(Vec2D, MouseButton),
    //Motion(Vec2D),
}

impl SketchBoardMessage {
    pub fn new_mouse_event(event: MouseEventMsg) -> SketchBoardMessage {
        SketchBoardMessage::InputEvent(InputEvent::MouseEvent(event))
    }
    pub fn new_key_event(event: KeyEventMsg) -> SketchBoardMessage {
        SketchBoardMessage::InputEvent(InputEvent::KeyEvent(event))
    }
}

impl InputEvent {
    fn screen2image(p: &mut Vec2D, scale: f64) {
        p.x /= scale;
        p.y /= scale;
    }

    fn remap_event_coordinates(&mut self, scale: f64) {
        match self {
            InputEvent::MouseEvent(me) => match me {
                MouseEventMsg::BeginDrag(p) => Self::screen2image(p, scale),
                MouseEventMsg::EndDrag(p) => Self::screen2image(p, scale),
                MouseEventMsg::UpdateDrag(p) => Self::screen2image(p, scale),
                MouseEventMsg::Click(p, _) => Self::screen2image(p, scale),
            },
            _ => (),
        };
    }
}

pub struct SketchBoardConfig {
    pub original_image: Pixbuf,
    pub output_filename: Option<String>,
}

pub struct SketchBoard {
    handler: DrawHandler,
    board_dimensions: Vec2D,
    scale_factor: f64,
    active_tool: Rc<RefCell<dyn Tool>>,
    tools: ToolsManager,
    drawables: Vec<Box<dyn Drawable>>,
    redo_stack: Vec<Box<dyn Drawable>>,
    style: Style,
    config: SketchBoardConfig,
}

impl SketchBoard {
    fn resize(&mut self, new_dimensions: Vec2D) {
        let aspect_ratio =
            self.config.original_image.width() as f64 / self.config.original_image.height() as f64;
        self.scale_factor = if new_dimensions.x / aspect_ratio <= new_dimensions.y {
            new_dimensions.x / aspect_ratio / self.config.original_image.height() as f64
        } else {
            new_dimensions.y * aspect_ratio / self.config.original_image.width() as f64
        };

        self.board_dimensions = new_dimensions;
    }

    fn render_to_window(&self, cx: &Context) -> Result<()> {
        let surface = self.render_full_size(true)?;

        // render to window
        cx.scale(self.scale_factor, self.scale_factor);
        cx.set_source_surface(surface, 0.0, 0.0)?;
        Pattern::set_filter(&cx.source(), Filter::Fast);
        cx.paint()?;

        Ok(())
    }

    fn render_full_size(&self, render_crop: bool) -> Result<ImageSurface> {
        let surface = ImageSurface::create(
            Format::ARgb32,
            self.config.original_image.width(),
            self.config.original_image.height(),
        )?;

        let cx: Context = Context::new(surface.clone())?;
        cx.set_operator(Operator::Over);

        // render background image
        cx.set_source_pixbuf(&self.config.original_image, 0.0, 0.0);
        cx.paint()?;

        // render comitted drawables
        for da in &self.drawables {
            da.draw(&cx, &surface)?;
        }

        // render drawable of active tool, if any
        if let Some(d) = &self.active_tool.borrow().get_drawable() {
            d.draw(&cx, &surface)?;
        }

        if render_crop {
            // render crop (even if tool not active)
            if let Some(c) = self.tools.get_crop_tool().borrow().get_crop() {
                c.draw(&cx, &surface)?;
            }
        }

        Ok(surface)
    }

    fn render_with_crop(&self) -> Result<ImageSurface> {
        // render final image
        let mut surface = self.render_full_size(false)?;

        if let Some((pos, size)) = self
            .tools
            .get_crop_tool()
            .borrow()
            .get_crop()
            .and_then(|c| c.get_rectangle())
        {
            // crop the full size render to target values
            let cropped_surface =
                ImageSurface::create(Format::ARgb32, size.x as i32, size.y as i32)?;
            let cropped_cx = Context::new(cropped_surface.clone())?;
            cropped_cx.set_source_surface(surface, -pos.x, -pos.y)?;
            cropped_cx.paint()?;

            surface = cropped_surface.clone();
        }

        Ok(surface)
    }

    fn render_to_texture(&self) -> Result<MemoryTexture> {
        let mut surface = self.render_with_crop()?;

        let height = surface.height();
        let width = surface.width();
        let stride = surface.stride() as usize;
        let data = surface.data()?;

        let texture = MemoryTexture::new(
            width,
            height,
            MemoryFormat::B8g8r8a8Premultiplied,
            &Bytes::from(&*data),
            stride,
        );

        Ok(texture)
    }

    fn redraw_screen(&mut self) {
        let cx = self.handler.get_context();
        if let Err(e) = self.render_to_window(&cx) {
            println!("Error drawing: {:?}", e);
        }
    }

    fn handle_save(&self) {
        if self.config.output_filename.is_none() {
            println!("No Output filename specified!");
            return;
        }

        let texture = match self.render_to_texture() {
            Ok(t) => t,
            Err(e) => {
                println!("Error while creating texture: {e}");
                return;
            }
        };

        if let Err(e) = texture.save_to_png(&self.config.output_filename.as_ref().unwrap()) {
            println!("Error while saving texture: {e}");
            return;
        }
    }

    fn handle_copy_clipboard(&self) {
        let texture = match self.render_to_texture() {
            Ok(t) => t,
            Err(e) => {
                println!("Error while creating texture: {e}");
                return;
            }
        };
        match DisplayManager::get().default_display() {
            Some(display) => display.clipboard().set_texture(&texture),
            None => {
                println!("Cannot save to clipboard");
                return;
            }
        }
    }

    fn handle_undo(&mut self) -> ToolUpdateResult {
        match self.drawables.pop() {
            Some(d) => {
                self.redo_stack.push(d);
                ToolUpdateResult::Redraw
            }
            None => ToolUpdateResult::Unmodified,
        }
    }

    fn handle_redo(&mut self) -> ToolUpdateResult {
        match self.redo_stack.pop() {
            Some(d) => {
                self.drawables.push(d);
                ToolUpdateResult::Redraw
            }
            None => ToolUpdateResult::Unmodified,
        }
    }
}

#[relm4::component(pub)]
impl Component for SketchBoard {
    type CommandOutput = ();
    type Input = SketchBoardMessage;
    type Output = ();
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
                            if controller.current_button() == gtk::gdk::BUTTON_PRIMARY {
                                sender.input(SketchBoardMessage::new_mouse_event(MouseEventMsg::BeginDrag(Vec2D::new(x, y))));
                            }
                        },
                        connect_drag_update[sender] => move |controller, x, y| {
                            if controller.current_button() == gtk::gdk::BUTTON_PRIMARY {
                                sender.input(SketchBoardMessage::new_mouse_event(MouseEventMsg::UpdateDrag(Vec2D::new(x, y))));
                            }
                        },
                        connect_drag_end[sender] => move |controller, x, y| {
                            if controller.current_button() == gtk::gdk::BUTTON_PRIMARY {
                                sender.input(SketchBoardMessage::new_mouse_event(MouseEventMsg::EndDrag(Vec2D::new(x, y))));
                            }
                        }
                },
                add_controller = gtk::GestureClick {
                    set_button: 0,
                    connect_pressed[sender] => move |controller, _, x, y| {
                        let button = if controller.current_button() == gtk::gdk::BUTTON_PRIMARY {
                            MouseButton::Primary
                        } else if controller.current_button() == gtk::gdk::BUTTON_SECONDARY {
                            MouseButton::Secondary
                        } else {
                            MouseButton::Middle
                        };

                        sender.input(SketchBoardMessage::new_mouse_event(MouseEventMsg::Click(Vec2D::new(x, y), button)));
                    }
                },

                connect_resize[sender] => move |_, x, y| {
                    sender.input(SketchBoardMessage::Resize(Vec2D::new(x as f64,y as f64)));
                }
            }
        },
    }

    fn update(
        &mut self,
        msg: SketchBoardMessage,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        // handle resize ourselves, pass everything else to tool
        let result = match msg {
            SketchBoardMessage::Resize(dim) => {
                self.resize(dim);
                ToolUpdateResult::Redraw
            }
            SketchBoardMessage::ToolSelected(tool) => {
                // deactivate old tool and save drawable, if any
                let mut deactivate_result = self
                    .active_tool
                    .borrow_mut()
                    .handle_event(ToolEvent::Deactivated);

                if let ToolUpdateResult::Commit(d) = deactivate_result {
                    self.drawables.push(d);
                    self.redo_stack.clear();
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
            SketchBoardMessage::InputEvent(mut ie) => {
                if let InputEvent::KeyEvent(ke) = ie {
                    if ke.key == Key::z && ke.modifier == ModifierType::CONTROL_MASK {
                        self.handle_undo()
                    } else if ke.key == Key::y && ke.modifier == ModifierType::CONTROL_MASK {
                        self.handle_redo()
                    } else if ke.key == Key::s && ke.modifier == ModifierType::CONTROL_MASK {
                        self.handle_save();
                        ToolUpdateResult::Unmodified
                    } else if ke.key == Key::c && ke.modifier == ModifierType::CONTROL_MASK {
                        self.handle_copy_clipboard();
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
            SketchBoardMessage::ColorSelected(color) => {
                self.style.color = color;
                self.active_tool
                    .borrow_mut()
                    .handle_event(ToolEvent::StyleChanged(self.style))
            }
            SketchBoardMessage::SizeSelected(size) => {
                self.style.size = size;
                self.active_tool
                    .borrow_mut()
                    .handle_event(ToolEvent::StyleChanged(self.style))
            }
            SketchBoardMessage::SaveFile => {
                self.handle_save();
                ToolUpdateResult::Unmodified
            }
            SketchBoardMessage::CopyClipboard => {
                self.handle_copy_clipboard();
                ToolUpdateResult::Unmodified
            }
            SketchBoardMessage::Undo => self.handle_undo(),
            SketchBoardMessage::Redo => self.handle_redo(),
        };

        //println!("Event={:?} Result={:?}", msg, result);
        match result {
            ToolUpdateResult::Commit(drawable) => {
                self.drawables.push(drawable);
                self.redo_stack.clear();
                self.redraw_screen();
            }
            ToolUpdateResult::Unmodified => (),
            ToolUpdateResult::Redraw => self.redraw_screen(),
        };
    }

    fn init(
        config: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let board_dimensions = Vec2D::new(100.0, 100.0);

        let tools = ToolsManager::new();

        let model = Self {
            handler: DrawHandler::new(),
            board_dimensions,
            scale_factor: 1.0,
            active_tool: tools.get(&Tools::Crop),
            drawables: Vec::new(),
            redo_stack: Vec::new(),
            style: Style::default(),
            config,
            tools,
        };

        let area = model.handler.drawing_area();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}

impl Vec2D {
    pub fn zero() -> Self {
        Self { x: 0f64, y: 0f64 }
    }

    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
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
