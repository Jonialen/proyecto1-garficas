use crate::graphics::Pixel;
use crate::math::Vec2;

#[derive(PartialEq)]
pub enum EntityType {
    Player,
    Enemy,
    Item,
    Decoration,
}

pub struct Entity {
    pub position: Vec2,
    pub pixel: Pixel,
    pub entity_type: EntityType,
}

impl Entity {
    pub fn new(x: f32, y: f32, pixel: Pixel, entity_type: EntityType) -> Self {
        Self {
            position: Vec2::new(x, y),
            pixel,
            entity_type,
        }
    }
}
