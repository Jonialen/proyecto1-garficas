use crossterm::style::Color;

#[derive(Clone, Copy, PartialEq)]
pub struct Pixel {
    pub color: Color,
    pub symbol: char,
}

impl Pixel {
    pub fn new(color: Color, symbol: char) -> Self {
        Self { color, symbol }
    }
}
