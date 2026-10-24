use crossterm::style::Color;

/// Representa un único píxel en la terminal.
/// Cada píxel tiene un color de primer plano, un color de fondo y un símbolo.
#[derive(Clone, Copy, PartialEq)]
pub struct Pixel {
    /// El color del símbolo (primer plano).
    pub color: Color,
    /// El color de fondo de la celda. `Color::Reset` usa el fondo de la terminal.
    pub bg: Color,
    /// El carácter que se mostrará en la terminal.
    pub symbol: char,
}

impl Pixel {
    /// Crea un nuevo píxel con un color y un símbolo específicos, sobre el fondo de la terminal.
    pub fn new(color: Color, symbol: char) -> Self {
        Self { color, bg: Color::Reset, symbol }
    }

    /// Crea un píxel con colores de primer plano y de fondo explícitos.
    pub fn with_bg(color: Color, bg: Color, symbol: char) -> Self {
        Self { color, bg, symbol }
    }
}
