//! # Biblioteca del Juego Laberinto Raytracer
//!
//! Este archivo es el punto de entrada de la biblioteca del juego. Se encarga de
//! organizar y exponer los módulos principales del proyecto: `game`, `graphics` y `math`.

// Declaración de los módulos que componen la biblioteca.
pub mod game;
pub mod graphics;
pub mod math;

// Exporta todo el contenido de los módulos para que sea accesible por el binario principal.
pub use game::*;
pub use graphics::*;
pub use math::*;
