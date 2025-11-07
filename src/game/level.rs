pub struct Level {
    pub map: Vec<Vec<u8>>,
    pub required_items: usize,
    pub name: String,
}

impl Level {
    pub fn new(map: Vec<Vec<u8>>, required_items: usize, name: &str) -> Self {
        Self {
            map,
            required_items,
            name: name.to_string(),
        }
    }

    pub fn get_width(&self) -> usize {
        self.map[0].len()
    }

    pub fn get_height(&self) -> usize {
        self.map.len()
    }

    pub fn collect_item(&mut self, x: usize, y: usize) -> bool {
        if y < self.map.len() && x < self.map[0].len() && self.map[y][x] == 5 {
            self.map[y][x] = 0; // Cambiar el item a espacio vacío
            true
        } else {
            false
        }
    }
}