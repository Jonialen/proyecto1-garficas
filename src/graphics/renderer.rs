use crossterm::{
    ExecutableCommand,
    cursor::MoveTo,
    style::{Color, Stylize},
    terminal::{Clear, ClearType, size},
};
use std::collections::HashMap;
use std::io::{Write, stdout};

use crate::game::{CameraMode, Entity, Player};
use crate::graphics::{Framebuffer, Pixel};
use crate::math::Vec2;

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
        map_symbols.insert(0, Pixel::new(Color::Black, '█'));
        map_symbols.insert(1, Pixel::new(Color::White, '█'));
        map_symbols.insert(2, Pixel::new(Color::Green, '█'));
        map_symbols.insert(3, Pixel::new(Color::Red, '█'));
        map_symbols.insert(4, Pixel::new(Color::Blue, '█'));

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

        stdout
            .execute(MoveTo(center_x.saturating_sub(10), start_y))
            .unwrap();
        print!("{}", "🎮 RAYTRACER MAZE 🎮".with(Color::Cyan));

        stdout
            .execute(MoveTo(center_x.saturating_sub(8), start_y + 2))
            .unwrap();
        print!("{}", "═══════════════════".with(Color::White));

        stdout
            .execute(MoveTo(center_x.saturating_sub(12), start_y + 4))
            .unwrap();
        print!("{}", "⌨️  CONTROLES:".with(Color::Yellow));

        stdout
            .execute(MoveTo(center_x.saturating_sub(15), start_y + 6))
            .unwrap();
        print!("{}", "WASD o ↑↓←→  - Mover jugador".with(Color::White));

        stdout
            .execute(MoveTo(center_x.saturating_sub(15), start_y + 7))
            .unwrap();
        print!("{}", "Q / E        - Rotar cámara".with(Color::White));

        stdout
            .execute(MoveTo(center_x.saturating_sub(15), start_y + 8))
            .unwrap();
        print!("{}", "C            - Cambiar vista".with(Color::White));

        stdout
            .execute(MoveTo(center_x.saturating_sub(15), start_y + 9))
            .unwrap();
        print!("{}", "X / ESC      - Salir".with(Color::White));

        stdout
            .execute(MoveTo(center_x.saturating_sub(12), start_y + 11))
            .unwrap();
        print!("{}", "🎯 OBJETIVO:".with(Color::Yellow));

        stdout
            .execute(MoveTo(center_x.saturating_sub(20), start_y + 12))
            .unwrap();
        print!(
            "{} {} {}",
            "Llega desde el inicio {} hasta la meta {}".with(Color::White),
            "🟩".with(Color::Green),
            "🟥".with(Color::Red)
        );

        stdout
            .execute(MoveTo(center_x.saturating_sub(12), start_y + 15))
            .unwrap();
        print!("{}", "Presiona ENTER para jugar".with(Color::Green));

        stdout
            .execute(MoveTo(center_x.saturating_sub(8), start_y + 16))
            .unwrap();
        print!("{}", "═══════════════════".with(Color::White));

        stdout.flush().unwrap();
    }

    pub fn show_victory(&self) {
        let mut stdout = stdout();
        stdout.execute(Clear(ClearType::All)).unwrap();

        let (cols, rows) = size().unwrap();
        let center_x = cols / 2;
        let center_y = rows / 2;

        stdout
            .execute(MoveTo(
                center_x.saturating_sub(8),
                center_y.saturating_sub(3),
            ))
            .unwrap();
        print!("{}", "🎉 ¡VICTORIA! 🎉".with(Color::Green));

        stdout
            .execute(MoveTo(
                center_x.saturating_sub(12),
                center_y.saturating_sub(1),
            ))
            .unwrap();
        print!("{}", "¡Has llegado a la meta!".with(Color::Yellow));

        stdout
            .execute(MoveTo(center_x.saturating_sub(15), center_y + 1))
            .unwrap();
        print!(
            "{}",
            "Presiona cualquier tecla para salir".with(Color::White)
        );

        stdout.flush().unwrap();
    }

    pub fn render_top_down(
        &self,
        framebuffer: &mut Framebuffer,
        map: &[Vec<u8>],
        player: &Player,
        entities: &[Entity],
    ) {
        framebuffer.clear(Color::Black);

        // Renderizar mapa
        for (row_idx, fila) in map.iter().enumerate() {
            for (col_idx, &celda) in fila.iter().enumerate() {
                let pixel = *self
                    .map_symbols
                    .get(&celda)
                    .unwrap_or(&Pixel::new(Color::Red, '?'));

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

        self.render_vision_rays(framebuffer, player, map);
    }

    pub fn render_first_person(
        &self,
        framebuffer: &mut Framebuffer,
        player: &Player,
        map: &[Vec<u8>],
    ) {
        // Cielo y suelo
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

        self.render_minimap(framebuffer, player, map);
    }

    fn render_minimap(&self, framebuffer: &mut Framebuffer, player: &Player, map: &[Vec<u8>]) {
        let total_minimap_width = self.minimap_size + 2;
        let total_minimap_height = self.minimap_size + 2;

        if framebuffer.width < total_minimap_width + 2
            || framebuffer.height < total_minimap_height + 4
        {
            return;
        }

        let minimap_x = framebuffer.width - total_minimap_width - 1;
        let minimap_y = 1;

        // Marco del minimapa
        for i in 0..total_minimap_width {
            for j in 0..total_minimap_height {
                let x = minimap_x + i;
                let y = minimap_y + j;

                if x < framebuffer.width && y < framebuffer.height {
                    if i == 0
                        || i == total_minimap_width - 1
                        || j == 0
                        || j == total_minimap_height - 1
                    {
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
                    Pixel::new(Color::Cyan, '█')
                } else if world_y < map.len() && world_x < map[0].len() {
                    match map[world_y][world_x] {
                        0 => Pixel::new(Color::DarkGrey, '█'),
                        1 => Pixel::new(Color::White, '█'),
                        2 => Pixel::new(Color::Green, '█'),
                        3 => Pixel::new(Color::Red, '█'),
                        4 => Pixel::new(Color::Blue, '█'),
                        _ => Pixel::new(Color::Black, '█'),
                    }
                } else {
                    Pixel::new(Color::Black, '█')
                };

                framebuffer.set_pixel(screen_x, screen_y, pixel);
            }
        }

        // Indicador de dirección del jugador
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
                center_x,
                center_y,
                safe_end_x,
                safe_end_y,
                Pixel::new(Color::Yellow, '█'),
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

            if map_x < 0
                || map_y < 0
                || map_y as usize >= map.len()
                || map_x as usize >= map[0].len()
                || map[map_y as usize][map_x as usize] == 1
            {
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

        let use_offset = framebuffer.width < size().unwrap().0 as usize;

        for (y, row) in framebuffer.pixels.iter().enumerate() {
            let x_pos = if use_offset { self.offset_x } else { 0 };
            let y_pos = if use_offset {
                self.offset_y + y as u16
            } else {
                y as u16
            };

            stdout.execute(MoveTo(x_pos, y_pos)).unwrap();

            for pixel in row {
                let styled = pixel.symbol.to_string().with(pixel.color);
                print!("{}", styled);
            }
        }
        stdout.flush().unwrap();
    }

    pub fn get_cell_width(&self) -> usize {
        self.cell_width
    }

    pub fn get_cell_height(&self) -> usize {
        self.cell_height
    }
}
