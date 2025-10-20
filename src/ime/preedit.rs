use std::ops::Range;

use crate::style::Color;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UnderlineKind {
    #[default]
    None,
    Single,
    Double,
    Low,
    Wavy,
    Error,
}

#[derive(Clone, Debug, Default)]
pub struct PreeditSpan {
    pub range: Range<usize>,
    pub selected: bool,
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub underline: UnderlineKind,
    pub underline_color: Option<Color>,
}

#[derive(Clone, Debug, Default)]
pub struct Preedit {
    pub text: String,
    pub cursor_chars: Option<usize>,
    pub spans: Vec<PreeditSpan>,
}
