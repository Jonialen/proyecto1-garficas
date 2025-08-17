use crossterm::{
    ExecutableCommand,
    cursor::MoveTo,
    style::{Color, Stylize},
    terminal::{Clear, ClearType, size},
    event::{poll, read, Event, KeyCode, KeyEvent},
};
use std::collections::HashMap;
use std::io::{Write, stdout};
use std::time::Duration;

#[derive(Clone, Copy, PartialEq)]
pub struct Pixel {
    pub color: Color,
    pub symbol: char,
}

impl Pixel {
    pub fn new(color: Color, symbol: char) -> Self {
        Self { color, symbol }
    }
}

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<Vec<Pixel>>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        let pixels = vec![vec![Pixel::new(Color::Black, ' '); width]; height];
        Self { width, height, pixels }
    }

    pub fn clear(&mut self, color: Color) {
        for row in &mut self.pixels {
            for pixel in row {
                pixel.color = color;
                pixel.symbol = ' ';
            }
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, pixel: Pixel) {
        if x < self.width && y < self.height {
            self.pixels[y][x] = pixel;
        }
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Option<&Pixel> {
        if x < self.width && y < self.height {
            Some(&self.pixels[y][x])
        } else {
            None
        }
    }

    // Dibujar línea simple para el raytracer
    pub fn draw_line(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, pixel: Pixel) {
        let dx = (x1 as isize - x0 as isize).abs();
        let dy = (y1 as isize - y0 as isize).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        
        let mut x = x0 as isize;
        let mut y = y0 as isize;
        
        loop {
            self.set_pixel(x as usize, y as usize, pixel);
            
            if x == x1 as isize && y == y1 as isize { break; }
            
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn rotate(&self, angle: f32) -> Self {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        Self {
            x: self.x * cos_a - self.y * sin_a,
            y: self.x * sin_a + self.y * cos_a,
        }
    }

    pub fn normalize(&self) -> Self {
        let len = (self.x * self.x + self.y * self.y).sqrt();
        if len > 0.0 {
            Self { x: self.x / len, y: self.y / len }
        } else {
            *self
        }
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

pub struct Player {
    pub position: Vec2,
    pub direction: Vec2,
    pub fov: f32,
}

impl Player {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            position: Vec2::new(x, y),
            direction: Vec2::new(1.0, 0.0),
            fov: std::f32::consts::PI / 3.0,
        }
    }

    // Crear jugador en la posición de inicio del mapa (valor 2)
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
    }

    pub fn move_forward(&mut self, distance: f32, map: &[Vec<u8>]) {
        let new_pos = Vec2::new(
            self.position.x + self.direction.x * distance,
            self.position.y + self.direction.y * distance,
        );
        
        if self.is_valid_position(new_pos, map) {
            self.position = new_pos;
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

    pub fn get_direction_symbol(&self) -> char {
        '█'
    }
}

pub struct Camera {
    pub mode: CameraMode,
    pub zoom: f32,
}

#[derive(PartialEq, Debug)]
pub enum CameraMode {
    TopDown,
    FirstPerson,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            mode: CameraMode::TopDown,
            zoom: 1.0,
        }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            CameraMode::TopDown => CameraMode::FirstPerson,
            CameraMode::FirstPerson => CameraMode::TopDown,
        };
    }
}

pub struct Entity {
    pub position: Vec2,
    pub pixel: Pixel,
    pub entity_type: EntityType,
}

#[derive(PartialEq)]
pub enum EntityType {
    Player,
    Enemy,
    Item,
    Decoration,
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

#[derive(PartialEq, Debug)]
pub enum GameState {
    Menu,
    Playing,
    Victory,
}

pub struct GameRenderer {
    cell_width: usize,
    cell_height: usize,
    offset_x: u16,
    offset_y: u16,
    map_symbols: HashMap<u8, Pixel>,
    minimap_size: usize,
    minimap_scale: usize,
}

impl GameRenderer {
    pub fn new(terminal_cols: usize, terminal_rows: usize, map_width: usize, map_height: usize) -> Self {
        let rows = terminal_rows.saturating_sub(4);
        let aspect_fix = 2.0;

        let max_cell_width_by_cols = terminal_cols / map_width;
        let max_cell_height_by_rows = rows / map_height;
        
        let (cell_width, cell_height) = if max_cell_width_by_cols as f32 / aspect_fix <= max_cell_height_by_rows as f32 {
            let cw = max_cell_width_by_cols.max(1);
            let ch = ((cw as f32 / aspect_fix) as usize).max(1);
            (cw, ch)
        } else {
            let ch = max_cell_height_by_rows.max(1);
            let cw = ((ch as f32 * aspect_fix) as usize).max(1);
            (cw, ch)
        };

        let map_pixel_width = map_width * cell_width;
        let map_pixel_height = map_height * cell_height;
        
        let offset_x = ((terminal_cols as isize - map_pixel_width as isize) / 2).max(0) as u16;
        let offset_y = ((rows as isize - map_pixel_height as isize) / 2).max(0) as u16;

        let mut map_symbols = HashMap::new();
        map_symbols.insert(0, Pixel::new(Color::Black, '█'));
        map_symbols.insert(1, Pixel::new(Color::White, '█'));
        map_symbols.insert(2, Pixel::new(Color::Green, '█'));
        map_symbols.insert(3, Pixel::new(Color::Red, '█'));
        map_symbols.insert(4, Pixel::new(Color::Blue, '█'));

        // Configuración del minimapa
        let minimap_size = 12.min(terminal_cols / 6).min(terminal_rows / 6).max(8);
        let minimap_scale = 1;

        Self {
            cell_width,
            cell_height,
            offset_x,
            offset_y,
            map_symbols,
            minimap_size,
            minimap_scale,
        }
    }

    pub fn show_menu(&self) {
        let mut stdout = stdout();
        stdout.execute(Clear(ClearType::All)).unwrap();
        
        let (cols, rows) = size().unwrap();
        let center_x = cols / 2;
        let start_y = rows / 4;

        // Título
        stdout.execute(MoveTo(center_x.saturating_sub(10), start_y)).unwrap();
        print!("{}", "🎮 RAYTRACER MAZE 🎮".with(Color::Cyan));
        
        stdout.execute(MoveTo(center_x.saturating_sub(8), start_y + 2)).unwrap();
        print!("{}", "═══════════════════".with(Color::White));

        // Controles
        stdout.execute(MoveTo(center_x.saturating_sub(12), start_y + 4)).unwrap();
        print!("{}", "⌨️  CONTROLES:".with(Color::Yellow));

        stdout.execute(MoveTo(center_x.saturating_sub(15), start_y + 6)).unwrap();
        print!("{}", "WASD o ↑↓←→  - Mover jugador".with(Color::White));
        
        stdout.execute(MoveTo(center_x.saturating_sub(15), start_y + 7)).unwrap();
        print!("{}", "Q / E        - Rotar cámara".with(Color::White));
        
        stdout.execute(MoveTo(center_x.saturating_sub(15), start_y + 8)).unwrap();
        print!("{}", "C            - Cambiar vista".with(Color::White));
        
        stdout.execute(MoveTo(center_x.saturating_sub(15), start_y + 9)).unwrap();
        print!("{}", "X / ESC      - Salir".with(Color::White));

        // Objetivo
        stdout.execute(MoveTo(center_x.saturating_sub(12), start_y + 11)).unwrap();
        print!("{}", "🎯 OBJETIVO:".with(Color::Yellow));
        
        stdout.execute(MoveTo(center_x.saturating_sub(20), start_y + 12)).unwrap();
        print!("{} {} {}", "Llega desde el inicio {} hasta la meta {}".with(Color::White), "🟩".with(Color::Green), "🟥".with(Color::Red));

        // Instrucción para continuar
        stdout.execute(MoveTo(center_x.saturating_sub(12), start_y + 15)).unwrap();
        print!("{}", "Presiona ENTER para jugar".with(Color::Green));
        
        stdout.execute(MoveTo(center_x.saturating_sub(8), start_y + 16)).unwrap();
        print!("{}", "═══════════════════".with(Color::White));

        stdout.flush().unwrap();
    }

    pub fn show_victory(&self) {
        let mut stdout = stdout();
        stdout.execute(Clear(ClearType::All)).unwrap();
        
        let (cols, rows) = size().unwrap();
        let center_x = cols / 2;
        let center_y = rows / 2;

        stdout.execute(MoveTo(center_x.saturating_sub(8), center_y.saturating_sub(3))).unwrap();
        print!("{}", "🎉 ¡VICTORIA! 🎉".with(Color::Green));
        
        stdout.execute(MoveTo(center_x.saturating_sub(12), center_y.saturating_sub(1))).unwrap();
        print!("{}", "¡Has llegado a la meta!".with(Color::Yellow));
        
        stdout.execute(MoveTo(center_x.saturating_sub(15), center_y + 1)).unwrap();
        print!("{}", "Presiona cualquier tecla para salir".with(Color::White));

        stdout.flush().unwrap();
    }

    pub fn render_top_down(&self, framebuffer: &mut Framebuffer, map: &[Vec<u8>], player: &Player, entities: &[Entity]) {
        framebuffer.clear(Color::Black);
        
        // Renderizar mapa
        for (row_idx, fila) in map.iter().enumerate() {
            for (col_idx, &celda) in fila.iter().enumerate() {
                let pixel = *self.map_symbols.get(&celda).unwrap_or(&Pixel::new(Color::Red, '?'));

                for sub_row in 0..self.cell_height {
                    for sub_col in 0..self.cell_width {
                        let fb_x = col_idx * self.cell_width + sub_col;
                        let fb_y = row_idx * self.cell_height + sub_row;
                        framebuffer.set_pixel(fb_x, fb_y, pixel);
                    }
                }
            }
        }

        // Renderizar entidades
        for entity in entities {
            let grid_x = entity.position.x as usize;
            let grid_y = entity.position.y as usize;
            
            for sub_row in 0..self.cell_height {
                for sub_col in 0..self.cell_width {
                    let fb_x = grid_x * self.cell_width + sub_col;
                    let fb_y = grid_y * self.cell_height + sub_row;
                    framebuffer.set_pixel(fb_x, fb_y, entity.pixel);
                }
            }
        }

        // Renderizar jugador
        let (grid_x, grid_y) = player.get_grid_position();
        let player_pixel = Pixel::new(Color::Cyan, '█');
        
        for sub_row in 0..self.cell_height {
            for sub_col in 0..self.cell_width {
                let fb_x = grid_x * self.cell_width + sub_col;
                let fb_y = grid_y * self.cell_height + sub_row;
                framebuffer.set_pixel(fb_x, fb_y, player_pixel);
            }
        }

        // Dibujar rayos de visión
        self.render_vision_rays(framebuffer, player, map);
    }

    pub fn render_first_person(&self, framebuffer: &mut Framebuffer, player: &Player, map: &[Vec<u8>]) {
        // Cielo y suelo usando bloques completos
        for y in 0..framebuffer.height {
            for x in 0..framebuffer.width {
                if y < framebuffer.height / 2 {
                    framebuffer.set_pixel(x, y, Pixel::new(Color::DarkBlue, '█'));
                } else {
                    framebuffer.set_pixel(x, y, Pixel::new(Color::DarkGreen, '█'));
                }
            }
        }
        
        let ray_count = framebuffer.width;
        let ray_angle_step = player.fov / ray_count as f32;
        let start_angle = -player.fov / 2.0;
        
        for i in 0..ray_count {
            let ray_angle = start_angle + i as f32 * ray_angle_step;
            let ray_dir = player.direction.rotate(ray_angle);
            
            let distance = self.cast_ray(player.position, ray_dir, map);
            let corrected_distance = distance * ray_angle.cos();
            
            let line_height = if corrected_distance > 0.0 {
                (framebuffer.height as f32 / corrected_distance) as usize
            } else {
                framebuffer.height
            };
            
            let draw_start = if line_height < framebuffer.height {
                (framebuffer.height - line_height) / 2
            } else {
                0
            };
            let draw_end = if line_height < framebuffer.height {
                (framebuffer.height + line_height) / 2
            } else {
                framebuffer.height - 1
            };
            
            let color = if corrected_distance < 2.0 {
                Color::White
            } else if corrected_distance < 4.0 {
                Color::Grey
            } else if corrected_distance < 8.0 {
                Color::DarkGrey
            } else {
                Color::Black
            };
            
            for y in draw_start..=draw_end.min(framebuffer.height - 1) {
                framebuffer.set_pixel(i, y, Pixel::new(color, '█'));
            }
        }

        // Renderizar minimapa en primera persona
        self.render_minimap(framebuffer, player, map);
    }

    fn render_minimap(&self, framebuffer: &mut Framebuffer, player: &Player, map: &[Vec<u8>]) {
    // Verificar que tenemos espacio suficiente para el minimapa
    let total_minimap_width = self.minimap_size + 2;
    let total_minimap_height = self.minimap_size + 2;
    
    if framebuffer.width < total_minimap_width + 2 || framebuffer.height < total_minimap_height + 4 {
        return; // No renderizar minimapa si la pantalla es muy pequeña
    }
    
    // Calcular posición desde la esquina superior derecha
    let minimap_x = framebuffer.width - total_minimap_width - 1; // -1 para margen
    let minimap_y = 1; // Margen superior
    
    // Marco del minimapa
    for i in 0..total_minimap_width {
        for j in 0..total_minimap_height {
            let x = minimap_x + i;
            let y = minimap_y + j;
            
            // Verificar límites antes de dibujar (aunque ya deberían estar bien)
            if x < framebuffer.width && y < framebuffer.height {
                if i == 0 || i == total_minimap_width - 1 || j == 0 || j == total_minimap_height - 1 {
                    framebuffer.set_pixel(x, y, Pixel::new(Color::Grey, '█'));
                }
            }
        }
    }

    // Contenido del minimapa
    let player_map_x = player.position.x as usize;
    let player_map_y = player.position.y as usize;
    let half_size = self.minimap_size / 2;
    
    for dy in 0..self.minimap_size {
        for dx in 0..self.minimap_size {
            // Calcular posición en el mundo
            let world_x = if player_map_x >= half_size {
                player_map_x - half_size + dx
            } else {
                dx.saturating_sub(half_size - player_map_x)
            };
            
            let world_y = if player_map_y >= half_size {
                player_map_y - half_size + dy
            } else {
                dy.saturating_sub(half_size - player_map_y)
            };
            
            // Posición en pantalla (dentro del marco)
            let screen_x = minimap_x + 1 + dx;
            let screen_y = minimap_y + 1 + dy;
            
            // Verificar límites del framebuffer (redundante pero seguro)
            if screen_x >= framebuffer.width || screen_y >= framebuffer.height {
                continue;
            }
            
            let pixel = if world_x == player_map_x && world_y == player_map_y {
                // Jugador en el minimapa
                Pixel::new(Color::Cyan, '█')
            } else if world_y < map.len() && world_x < map[0].len() {
                // Celda del mapa
                match map[world_y][world_x] {
                    0 => Pixel::new(Color::DarkGrey, '█'),  // Espacio vacío
                    1 => Pixel::new(Color::White, '█'),     // Pared
                    2 => Pixel::new(Color::Green, '█'),     // Inicio
                    3 => Pixel::new(Color::Red, '█'),       // Final
                    4 => Pixel::new(Color::Blue, '█'),      // Agua
                    _ => Pixel::new(Color::Black, '█'),
                }
            } else {
                // Fuera del mapa
                Pixel::new(Color::Black, '█')
            };
            
            framebuffer.set_pixel(screen_x, screen_y, pixel);
        }
    }

    // Indicador de dirección del jugador en el minimapa
    let center_x = minimap_x + 1 + half_size;
    let center_y = minimap_y + 1 + half_size;
    let dir_x = (center_x as f32 + player.direction.x * 2.0).round() as usize;
    let dir_y = (center_y as f32 + player.direction.y * 2.0).round() as usize;
    
    // Verificar que la dirección esté dentro del minimapa
    if dir_x >= minimap_x + 1 && dir_x < minimap_x + total_minimap_width - 1 &&
       dir_y >= minimap_y + 1 && dir_y < minimap_y + total_minimap_height - 1 &&
       dir_x < framebuffer.width && dir_y < framebuffer.height {
        framebuffer.set_pixel(dir_x, dir_y, Pixel::new(Color::Yellow, '█'));
    }
}

    fn render_vision_rays(&self, framebuffer: &mut Framebuffer, player: &Player, map: &[Vec<u8>]) {
        let center_x = (player.position.x * self.cell_width as f32) as usize;
        let center_y = (player.position.y * self.cell_height as f32) as usize;
        
        for i in 0..5 {
            let angle_offset = (i as f32 - 2.0) * (player.fov / 4.0);
            let ray_dir = player.direction.rotate(angle_offset);
            
            let distance = self.cast_ray(player.position, ray_dir, map);
            let end_x = player.position.x + ray_dir.x * distance;
            let end_y = player.position.y + ray_dir.y * distance;
            
            let screen_end_x = (end_x * self.cell_width as f32) as usize;
            let screen_end_y = (end_y * self.cell_height as f32) as usize;
            
            let safe_end_x = screen_end_x.min(framebuffer.width - 1);
            let safe_end_y = screen_end_y.min(framebuffer.height - 1);
            
            framebuffer.draw_line(
                center_x, center_y,
                safe_end_x, safe_end_y,
                Pixel::new(Color::Yellow, '█')
            );
        }
    }

    fn cast_ray(&self, start: Vec2, direction: Vec2, map: &[Vec<u8>]) -> f32 {
        let mut map_x = start.x as i32;
        let mut map_y = start.y as i32;
        
        let delta_dist_x = (1.0 / direction.x).abs();
        let delta_dist_y = (1.0 / direction.y).abs();
        
        let (step_x, mut side_dist_x) = if direction.x < 0.0 {
            (-1, (start.x - map_x as f32) * delta_dist_x)
        } else {
            (1, (map_x as f32 + 1.0 - start.x) * delta_dist_x)
        };
        
        let (step_y, mut side_dist_y) = if direction.y < 0.0 {
            (-1, (start.y - map_y as f32) * delta_dist_y)
        } else {
            (1, (map_y as f32 + 1.0 - start.y) * delta_dist_y)
        };
        
        let mut hit = false;
        let mut side = 0;
        
        while !hit {
            if side_dist_x < side_dist_y {
                side_dist_x += delta_dist_x;
                map_x += step_x;
                side = 0;
            } else {
                side_dist_y += delta_dist_y;
                map_y += step_y;
                side = 1;
            }
            
            if map_x < 0 || map_y < 0 || 
               map_y as usize >= map.len() || 
               map_x as usize >= map[0].len() || 
               map[map_y as usize][map_x as usize] == 1 {
                hit = true;
            }
        }
        
        let perp_wall_dist = if side == 0 {
            (map_x as f32 - start.x + (1 - step_x) as f32 / 2.0) / direction.x
        } else {
            (map_y as f32 - start.y + (1 - step_y) as f32 / 2.0) / direction.y
        };
        
        perp_wall_dist.abs()
    }

    pub fn display_framebuffer(&self, framebuffer: &Framebuffer) {
    let mut stdout = stdout();
    stdout.execute(Clear(ClearType::All)).unwrap();

    // No usar offset si el framebuffer es del tamaño completo de la terminal
    let use_offset = framebuffer.width < size().unwrap().0 as usize;
    
    for (y, row) in framebuffer.pixels.iter().enumerate() {
        let x_pos = if use_offset { self.offset_x } else { 0 };
        let y_pos = if use_offset { self.offset_y + y as u16 } else { y as u16 };
        
        stdout.execute(MoveTo(x_pos, y_pos)).unwrap();
        
        for pixel in row {
            let styled = pixel.symbol.to_string().with(pixel.color);
            print!("{}", styled);
        }
    }
    stdout.flush().unwrap();
}
}

fn main() {
    let mapa: Vec<Vec<u8>> = vec![
        vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        vec![1, 2, 0, 0, 0, 0, 0, 0, 0, 1],
        vec![1, 0, 1, 0, 1, 1, 0, 1, 0, 1],
        vec![1, 0, 0, 0, 0, 0, 0, 0, 3, 1],
        vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    ];

    let map_height = mapa.len();
    let map_width = mapa[0].len();

    let (cols, rows) = size().unwrap();
    let cols = cols as usize;
    let rows = rows as usize;

    let renderer = GameRenderer::new(cols, rows, map_width, map_height);
    
    let mut player = Player::from_map(&mapa);
    let mut camera = Camera::new();
    let mut game_state = GameState::Menu;
    
    let entities = vec![
        Entity::new(3.0, 3.0, Pixel::new(Color::Magenta, '█'), EntityType::Item),
        Entity::new(7.0, 1.0, Pixel::new(Color::Yellow, '█'), EntityType::Enemy),
    ];

    crossterm::terminal::enable_raw_mode().unwrap();

    let mut running = true;
    while running {
        match game_state {
            GameState::Menu => {
                renderer.show_menu();
                
                if poll(Duration::from_millis(16)).unwrap() {
                    if let Ok(Event::Key(KeyEvent { code, .. })) = read() {
                        match code {
                            KeyCode::Enter => game_state = GameState::Playing,
                            KeyCode::Char('x') | KeyCode::Esc => running = false,
                            _ => {}
                        }
                    }
                }
            },
            
            GameState::Playing => {
                // Verificar victoria
                if player.is_at_goal(&mapa) {
                    game_state = GameState::Victory;
                    continue;
                }

                // Crear framebuffer según el modo de cámara
                let mut framebuffer = match camera.mode {
                    CameraMode::TopDown => Framebuffer::new(
                        map_width * renderer.cell_width,
                        map_height * renderer.cell_height
                    ),
                    CameraMode::FirstPerson => Framebuffer::new(cols, rows.saturating_sub(4)),
                };

                // Renderizar según el modo de cámara
                match camera.mode {
                    CameraMode::TopDown => {
                        renderer.render_top_down(&mut framebuffer, &mapa, &player, &entities);
                    }
                    CameraMode::FirstPerson => {
                        renderer.render_first_person(&mut framebuffer, &player, &mapa);
                    }
                }
                
                renderer.display_framebuffer(&framebuffer);

                // Debug info
                let (px, py) = player.get_grid_position();
                println!("\n[CONTROLS] WASD:move, QE:rotate, C:camera mode, X:quit");
                println!("\n[DEBUG] Player: ({:.1}, {:.1}) grid: ({}, {}), Camera: {:?}", 
                        player.position.x, player.position.y, px, py, camera.mode);
                println!("\n[MINIMAP] Visible en modo First Person (esquina superior derecha)");

                // Input handling
                if poll(Duration::from_millis(16)).unwrap() {
                    if let Ok(Event::Key(KeyEvent { code, .. })) = read() {
                        match code {
                            KeyCode::Char('w') | KeyCode::Up => player.move_forward(0.2, &mapa),
                            KeyCode::Char('s') | KeyCode::Down => player.move_backward(0.2, &mapa),
                            KeyCode::Char('a') | KeyCode::Left => player.strafe(-0.2, &mapa),
                            KeyCode::Char('d') | KeyCode::Right => player.strafe(0.2, &mapa),
                            KeyCode::Char('q') => player.rotate(-0.1),
                            KeyCode::Char('e') => player.rotate(0.1),
                            KeyCode::Char('c') => camera.toggle_mode(),
                            KeyCode::Char('x') | KeyCode::Esc => running = false,
                            _ => {}
                        }
                    }
                }
            },
            
            GameState::Victory => {
                renderer.show_victory();
                
                if poll(Duration::from_millis(16)).unwrap() {
                    if let Ok(Event::Key(KeyEvent { .. })) = read() {
                        running = false;
                    }
                }
            }
        }
    }

    crossterm::terminal::disable_raw_mode().unwrap();
    println!("¡Gracias por jugar!");
}