use std::borrow::Cow;

use gdk_pixbuf::{
    glib::{FromVariant, Variant, VariantTy},
    prelude::{StaticVariantType, ToVariant},
};
use pangocairo::pango::SCALE;

#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub color: Color,
    pub size: Size,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Color {
    Orange = 0,
    Red = 1,
    Green = 2,
    Blue = 3,
    Cove = 4,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Size {
    Small = 0,
    Medium = 1,
    Large = 2,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            color: Color::Orange,
            size: Size::Medium,
        }
    }
}

impl StaticVariantType for Color {
    fn static_variant_type() -> Cow<'static, VariantTy> {
        Cow::Borrowed(VariantTy::UINT32)
    }
}

impl ToVariant for Color {
    fn to_variant(&self) -> Variant {
        Variant::from(*self as u32)
    }
}

impl FromVariant for Color {
    fn from_variant(variant: &Variant) -> Option<Self> {
        variant.get::<u32>().and_then(|v| match v {
            0 => Some(Color::Orange),
            1 => Some(Color::Red),
            2 => Some(Color::Green),
            3 => Some(Color::Blue),
            4 => Some(Color::Cove),
            _ => None,
        })
    }
}

impl Color {
    pub fn to_rgb_f64(&self) -> (f64, f64, f64) {
        let (r, g, b) = self.to_rgb_u8();
        ((r as f64) / 255.0, (g as f64) / 255.0, (b as f64) / 255.0)
    }

    pub fn to_rgb_u8(&self) -> (u8, u8, u8) {
        match *self {
            Color::Orange => (240, 147, 43),
            Color::Red => (235, 77, 75),
            Color::Green => (106, 176, 76),
            Color::Blue => (34, 166, 179),
            Color::Cove => (19, 15, 64),
        }
    }

    pub fn to_rgba_u32(&self) -> u32 {
        let (r, g, b) = self.to_rgb_u8();
        ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (255u32)
    }
}

impl StaticVariantType for Size {
    fn static_variant_type() -> Cow<'static, VariantTy> {
        Cow::Borrowed(VariantTy::UINT32)
    }
}

impl ToVariant for Size {
    fn to_variant(&self) -> Variant {
        Variant::from(*self as u32)
    }
}

impl FromVariant for Size {
    fn from_variant(variant: &Variant) -> Option<Self> {
        variant.get::<u32>().and_then(|v| match v {
            0 => Some(Size::Small),
            1 => Some(Size::Medium),
            2 => Some(Size::Large),
            _ => None,
        })
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::Cove
    }
}

impl Size {
    pub fn to_text_size(&self) -> i32 {
        match *self {
            Size::Small => 12 * SCALE,
            Size::Medium => 18 * SCALE,
            Size::Large => 32 * SCALE,
        }
    }

    pub fn to_line_width(&self) -> f64 {
        match *self {
            Size::Small => 2.0,
            Size::Medium => 3.0,
            Size::Large => 5.0,
        }
    }

    pub fn to_blur_factor(&self) -> f64 {
        match *self {
            Size::Small => 6.0,
            Size::Medium => 10.0,
            Size::Large => 20.0,
        }
    }
}
