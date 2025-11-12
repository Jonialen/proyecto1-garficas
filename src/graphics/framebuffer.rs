use crate::graphics::Pixel;
use crossterm::style::Color;

/// Representa un framebuffer, un búfer de memoria que contiene los datos de una imagen a renderizar.
pub struct Framebuffer {
    /// El ancho del framebuffer en píxeles.
    pub width: usize,
    /// La altura del framebuffer en píxeles.
    pub height: usize,
    /// Una matriz 2D de píxeles que componen el framebuffer.
    pub pixels: Vec<Vec<Pixel>>,
}

impl Framebuffer {
    /// Crea un nuevo framebuffer con un ancho y alto específicos, inicializado con píxeles negros.
    pub fn new(width: usize, height: usize) -> Self {
        let pixels = vec![vec![Pixel::new(Color::Black, ' '); width]; height];
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Limpia el framebuffer, rellenándolo con un color específico.
    pub fn clear(&mut self, color: Color) {
        for row in &mut self.pixels {
            for pixel in row {
                pixel.color = color;
                pixel.symbol = ' ';
            }
        }
    }

    /// Establece el valor de un píxel en una coordenada específica del framebuffer.
    pub fn set_pixel(&mut self, x: usize, y: usize, pixel: Pixel) {
        if x < self.width && y < self.height {
            self.pixels[y][x] = pixel;
        }
    }

    /// Obtiene una referencia a un píxel en una coordenada específica.
    pub fn get_pixel(&self, x: usize, y: usize) -> Option<&Pixel> {
        if x < self.width && y < self.height {
            Some(&self.pixels[y][x])
        } else {
            None
        }
    }

    /// Dibuja una línea entre dos puntos utilizando el algoritmo de Bresenham.
    pub fn draw_line(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, pixel: Pixel) {
        let dx = (x1 as isize - x0 as isize).abs();
        let dy = (y1 as isize - y0 as isize).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;

        let mut x = x0 as isize;
        let mut y = y0 as isize;

        loop {
            self.set_pixel(x as usize, y as usize, pixel);

            if x == x1 as isize && y == y1 as isize {
                break;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }
}
