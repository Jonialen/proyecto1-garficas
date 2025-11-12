//! El módulo `math` provee estructuras y funciones para operaciones matemáticas básicas en 2D.

/// Representa un vector de 2 dimensiones con componentes de punto flotante.
#[derive(Clone, Copy)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    /// Crea un nuevo vector 2D con los componentes `x` e `y` dados.
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Rota el vector por un ángulo dado en radianes.
    pub fn rotate(&self, angle: f32) -> Self {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        Self {
            x: self.x * cos_a - self.y * sin_a,
            y: self.x * sin_a + self.y * cos_a,
        }
    }

    /// Devuelve un nuevo vector con la misma dirección pero con una longitud de 1.
    pub fn normalize(&self) -> Self {
        let len = (self.x * self.x + self.y * self.y).sqrt();
        if len > 0.0 {
            Self {
                x: self.x / len,
                y: self.y / len,
            }
        } else {
            *self
        }
    }

    /// Calcula la longitud (magnitud) del vector.
    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}
