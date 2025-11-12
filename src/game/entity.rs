use crate::graphics::Pixel;
use crate::math::Vec2;

/// Enumera los diferentes tipos de entidades que pueden existir en el juego.
#[derive(PartialEq)]
pub enum EntityType {
    Player,    // La entidad controlada por el jugador.
    Enemy,     // Una entidad hostil.
    Item,      // Un objeto que se puede recoger.
    Decoration, // Un objeto decorativo sin interacción.
}

/// Representa un objeto o personaje en el mundo del juego.
pub struct Entity {
    /// La posición de la entidad en el espacio 2D.
    pub position: Vec2,
    /// La representación visual de la entidad como un píxel.
    pub pixel: Pixel,
    /// El tipo de la entidad, que define su comportamiento y rol.
    pub entity_type: EntityType,
}

impl Entity {
    /// Crea una nueva entidad con una posición, apariencia y tipo específicos.
    pub fn new(x: f32, y: f32, pixel: Pixel, entity_type: EntityType) -> Self {
        Self {
            position: Vec2::new(x, y),
            pixel,
            entity_type,
        }
    }
}
