//! El módulo `game` contiene la lógica principal y las estructuras de datos del juego.

// Declaración de los submódulos que componen el módulo de juego.
pub mod camera;
pub mod entity;
pub mod player;
pub mod state;
pub mod level;

// Exporta las estructuras y enums más importantes para que sean accesibles desde otros módulos.
pub use camera::*;
pub use entity::*;
pub use player::*;
pub use state::*;
pub use level::*;
