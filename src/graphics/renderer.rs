use crossterm::{
    ExecutableCommand,
    cursor::MoveTo,
    style::{Color, Stylize},
    terminal::{Clear, ClearType, size},
};
use std::collections::{HashMap, HashSet};
use std::io::{Write, stdout};

use crate::game::{CameraMode, Entity, Player};
use crate::graphics::{Framebuffer, Pixel};
use crate::math::Vec2;
use crate::game::Level;

pub struct GameRenderer {
    cell_width: usize,
    cell_height: usize,
    offset_x: u16,
    offset_y: u16,
    map_symbols: HashMap<u8, Pixel>,
    minimap_size: usize,
    collected_positions: HashSet<(usize, usize)>,
}

impl GameRenderer {
    pub fn new(
        terminal_cols: usize,
        terminal_rows: usize,
        map_width: usize,
        map_height: usize,
    ) -> Self {
        let rows = terminal_rows.saturating_sub(4);
        let aspect_fix = 2.0;

        let max_cell_width_by_cols = terminal_cols / map_width;
        let max_cell_height_by_rows = rows / map_height;

        let (cell_width, cell_height) =
            if max_cell_width_by_cols as f32 / aspect_fix <= max_cell_height_by_rows as f32 {
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
        map_symbols.insert(0, Pixel::new(Color::DarkGrey, '·'));
        map_symbols.insert(1, Pixel::new(Color::White, '█'));
        map_symbols.insert(2, Pixel::new(Color::Green, '█'));
        map_symbols.insert(3, Pixel::new(Color::Red, '█'));
        map_symbols.insert(5, Pixel::new(Color::Yellow, '◆'));

        let minimap_size = 14.min(terminal_cols / 6).min(terminal_rows / 6).max(10);

        Self {
            cell_width,
            cell_height,
            offset_x,
            offset_y,
            map_symbols,
            minimap_size,
            collected_positions: HashSet::new(),
        }
    }

    pub fn show_menu(&self) {
        let mut stdout = stdout();
        stdout.execute(Clear(ClearType::All)).unwrap();

        let (cols, rows) = size().unwrap();
        let center_x = cols / 2;
        let start_y = rows / 4;

        stdout.execute(MoveTo(center_x.saturating_sub(15), start_y)).unwrap();
        print!("{}", "🎮 LABERINTO DE RECOLECCIÓN 🎮".with(Color::Cyan).bold());

        stdout.execute(MoveTo(center_x.saturating_sub(10), start_y + 2)).unwrap();
        print!("{}", "═══════════════════════".with(Color::DarkCyan));

        stdout.execute(MoveTo(center_x.saturating_sub(10), start_y + 4)).unwrap();
        print!("{}", "⌨️  CONTROLES:".with(Color::Yellow).bold());

        let controls = [
            "WASD / ↑↓←→  - Mover jugador",
            "Q / E        - Rotar cámara",
            "C            - Cambiar vista",
            "X / ESC      - Salir",
        ];

        for (i, control) in controls.iter().enumerate() {
            stdout.execute(MoveTo(center_x.saturating_sub(15), start_y + 6 + i as u16)).unwrap();
            print!("{}", control.with(Color::White));
        }

        stdout.execute(MoveTo(center_x.saturating_sub(10), start_y + 11)).unwrap();
        print!("{}", "🎯 OBJETIVO:".with(Color::Yellow).bold());

        stdout.execute(MoveTo(center_x.saturating_sub(22), start_y + 12)).unwrap();
        print!("{}", "Recolecta ◆ y llega a la meta 🟥 en 3 niveles".with(Color::White));

        stdout.execute(MoveTo(center_x.saturating_sub(15), start_y + 15)).unwrap();
        print!("{}", "Presiona ENTER para jugar".with(Color::Green).bold());

        stdout.flush().unwrap();
    }

    pub fn show_victory(&self) {
        let mut stdout = stdout();
        stdout.execute(Clear(ClearType::All)).unwrap();

        let (cols, rows) = size().unwrap();
        let center_x = cols / 2;
        let center_y = rows / 2;

        stdout.execute(MoveTo(center_x.saturating_sub(15), center_y.saturating_sub(3))).unwrap();
        print!("{}", "🎉 ¡FELICIDADES! 🎉".with(Color::Green).bold());

        stdout.execute(MoveTo(center_x.saturating_sub(18), center_y.saturating_sub(1))).unwrap();
        print!("{}", "¡Has completado todos los niveles!".with(Color::Yellow));

        stdout.execute(MoveTo(center_x.saturating_sub(18), center_y + 1)).unwrap();
        print!("{}", "Presiona cualquier tecla para salir".with(Color::White));

        stdout.flush().unwrap();
    }

    pub fn render_top_down(
        &mut self,
        framebuffer: &mut Framebuffer,
        map: &[Vec<u8>],
        player: &Player,
        _entities: &[Entity],
        collected_items: usize,
    ) {
        framebuffer.clear(Color::Black);

        let (px, py) = player.get_grid_position();
        if map[py][px] == 5 {
            self.collected_positions.insert((px, py));
        }

        let map_width = map[0].len();
        let map_height = map.len();
        
        // Dibujar borde superior
        for x in 0..map_width * self.cell_width {
            framebuffer.set_pixel(x, 0, Pixel::new(Color::DarkCyan, '═'));
        }
        
        // Dibujar borde inferior
        for x in 0..map_width * self.cell_width {
            let y = map_height * self.cell_height - 1;
            if y < framebuffer.height {
                framebuffer.set_pixel(x, y, Pixel::new(Color::DarkCyan, '═'));
            }
        }
        
        // Dibujar bordes laterales
        for y in 0..map_height * self.cell_height {
            if y < framebuffer.height {
                framebuffer.set_pixel(0, y, Pixel::new(Color::DarkCyan, '║'));
                let x = map_width * self.cell_width - 1;
                if x < framebuffer.width {
                    framebuffer.set_pixel(x, y, Pixel::new(Color::DarkCyan, '║'));
                }
            }
        }
        
        // Esquinas
        framebuffer.set_pixel(0, 0, Pixel::new(Color::DarkCyan, '╔'));
        framebuffer.set_pixel(map_width * self.cell_width - 1, 0, Pixel::new(Color::DarkCyan, '╗'));
        let bottom_left_y = map_height * self.cell_height - 1;
        if bottom_left_y < framebuffer.height {
            framebuffer.set_pixel(0, bottom_left_y, Pixel::new(Color::DarkCyan, '╚'));
            framebuffer.set_pixel(
                map_width * self.cell_width - 1, 
                bottom_left_y, 
                Pixel::new(Color::DarkCyan, '╝')
            );
        }

        // Renderizar mapa con mejor contraste
        for (row_idx, fila) in map.iter().enumerate() {
            for (col_idx, &celda) in fila.iter().enumerate() {
                let actual_cell = if celda == 5 && self.collected_positions.contains(&(col_idx, row_idx)) {
                    0
                } else {
                    celda
                };

                let pixel = match actual_cell {
                    0 => Pixel::new(Color::Black, ' '),
                    1 => Pixel::new(Color::White, '█'),
                    2 => Pixel::new(Color::Green, '▓'),
                    3 => Pixel::new(Color::Red, '▓'),
                    5 => Pixel::new(Color::Yellow, '◆'),
                    _ => Pixel::new(Color::Red, '?'),
                };

                // Agregar sombra/profundidad a las paredes
                let enhanced_pixel = if actual_cell == 1 {
                    // Variar ligeramente las paredes según posición para dar textura
                    if (col_idx + row_idx) % 3 == 0 {
                        Pixel::new(Color::Grey, '█')
                    } else {
                        pixel
                    }
                } else {
                    pixel
                };

                for sub_row in 0..self.cell_height {
                    for sub_col in 0..self.cell_width {
                        let fb_x = col_idx * self.cell_width + sub_col;
                        let fb_y = row_idx * self.cell_height + sub_row;
                        
                        // No sobrescribir el borde
                        if fb_x > 0 && fb_x < framebuffer.width - 1 && 
                           fb_y > 0 && fb_y < framebuffer.height - 1 {
                            framebuffer.set_pixel(fb_x, fb_y, enhanced_pixel);
                        }
                    }
                }
            }
        }

        // Renderizar jugador con dirección más clara
        let player_center_x = px * self.cell_width + self.cell_width / 2;
        let player_center_y = py * self.cell_height + self.cell_height / 2;
        
        // Cuerpo del jugador
        for sub_row in 0..self.cell_height {
            for sub_col in 0..self.cell_width {
                let fb_x = px * self.cell_width + sub_col;
                let fb_y = py * self.cell_height + sub_row;
                if fb_x > 0 && fb_x < framebuffer.width - 1 && 
                   fb_y > 0 && fb_y < framebuffer.height - 1 {
                    framebuffer.set_pixel(fb_x, fb_y, Pixel::new(Color::Cyan, '●'));
                }
            }
        }
        
        // Indicador de dirección del jugador
        let dir_length = (self.cell_width.max(self.cell_height) as f32 * 0.8) as usize;
        for i in 1..=dir_length {
            let dir_x = player_center_x as f32 + player.direction.x * i as f32;
            let dir_y = player_center_y as f32 + player.direction.y * i as f32;
            let dx = dir_x as usize;
            let dy = dir_y as usize;
            
            if dx > 0 && dx < framebuffer.width - 1 && 
               dy > 0 && dy < framebuffer.height - 1 {
                let symbol = if i == dir_length { '▶' } else { '─' };
                framebuffer.set_pixel(dx, dy, Pixel::new(Color::Yellow, symbol));
            }
        }

        self.render_vision_rays(framebuffer, player, map);
    }

    pub fn render_first_person(
        &mut self,
        framebuffer: &mut Framebuffer,
        player: &Player,
        map: &[Vec<u8>],
        collected_items: usize,
    ) {
        let (px, py) = player.get_grid_position();
        if map[py][px] == 5 {
            self.collected_positions.insert((px, py));
        }

        // Cielo - sólido con gradiente
        for y in 0..framebuffer.height / 2 {
            for x in 0..framebuffer.width {
                let color = if y < framebuffer.height / 6 {
                    Color::DarkBlue
                } else if y < framebuffer.height / 4 {
                    Color::Blue
                } else {
                    Color::DarkCyan
                };
                
                framebuffer.set_pixel(x, y, Pixel::new(color, '█'));
            }
        }

        // Suelo - sólido con gradiente
        for y in framebuffer.height / 2..framebuffer.height {
            for x in 0..framebuffer.width {
                let color = if y > framebuffer.height * 4 / 5 {
                    Color::Black
                } else if y > framebuffer.height * 7 / 10 {
                    Color::DarkGrey
                } else {
                    Color::DarkGreen
                };
                
                framebuffer.set_pixel(x, y, Pixel::new(color, '█'));
            }
        }

        let ray_count = framebuffer.width;
        let ray_angle_step = player.fov / ray_count as f32;
        let start_angle = -player.fov / 2.0;

        for i in 0..ray_count {
            let ray_angle = start_angle + i as f32 * ray_angle_step;
            let ray_dir = player.direction.rotate(ray_angle);

            let (distance, hit_type) = self.cast_ray_with_type(player.position, ray_dir, map);
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

            let base_color = match hit_type {
                1 => Color::White,
                _ => Color::Grey,
            };

            let color = if corrected_distance < 2.0 {
                base_color
            } else if corrected_distance < 4.0 {
                Color::Grey
            } else if corrected_distance < 8.0 {
                Color::DarkGrey
            } else {
                Color::Black
            };

            let symbol = if corrected_distance < 3.0 { '█' } else { '▓' };

            for y in draw_start..=draw_end.min(framebuffer.height - 1) {
                framebuffer.set_pixel(i, y, Pixel::new(color, symbol));
            }
        }

        self.render_minimap(framebuffer, player, map, collected_items);
    }

    fn render_minimap(&self, framebuffer: &mut Framebuffer, player: &Player, map: &[Vec<u8>], collected_items: usize) {
        let total_minimap_width = self.minimap_size + 2;
        let total_minimap_height = self.minimap_size + 2;

        if framebuffer.width < total_minimap_width + 2
            || framebuffer.height < total_minimap_height + 4
        {
            return;
        }

        let minimap_x = framebuffer.width - total_minimap_width - 1;
        let minimap_y = 1;

        // Marco del minimapa con estilo mejorado
        for i in 0..total_minimap_width {
            for j in 0..total_minimap_height {
                let x = minimap_x + i;
                let y = minimap_y + j;

                if x < framebuffer.width && y < framebuffer.height {
                    if i == 0 && j == 0 {
                        framebuffer.set_pixel(x, y, Pixel::new(Color::Yellow, '╔'));
                    } else if i == total_minimap_width - 1 && j == 0 {
                        framebuffer.set_pixel(x, y, Pixel::new(Color::Yellow, '╗'));
                    } else if i == 0 && j == total_minimap_height - 1 {
                        framebuffer.set_pixel(x, y, Pixel::new(Color::Yellow, '╚'));
                    } else if i == total_minimap_width - 1 && j == total_minimap_height - 1 {
                        framebuffer.set_pixel(x, y, Pixel::new(Color::Yellow, '╝'));
                    } else if i == 0 || i == total_minimap_width - 1 {
                        framebuffer.set_pixel(x, y, Pixel::new(Color::Yellow, '║'));
                    } else if j == 0 || j == total_minimap_height - 1 {
                        framebuffer.set_pixel(x, y, Pixel::new(Color::Yellow, '═'));
                    }
                }
            }
        }

        let player_map_x = player.position.x as usize;
        let player_map_y = player.position.y as usize;
        let half_size = self.minimap_size / 2;

        for dy in 0..self.minimap_size {
            for dx in 0..self.minimap_size {
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

                let screen_x = minimap_x + 1 + dx;
                let screen_y = minimap_y + 1 + dy;

                if screen_x >= framebuffer.width || screen_y >= framebuffer.height {
                    continue;
                }

                let pixel = if world_x == player_map_x && world_y == player_map_y {
                    Pixel::new(Color::Cyan, '●')
                } else if world_y < map.len() && world_x < map[0].len() {
                    let cell = map[world_y][world_x];
                    if cell == 5 && self.collected_positions.contains(&(world_x, world_y)) {
                        Pixel::new(Color::Black, ' ')
                    } else {
                        match cell {
                            0 => Pixel::new(Color::Black, ' '),
                            1 => Pixel::new(Color::White, '█'),
                            2 => Pixel::new(Color::Green, '▓'),
                            3 => Pixel::new(Color::Red, '▓'),
                            5 => Pixel::new(Color::Yellow, '◆'),
                            _ => Pixel::new(Color::Black, '?'),
                        }
                    }
                } else {
                    Pixel::new(Color::Black, ' ')
                };

                framebuffer.set_pixel(screen_x, screen_y, pixel);
            }
        }

        // Indicador de dirección mejorado
        let center_x = minimap_x + 1 + half_size;
        let center_y = minimap_y + 1 + half_size;
        let dir_x = (center_x as f32 + player.direction.x * 2.0).round() as usize;
        let dir_y = (center_y as f32 + player.direction.y * 2.0).round() as usize;

        if dir_x >= minimap_x + 1
            && dir_x < minimap_x + total_minimap_width - 1
            && dir_y >= minimap_y + 1
            && dir_y < minimap_y + total_minimap_height - 1
            && dir_x < framebuffer.width
            && dir_y < framebuffer.height
        {
            framebuffer.set_pixel(dir_x, dir_y, Pixel::new(Color::Red, '▲'));
        }
    }

    fn render_vision_rays(&self, framebuffer: &mut Framebuffer, player: &Player, map: &[Vec<u8>]) {
        let center_x = (player.position.x * self.cell_width as f32) as usize;
        let center_y = (player.position.y * self.cell_height as f32) as usize;

        // Renderizar menos rayos y más sutiles
        for i in 0..5 {
            let angle_offset = (i as f32 - 2.0) * (player.fov / 4.0);
            let ray_dir = player.direction.rotate(angle_offset);

            let (distance, _) = self.cast_ray_with_type(player.position, ray_dir, map);
            let end_x = player.position.x + ray_dir.x * distance;
            let end_y = player.position.y + ray_dir.y * distance;

            let screen_end_x = (end_x * self.cell_width as f32) as usize;
            let screen_end_y = (end_y * self.cell_height as f32) as usize;

            let safe_end_x = screen_end_x.min(framebuffer.width - 1);
            let safe_end_y = screen_end_y.min(framebuffer.height - 1);

            framebuffer.draw_line(
                center_x,
                center_y,
                safe_end_x,
                safe_end_y,
                Pixel::new(Color::DarkYellow, '·'),
            );
        }
    }

    fn cast_ray_with_type(&self, start: Vec2, direction: Vec2, map: &[Vec<u8>]) -> (f32, u8) {
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
        let mut hit_type = 0u8;

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

            if map_x < 0 || map_y < 0 || map_y as usize >= map.len() || map_x as usize >= map[0].len() {
                hit = true;
                hit_type = 1;
            } else {
                let cell = map[map_y as usize][map_x as usize];
                if cell == 1 {
                    hit = true;
                    hit_type = cell;
                }
            }
        }

        let perp_wall_dist = if side == 0 {
            (map_x as f32 - start.x + (1 - step_x) as f32 / 2.0) / direction.x
        } else {
            (map_y as f32 - start.y + (1 - step_y) as f32 / 2.0) / direction.y
        };

        (perp_wall_dist.abs(), hit_type)
    }

