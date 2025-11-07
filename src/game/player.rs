use crate::math::Vec2;

/// Representa al jugador en el juego.
pub struct Player {
    /// La posición actual del jugador en el mapa.
    pub position: Vec2,
    /// El vector de dirección que indica hacia dónde está mirando el jugador.
    pub direction: Vec2,
    /// El campo de visión (Field of View) del jugador, en radianes.
    pub fov: f32,
    /// Un indicador para saber si el jugador se ha movido, útil para optimizar el renderizado.
    pub has_moved: bool,
}

impl Player {
    /// Crea un nuevo jugador en una posición específica.
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            position: Vec2::new(x, y),
            direction: Vec2::new(1.0, 0.0), // Dirección inicial hacia la derecha.
            fov: std::f32::consts::PI / 3.0, // Campo de visión de 60 grados.
            has_moved: false,
        }
    }

    /// Crea un jugador a partir de la posición inicial definida en el mapa.
    pub fn from_map(map: &[Vec<u8>]) -> Self {
        for (y, row) in map.iter().enumerate() {
            for (x, &cell) in row.iter().enumerate() {
                if cell == 2 { // El número 2 representa la posición inicial del jugador.
                    return Self::new(x as f32 + 0.5, y as f32 + 0.5);
                }
            }
        }
        Self::new(1.5, 1.5) // Posición de respaldo si no se encuentra en el mapa.
    }

    /// Rota la dirección del jugador en un ángulo determinado.
    pub fn rotate(&mut self, angle: f32) {
        self.direction = self.direction.rotate(angle);
        self.has_moved = true;
    }

    /// Mueve al jugador hacia adelante, evitando colisiones con las paredes.
    pub fn move_forward(&mut self, distance: f32, map: &[Vec<u8>]) {
        let new_pos = Vec2::new(
            self.position.x + self.direction.x * distance,
            self.position.y + self.direction.y * distance,
        );
        
        if self.is_valid_position(new_pos, map) {
            self.position = new_pos;
            self.has_moved = true;
        }
    }

    /// Mueve al jugador hacia atrás, evitando colisiones con las paredes.
    pub fn move_backward(&mut self, distance: f32, map: &[Vec<u8>]) {
        let new_pos = Vec2::new(
            self.position.x - self.direction.x * distance,
            self.position.y - self.direction.y * distance,
        );
        
        if self.is_valid_position(new_pos, map) {
            self.position = new_pos;
            self.has_moved = true;
        }
    }

    /// Mueve al jugador lateralmente (strafe), evitando colisiones.
    pub fn strafe(&mut self, distance: f32, map: &[Vec<u8>]) {
        let strafe_dir = self.direction.rotate(std::f32::consts::PI / 2.0);
        let new_pos = Vec2::new(
            self.position.x + strafe_dir.x * distance,
            self.position.y + strafe_dir.y * distance,
        );

        if self.is_valid_position(new_pos, map) {
            self.position = new_pos;
            self.has_moved = true;
        }
    }

    /// Comprueba si una posición es válida (no es una pared y está dentro de los límites del mapa).
    fn is_valid_position(&self, pos: Vec2, map: &[Vec<u8>]) -> bool {
        let x = pos.x as usize;
        let y = pos.y as usize;

        if y >= map.len() || x >= map[0].len() {
            return false;
        }

        let cell = map[y][x];
        cell != 1 // El número 1 representa una pared.
    }

    /// Comprueba si el jugador ha llegado a la celda objetivo.
    pub fn is_at_goal(&self, map: &[Vec<u8>]) -> bool {
        let x = self.position.x as usize;
        let y = self.position.y as usize;

        if y < map.len() && x < map[0].len() {
            map[y][x] == 3 // El número 3 representa la meta.
        } else {
            false
        }
    }

    /// Devuelve la posición del jugador en la cuadrícula del mapa.
    pub fn get_grid_position(&self) -> (usize, usize) {
        (self.position.x as usize, self.position.y as usize)
    }
}
