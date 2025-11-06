use crate::math::Vec2;

pub struct Player {
    pub position: Vec2,
    pub direction: Vec2,
    pub fov: f32,
    pub has_moved: bool,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            position: Vec2::new(x, y),
            direction: Vec2::new(1.0, 0.0),
            fov: std::f32::consts::PI / 3.0,
            has_moved: false,
        }
    }

    pub fn from_map(map: &[Vec<u8>]) -> Self {
        for (y, row) in map.iter().enumerate() {
            for (x, &cell) in row.iter().enumerate() {
                if cell == 2 {
                    return Self::new(x as f32 + 0.5, y as f32 + 0.5);
                }
            }
        }
        Self::new(1.5, 1.5)
    }

    pub fn rotate(&mut self, angle: f32) {
        self.direction = self.direction.rotate(angle);
        self.has_moved = true;
    }

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

    pub fn move_backward(&mut self, distance: f32, map: &[Vec<u8>]) {
        self.move_forward(-distance, map);
    }

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

    fn is_valid_position(&self, pos: Vec2, map: &[Vec<u8>]) -> bool {
        let x = pos.x as usize;
        let y = pos.y as usize;

        if y >= map.len() || x >= map[0].len() {
            return false;
        }

        let cell = map[y][x];
        cell != 1
    }

    pub fn is_at_goal(&self, map: &[Vec<u8>]) -> bool {
        let x = self.position.x as usize;
        let y = self.position.y as usize;

        if y < map.len() && x < map[0].len() {
            map[y][x] == 3
        } else {
            false
        }
    }

    pub fn get_grid_position(&self) -> (usize, usize) {
        (self.position.x as usize, self.position.y as usize)
    }
}