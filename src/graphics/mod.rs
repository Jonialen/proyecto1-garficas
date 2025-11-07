//! El módulo `graphics` contiene todas las estructuras y funciones relacionadas con el renderizado.

// Declaración de los submódulos que componen el módulo de gráficos.
pub mod framebuffer;
pub mod pixel;
pub mod renderer;

// Exporta las estructuras y enums más importantes para que sean accesibles desde otros módulos.
pub use framebuffer::*;
pub use pixel::*;
pub use renderer::*;
