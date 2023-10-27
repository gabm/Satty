use std::{borrow::Cow, cell::RefCell, collections::HashMap, fmt::Debug, rc::Rc};

use anyhow::Result;
use gdk_pixbuf::{
    glib::{FromVariant, Variant, VariantTy},
    prelude::{StaticVariantType, ToVariant},
};
use pangocairo::cairo::ImageSurface;
use relm4::gtk::cairo::Context;

use crate::{
    sketch_board::{InputEvent, KeyEventMsg, MouseEventMsg},
    style::Style,
};

mod arrow;
mod blur;
mod crop;
mod line;
mod marker;
mod rectangle;
mod text;

pub enum ToolEvent {
    Activated,
    Deactivated,
    Input(InputEvent),
    StyleChanged(Style),
}

pub trait Tool {
    fn handle_event(&mut self, event: ToolEvent) -> ToolUpdateResult {
        match event {
            ToolEvent::Activated => self.handle_activated(),
            ToolEvent::Deactivated => self.handle_deactivated(),
            ToolEvent::Input(e) => self.handle_input_event(e),
            ToolEvent::StyleChanged(s) => self.handle_style_event(s),
        }
    }

    fn handle_activated(&mut self) -> ToolUpdateResult {
        ToolUpdateResult::Unmodified
    }

    fn handle_deactivated(&mut self) -> ToolUpdateResult {
        ToolUpdateResult::Unmodified
    }

    fn handle_input_event(&mut self, event: InputEvent) -> ToolUpdateResult {
        match event {
            InputEvent::MouseEvent(e) => self.handle_mouse_event(e),
            InputEvent::KeyEvent(e) => self.handle_key_event(e),
        }
    }

    fn handle_mouse_event(&mut self, event: MouseEventMsg) -> ToolUpdateResult {
        let _ = event;
        ToolUpdateResult::Unmodified
    }

    fn handle_key_event(&mut self, event: KeyEventMsg) -> ToolUpdateResult {
        let _ = event;
        ToolUpdateResult::Unmodified
    }

    fn handle_style_event(&mut self, style: Style) -> ToolUpdateResult {
        let _ = style;
        ToolUpdateResult::Unmodified
    }

    fn get_drawable(&self) -> Option<&dyn Drawable>;
}

// the clone method below has been adapted from: https://stackoverflow.com/questions/30353462/how-to-clone-a-struct-storing-a-boxed-trait-object
// it feels "strange" and especially the fact that drawable has to derive from DrawableClone feels "wrong".
pub trait DrawableClone {
    fn clone_box(&self) -> Box<dyn Drawable>;
}

impl<T> DrawableClone for T
where
    T: 'static + Drawable + Clone,
{
    fn clone_box(&self) -> Box<dyn Drawable> {
        Box::new(self.clone())
    }
}

pub trait Drawable: DrawableClone + Debug {
    fn draw(&self, cx: &Context, surface: &ImageSurface) -> Result<()>;
}

#[derive(Debug)]
pub enum ToolUpdateResult {
    Commit(Box<dyn Drawable>),
    Redraw,
    Unmodified,
}

pub use arrow::ArrowTool;
pub use blur::BlurTool;
pub use crop::CropTool;
pub use line::LineTool;
pub use marker::MARKER_CURRENT_NUMBER;
pub use rectangle::RectangleTool;
pub use text::TextTool;

use self::marker::MarkerTool;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum Tools {
    Crop = 0,
    Line = 1,
    Arrow = 2,
    Rectangle = 3,
    Text = 4,
    Marker = 5,
    Blur = 6,
}

pub struct ToolsManager {
    tools: HashMap<Tools, Rc<RefCell<dyn Tool>>>,
    crop_tool: Rc<RefCell<CropTool>>,
}

impl ToolsManager {
    pub fn new() -> Self {
        let mut tools: HashMap<Tools, Rc<RefCell<dyn Tool>>> = HashMap::new();
        //tools.insert(Tools::Crop, Rc::new(RefCell::new(CropTool::default())));
        tools.insert(Tools::Line, Rc::new(RefCell::new(LineTool::default())));
        tools.insert(Tools::Arrow, Rc::new(RefCell::new(ArrowTool::default())));
        tools.insert(
            Tools::Rectangle,
            Rc::new(RefCell::new(RectangleTool::default())),
        );
        tools.insert(Tools::Text, Rc::new(RefCell::new(TextTool::default())));
        tools.insert(Tools::Blur, Rc::new(RefCell::new(BlurTool::default())));
        tools.insert(Tools::Marker, Rc::new(RefCell::new(MarkerTool::default())));

        let crop_tool = Rc::new(RefCell::new(CropTool::default()));
        Self { tools, crop_tool }
    }

    pub fn get(&self, tool: &Tools) -> Rc<RefCell<dyn Tool>> {
        match tool {
            Tools::Crop => self.crop_tool.clone(),
            _ => self.tools.get(tool).unwrap().clone(),
        }
    }

    pub fn get_crop_tool(&self) -> Rc<RefCell<CropTool>> {
        self.crop_tool.clone()
    }
}

impl StaticVariantType for Tools {
    fn static_variant_type() -> Cow<'static, VariantTy> {
        Cow::Borrowed(VariantTy::UINT32)
    }
}

impl ToVariant for Tools {
    fn to_variant(&self) -> Variant {
        Variant::from(*self as u32)
    }
}

impl FromVariant for Tools {
    fn from_variant(variant: &Variant) -> Option<Self> {
        variant.get::<u32>().and_then(|v| match v {
            0 => Some(Tools::Crop),
            1 => Some(Tools::Line),
            2 => Some(Tools::Arrow),
            3 => Some(Tools::Rectangle),
            4 => Some(Tools::Text),
            5 => Some(Tools::Marker),
            6 => Some(Tools::Blur),
            _ => None,
        })
    }
}