    pub fn display_framebuffer(&self, framebuffer: &Framebuffer) {
        let mut stdout = stdout();
        
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

    pub fn display_ui(
        &self,
        fps: f32,
        px: f32,
        py: f32,
        grid_x: usize,
        grid_y: usize,
        mode: CameraMode,
        collected: usize,
        required: usize,
        level: usize,
        total_levels: usize,
        level_name: &str,
    ) {
        let mut stdout = stdout();
        let (_, rows) = size().unwrap();
        
        stdout.execute(MoveTo(0, rows.saturating_sub(3))).unwrap();
        print!("{}", format!("[WASD:move | QE:rotate | C:camera | X:quit] FPS: {:.0}", fps).with(Color::White));
        
        stdout.execute(MoveTo(0, rows.saturating_sub(2))).unwrap();
        print!("{}", format!(
            "Nivel {}/{}: {} | Items: {}/{} ◆ | Pos: ({:.1}, {:.1})",
            level, total_levels, level_name, collected, required, px, py
        ).with(Color::Yellow));
        
        stdout.flush().unwrap();
    }

    pub fn get_cell_width(&self) -> usize {
        self.cell_width
    }

    pub fn get_cell_height(&self) -> usize {
        self.cell_height
    }

    pub fn show_level_select(&self, selected: usize, levels: &[Level]) {
    let mut stdout = stdout();
    stdout.execute(Clear(ClearType::All)).unwrap();

    let (cols, rows) = size().unwrap();
    let center_x = cols / 2;
    let start_y = rows / 4;

    stdout.execute(MoveTo(center_x.saturating_sub(12), start_y)).unwrap();
    print!("{}", "🎯 SELECCIÓN DE NIVEL 🎯".with(Color::Cyan).bold());

    stdout.execute(MoveTo(center_x.saturating_sub(10), start_y + 2)).unwrap();
    print!("{}", "═══════════════════════".with(Color::DarkCyan));

    for (i, level) in levels.iter().enumerate() {
        let y_pos = start_y + 4 + (i as u16 * 2);
        stdout.execute(MoveTo(center_x.saturating_sub(20), y_pos)).unwrap();
        
        if i == selected {
            print!("{}", format!("→ {} - {} items requeridos", 
                level.name, level.required_items).with(Color::Yellow).bold());
        } else {
            print!("{}", format!("  {} - {} items requeridos", 
                level.name, level.required_items).with(Color::White));
        }
    }

    stdout.execute(MoveTo(center_x.saturating_sub(18), start_y + 12)).unwrap();
    print!("{}", "↑/↓ o W/S: Seleccionar nivel".with(Color::DarkGrey));

    stdout.execute(MoveTo(center_x.saturating_sub(18), start_y + 13)).unwrap();
    print!("{}", "1/2/3: Ir a nivel directamente".with(Color::DarkGrey));

    stdout.execute(MoveTo(center_x.saturating_sub(15), start_y + 15)).unwrap();
    print!("{}", "Presiona ENTER para jugar".with(Color::Green).bold());

    stdout.execute(MoveTo(center_x.saturating_sub(10), start_y + 17)).unwrap();
    print!("{}", "ESC: Volver al menú".with(Color::DarkGrey));

    stdout.flush().unwrap();
}
}