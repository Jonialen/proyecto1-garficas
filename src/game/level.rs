/// Representa un nivel del juego, incluyendo su diseño y objetivos.
pub struct Level {
    /// Una matriz 2D que define la estructura del mapa del nivel.
    /// Cada número representa un tipo de celda (pared, espacio vacío, ítem, etc.).
    pub map: Vec<Vec<u8>>,
    /// El número de ítems que el jugador debe recoger para completar el nivel.
    pub required_items: usize,
    /// El nombre del nivel, que se muestra en la interfaz de usuario.
    pub name: String,
}

impl Level {
    /// Crea un nuevo nivel con un mapa, número de ítems requeridos y nombre.
    pub fn new(map: Vec<Vec<u8>>, required_items: usize, name: &str) -> Self {
        Self {
            map,
            required_items,
            name: name.to_string(),
        }
    }

    /// Devuelve el ancho del mapa del nivel.
    pub fn get_width(&self) -> usize {
        self.map[0].len()
    }

    /// Devuelve la altura del mapa del nivel.
    pub fn get_height(&self) -> usize {
        self.map.len()
    }

    /// Intenta recoger un ítem en una posición específica del mapa.
    /// Devuelve `true` si se recogió un ítem, de lo contrario `false`.
    pub fn collect_item(&mut self, x: usize, y: usize) -> bool {
        if y < self.map.len() && x < self.map[0].len() && self.map[y][x] == 5 {
            self.map[y][x] = 0; // Cambia el ítem a un espacio vacío.
            true
        } else {
            false
        }
    }
}
