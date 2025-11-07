use crossterm::style::Color;

/// Representa un único píxel en la terminal.
/// Cada píxel tiene un color y un símbolo que lo representa.
#[derive(Clone, Copy, PartialEq)]
pub struct Pixel {
    /// El color del píxel.
    pub color: Color,
    /// El carácter que se mostrará en la terminal.
    pub symbol: char,
}

impl Pixel {
    /// Crea un nuevo píxel con un color y un símbolo específicos.
    pub fn new(color: Color, symbol: char) -> Self {
        Self { color, symbol }
    }
}
