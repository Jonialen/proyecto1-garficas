use crossterm::{
    ExecutableCommand,
    cursor::MoveTo,
    queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor, Stylize},
    terminal::{Clear, ClearType, size},
};
use std::collections::{HashSet};
use std::io::{BufWriter, Write, stdout};
use std::time::Instant;

use crate::game::{Entity, Player, Level};
use crate::graphics::{Framebuffer, Pixel};
use crate::math::Vec2;

/// Color de la niebla; también es el color del cielo en el horizonte.
const FOG_COLOR: [f32; 3] = [28.0, 30.0, 52.0];
/// Color del cielo en la parte superior de la pantalla.
const SKY_TOP: [f32; 3] = [6.0, 4.0, 28.0];
/// Color base de las paredes a distancia cero.
const WALL_COLOR: [f32; 3] = [210.0, 182.0, 148.0];
/// Color base del suelo a distancia cero.
const FLOOR_COLOR: [f32; 3] = [72.0, 92.0, 62.0];
/// Densidad de la niebla exponencial; valores mayores acortan la visibilidad.
const FOG_DENSITY: f32 = 0.18;
/// Multiplicador de luz para las caras norte/sur de las paredes.
const SIDE_SHADE: f32 = 0.65;

/// Píxeles del búfer de doble resolución por cada celda del minimapa.
const MINIMAP_SCALE: usize = 3;
/// Opacidad del minimapa sobre la escena (0 = invisible, 1 = opaco).
const MINIMAP_OPACITY: f32 = 0.82;
const MINIMAP_BORDER: [f32; 3] = [230.0, 190.0, 90.0];
const MINIMAP_OUTSIDE: [f32; 3] = [4.0, 4.0, 10.0];
const MINIMAP_FLOOR: [f32; 3] = [16.0, 18.0, 30.0];
const MINIMAP_WALL: [f32; 3] = [170.0, 150.0, 125.0];
const MINIMAP_START: [f32; 3] = [60.0, 170.0, 90.0];
const MINIMAP_GOAL: [f32; 3] = [220.0, 60.0, 60.0];
const MINIMAP_ITEM: [f32; 3] = [255.0, 215.0, 40.0];
const MINIMAP_PLAYER: [f32; 3] = [80.0, 230.0, 255.0];
const MINIMAP_DIRECTION: [f32; 3] = [255.0, 90.0, 90.0];

/// Mitad de la altura de un ítem (diamante), en unidades del mundo.
const GEM_HALF_HEIGHT: f32 = 0.2;
/// Relación ancho/alto en pantalla del diamante.
const GEM_ASPECT: f32 = 0.7;
const GEM_COLOR: [f32; 3] = [255.0, 196.0, 40.0];
/// Radio de la esfera de la meta, en unidades del mundo.
const ORB_RADIUS: f32 = 0.2;
const ORB_COLOR: [f32; 3] = [235.0, 45.0, 55.0];
/// Dirección (normalizada) hacia la luz en espacio de cámara: x derecha, y arriba, z hacia adentro.
const LIGHT_DIR: [f32; 3] = [-0.48, 0.67, -0.57];
const BRICK_MORTAR: [f32; 3] = [78.0, 72.0, 68.0];
/// Filas de ladrillos por unidad de altura de pared.
const BRICK_ROWS: f32 = 4.0;
/// Ladrillos por unidad de ancho de pared.
const BRICK_COLS: f32 = 2.0;

/// Resultado de lanzar un rayo contra el mapa.
struct RayHit {
    /// Distancia perpendicular a la pared.
    distance: f32,
    /// Lado golpeado: 0 = cara este/oeste, 1 = cara norte/sur.
    side: u8,
    /// Posición del impacto a lo largo de la pared, en [0, 1).
    wall_x: f32,
    /// Celda del mapa golpeada (puede estar fuera del mapa en los bordes).
    cell: (i32, i32),
}

/// Hash determinista de dos enteros a un valor en [0, 1).
fn hash2(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841);
    h ^= h >> 13;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 16;
    (h & 0x00ff_ffff) as f32 / 16_777_216.0
}

/// Devuelve el color de la textura de ladrillos en la coordenada (u, v) de una pared.
/// `line_height` es la altura de la pared en píxeles: si es pequeña se omite el mortero para evitar aliasing.
fn brick_texel(u: f32, v: f32, cell: (i32, i32), line_height: f32) -> [f32; 3] {
    let row_pos = v * BRICK_ROWS;
    let row = row_pos.floor() as i32;
    let offset = if row % 2 == 0 { 0.0 } else { 0.5 };
    let col_pos = u * BRICK_COLS + offset;
    let col = col_pos.floor() as i32;

    let mortar_v = 0.09 * (line_height / 24.0).min(1.0);
    let mortar_u = 0.05 * (line_height / 24.0).min(1.0);
    if row_pos.fract() < mortar_v || col_pos.fract() < mortar_u {
        return BRICK_MORTAR;
    }

    // Variación por ladrillo y un grano fino estable en espacio de textura.
    let brick = hash2(cell.0 * 31 + col, cell.1 * 17 + row);
    let grain = hash2((u * 24.0) as i32 + cell.0 * 97, (v * 24.0) as i32 + cell.1 * 89);
    let tone = 0.78 + 0.3 * brick + 0.08 * (grain - 0.5);
    [WALL_COLOR[0] * tone, WALL_COLOR[1] * tone * 0.97, WALL_COLOR[2] * tone * 0.94]
}

/// Sombrea un diamante (octaedro) que gira sobre su eje vertical.
/// `u` y `v` están normalizados a [-1, 1]; devuelve `None` si el píxel no toca el objeto.
fn shade_gem(u: f32, v: f32, spin: f32) -> Option<[f32; 3]> {
    let r = 1.0 - v.abs();
    if r <= 0.0 { return None; }

    // La sección horizontal es un cuadrado rotado; se busca la cara frontal que intersecta
    // el rayo de vista (paralelo a z) en x = u.
    let c = r / std::f32::consts::SQRT_2;
    let mut entry = f32::MIN;
    let mut exit = f32::MAX;
    let mut normal = (0.0, 0.0);
    for k in 0..4 {
        let phi = spin + std::f32::consts::FRAC_PI_4 + k as f32 * std::f32::consts::FRAC_PI_2;
        let (nx, nz) = (phi.cos(), phi.sin());
        if nz.abs() < 1e-4 { continue; }
        let t = (c - nx * u) / nz;
        if nz < 0.0 {
            if t > entry { entry = t; normal = (nx, nz); }
        } else if t < exit {
            exit = t;
        }
    }
    if entry > exit { return None; }

    let h = (2.0f32 / 3.0).sqrt();
    let n = [normal.0 * h, v.signum() / 3.0f32.sqrt(), normal.1 * h];
    Some(lit_surface(GEM_COLOR, n, 0.5, 48.0))
}

/// Sombrea la esfera de la meta, con un brillo que pulsa con el tiempo.
fn shade_orb(u: f32, v: f32, time: f32) -> Option<[f32; 3]> {
    let d2 = u * u + v * v;
    if d2 > 1.0 { return None; }
    let n = [u, v, -(1.0 - d2).sqrt()];
    let pulse = 0.25 + 0.2 * (time * 3.0).sin();
    Some(lit_surface(ORB_COLOR, n, pulse, 24.0))
}

/// Iluminación difusa + especular (Blinn-Phong) con la luz fija `LIGHT_DIR`.
fn lit_surface(base: [f32; 3], n: [f32; 3], ambient: f32, shininess: f32) -> [f32; 3] {
    let diffuse = (n[0] * LIGHT_DIR[0] + n[1] * LIGHT_DIR[1] + n[2] * LIGHT_DIR[2]).max(0.0);
    // Vector medio entre la luz y la cámara (que mira hacia -z).
    let half = [LIGHT_DIR[0], LIGHT_DIR[1], LIGHT_DIR[2] - 1.0];
    let half_len = (half[0] * half[0] + half[1] * half[1] + half[2] * half[2]).sqrt();
    let spec = ((n[0] * half[0] + n[1] * half[1] + n[2] * half[2]) / half_len).max(0.0).powf(shininess);
    let k = ambient + 0.85 * diffuse;
    [
        base[0] * k + 255.0 * spec,
        base[1] * k + 255.0 * spec,
        base[2] * k + 255.0 * spec,
    ]
}

/// Interpola linealmente entre dos colores RGB.
fn lerp_color(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// Mezcla un color con la niebla según la distancia (niebla exponencial).
fn apply_fog(color: [f32; 3], distance: f32) -> [f32; 3] {
    lerp_color(color, FOG_COLOR, 1.0 - (-FOG_DENSITY * distance).exp())
}

/// Convierte un color de punto flotante a un color de 24 bits de la terminal.
fn to_rgb(color: [f32; 3]) -> Color {
    Color::Rgb {
        r: color[0].clamp(0.0, 255.0) as u8,
        g: color[1].clamp(0.0, 255.0) as u8,
        b: color[2].clamp(0.0, 255.0) as u8,
    }
}

/// Gestiona todo el renderizado del juego, incluyendo el mapa, los menús y la interfaz de usuario.
pub struct GameRenderer {
    cell_width: usize, // Ancho de una celda del mapa en caracteres.
    cell_height: usize, // Altura de una celda del mapa en caracteres.
    offset_x: u16, // Desplazamiento horizontal para centrar el mapa.
    offset_y: u16, // Desplazamiento vertical para centrar el mapa.
    minimap_size: usize, // Tamaño del minimapa en la vista de primera persona.
    collected_positions: HashSet<(usize, usize)>, // Almacena las posiciones de los ítems recogidos.
    start_time: Instant, // Referencia de tiempo para las animaciones.
}

impl GameRenderer {
    /// Crea un nuevo renderizador, calculando las dimensiones óptimas para el mapa en la terminal.
    pub fn new(
        terminal_cols: usize,
        terminal_rows: usize,
        map_width: usize,
        map_height: usize,
    ) -> Self {
        let rows = terminal_rows.saturating_sub(4);
        let aspect_fix = 2.0; // Factor de corrección para el aspect ratio de los caracteres.

        // Calcula el tamaño de celda que mejor se ajusta a la terminal.
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

        // Calcula los desplazamientos para centrar el mapa.
        let offset_x = ((terminal_cols as isize - map_pixel_width as isize) / 2).max(0) as u16;
        let offset_y = ((rows as isize - map_pixel_height as isize) / 2).max(0) as u16;

        let minimap_size = 14.min(terminal_cols / 6).min(terminal_rows / 6).max(10);

        Self {
            cell_width,
            cell_height,
            offset_x,
            offset_y,
            minimap_size,
            collected_positions: HashSet::new(),
            start_time: Instant::now(),
        }
    }

    /// Muestra el menú principal del juego.
    pub fn show_menu(&self) {
        let mut stdout = stdout();
        stdout.execute(Clear(ClearType::All)).unwrap();

        let (cols, rows) = size().unwrap();
        let center_x = cols / 2;
        let start_y = rows / 4;

        // Título, controles e instrucciones.
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

    /// Muestra la pantalla de victoria cuando se completan todos los niveles.
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

    /// Renderiza la vista cenital (Top-Down) del juego.
    pub fn render_top_down(
        &mut self,
        framebuffer: &mut Framebuffer,
        map: &[Vec<u8>],
        player: &Player,
        _entities: &[Entity],
    ) {
        framebuffer.clear(Color::Black);

        let (px, py) = player.get_grid_position();
        if map[py][px] == 5 { // Si el jugador está sobre un ítem, lo marca como recogido.
            self.collected_positions.insert((px, py));
        }

        let map_width = map[0].len();
        let map_height = map.len();
        
        // Dibuja el borde del mapa.
        for x in 0..map_width * self.cell_width {
            framebuffer.set_pixel(x, 0, Pixel::new(Color::DarkCyan, '═'));
        }
        for x in 0..map_width * self.cell_width {
            let y = map_height * self.cell_height - 1;
            if y < framebuffer.height {
                framebuffer.set_pixel(x, y, Pixel::new(Color::DarkCyan, '═'));
            }
        }
        for y in 0..map_height * self.cell_height {
            if y < framebuffer.height {
                framebuffer.set_pixel(0, y, Pixel::new(Color::DarkCyan, '║'));
                let x = map_width * self.cell_width - 1;
                if x < framebuffer.width {
                    framebuffer.set_pixel(x, y, Pixel::new(Color::DarkCyan, '║'));
                }
            }
        }
        framebuffer.set_pixel(0, 0, Pixel::new(Color::DarkCyan, '╔'));
        framebuffer.set_pixel(map_width * self.cell_width - 1, 0, Pixel::new(Color::DarkCyan, '╗'));
        let bottom_left_y = map_height * self.cell_height - 1;
        if bottom_left_y < framebuffer.height {
            framebuffer.set_pixel(0, bottom_left_y, Pixel::new(Color::DarkCyan, '╚'));
            framebuffer.set_pixel(map_width * self.cell_width - 1, bottom_left_y, Pixel::new(Color::DarkCyan, '╝'));
        }

        // Renderiza el contenido del mapa.
        for (row_idx, fila) in map.iter().enumerate() {
            for (col_idx, &celda) in fila.iter().enumerate() {
                let actual_cell = if celda == 5 && self.collected_positions.contains(&(col_idx, row_idx)) {
                    0 // Si el ítem fue recogido, se muestra como espacio vacío.
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

                // Añade textura a las paredes para un efecto visual.
                let enhanced_pixel = if actual_cell == 1 {
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
                        
                        if fb_x > 0 && fb_x < framebuffer.width - 1 && fb_y > 0 && fb_y < framebuffer.height - 1 {
                            framebuffer.set_pixel(fb_x, fb_y, enhanced_pixel);
                        }
                    }
                }
            }
        }

        // Renderiza al jugador y su dirección.
        let player_center_x = px * self.cell_width + self.cell_width / 2;
        let player_center_y = py * self.cell_height + self.cell_height / 2;
        
        for sub_row in 0..self.cell_height {
            for sub_col in 0..self.cell_width {
                let fb_x = px * self.cell_width + sub_col;
                let fb_y = py * self.cell_height + sub_row;
                if fb_x > 0 && fb_x < framebuffer.width - 1 && fb_y > 0 && fb_y < framebuffer.height - 1 {
                    framebuffer.set_pixel(fb_x, fb_y, Pixel::new(Color::Cyan, '●'));
                }
            }
        }
        
        let dir_length = (self.cell_width.max(self.cell_height) as f32 * 0.8) as usize;
        for i in 1..=dir_length {
            let dir_x = player_center_x as f32 + player.direction.x * i as f32;
            let dir_y = player_center_y as f32 + player.direction.y * i as f32;
            let dx = dir_x as usize;
            let dy = dir_y as usize;
            
            if dx > 0 && dx < framebuffer.width - 1 && dy > 0 && dy < framebuffer.height - 1 {
                let symbol = if i == dir_length { '▶' } else { '─' };
                framebuffer.set_pixel(dx, dy, Pixel::new(Color::Yellow, symbol));
            }
        }

        self.render_vision_rays(framebuffer, player, map);
    }

    /// Renderiza la vista en primera persona (First-Person) del juego.
    pub fn render_first_person(
        &mut self,
        framebuffer: &mut Framebuffer,
        player: &Player,
        map: &[Vec<u8>],
    ) {
        let (px, py) = player.get_grid_position();
        if map[py][px] == 5 {
            self.collected_positions.insert((px, py));
        }

        // Se renderiza a doble resolución vertical: cada celda de la terminal muestra
        // dos píxeles usando '▀' (mitad superior = primer plano, mitad inferior = fondo).
        let width = framebuffer.width;
        let hi_height = framebuffer.height * 2;
        let horizon = hi_height as f32 / 2.0;
        let mut hi_res = vec![[0.0f32; 3]; width * hi_height];

        // Cielo y suelo: el cielo se funde con la niebla en el horizonte y el suelo
        // se oscurece con la misma niebla según la distancia de cada fila.
        for y in 0..hi_height {
            let row_center = y as f32 + 0.5;
            let color = if row_center < horizon {
                let t = row_center / horizon;
                lerp_color(SKY_TOP, FOG_COLOR, t * t)
            } else {
                let row_distance = hi_height as f32 / (2.0 * (row_center - horizon));
                apply_fog(FLOOR_COLOR, row_distance)
            };
            hi_res[y * width..(y + 1) * width].fill(color);
        }

        // Lanzamiento de rayos (Raycasting) para renderizar las paredes.
        let ray_angle_step = player.fov / width as f32;
        let start_angle = -player.fov / 2.0;
        let mut z_buffer = vec![f32::MAX; width];

        for i in 0..width {
            let ray_angle = start_angle + i as f32 * ray_angle_step;
            let ray_dir = player.direction.rotate(ray_angle);

            let hit = self.cast_ray_with_type(player.position, ray_dir, map);
            let corrected_distance = (hit.distance * ray_angle.cos()).max(0.0001); // Corrige la distorsión de ojo de pez.
            z_buffer[i] = corrected_distance;

            let line_height = hi_height as f32 / corrected_distance;
            let wall_top = horizon - line_height / 2.0;
            let draw_start = wall_top.max(0.0) as usize;
            let draw_end = ((horizon + line_height / 2.0) as usize).min(hi_height);

            // Las caras norte/sur se oscurecen para dar sensación de volumen.
            let light = if hit.side == 1 { SIDE_SHADE } else { 1.0 };

            for y in draw_start..draw_end {
                let v = (y as f32 + 0.5 - wall_top) / line_height;
                let texel = brick_texel(hit.wall_x, v, hit.cell, line_height);
                let lit = [texel[0] * light, texel[1] * light, texel[2] * light];
                hi_res[y * width + i] = apply_fog(lit, corrected_distance);
            }
        }

        self.render_sprites(&mut hi_res, &z_buffer, width, hi_height, player, map);

        self.render_minimap(&mut hi_res, width, hi_height, player, map);

        // Combina cada par de píxeles verticales en una sola celda de la terminal.
        for y in 0..framebuffer.height {
            for x in 0..width {
                let top = to_rgb(hi_res[(2 * y) * width + x]);
                let bottom = to_rgb(hi_res[(2 * y + 1) * width + x]);
                framebuffer.set_pixel(x, y, Pixel::with_bg(top, bottom, '▀'));
            }
        }
    }

    /// Renderiza los objetos del mapa (ítems y meta) como sprites 3D, de atrás hacia adelante,
    /// respetando el z-buffer de las paredes.
    fn render_sprites(&self, buf: &mut [[f32; 3]], z_buffer: &[f32], width: usize, height: usize, player: &Player, map: &[Vec<u8>]) {
        let time = self.start_time.elapsed().as_secs_f32();
        let dir = player.direction.normalize();
        let horizon = height as f32 / 2.0;
        let angle_step = player.fov / width as f32;

        let mut sprites: Vec<(f32, f32, usize, usize, u8)> = Vec::new();
        for (cy, row) in map.iter().enumerate() {
            for (cx, &cell) in row.iter().enumerate() {
                if cell != 5 && cell != 3 { continue; }
                let rel = Vec2::new(cx as f32 + 0.5 - player.position.x, cy as f32 + 0.5 - player.position.y);
                let depth = dir.x * rel.x + dir.y * rel.y;
                if depth < 0.2 { continue; }
                let angle = (dir.x * rel.y - dir.y * rel.x).atan2(depth);
                sprites.push((depth, angle, cx, cy, cell));
            }
        }
        sprites.sort_by(|a, b| b.0.total_cmp(&a.0));

        for (depth, angle, cx, cy, cell) in sprites {
            let center_col = (angle + player.fov / 2.0) / angle_step;
            let phase = hash2(cx as i32, cy as i32) * std::f32::consts::TAU;
            let scale = height as f32 / depth;
            let fog = 1.0 - (-FOG_DENSITY * depth).exp();

            // Altura sobre el suelo (0..1), radio vertical y proporción horizontal en pantalla.
            let (elevation, half_h, aspect) = if cell == 5 {
                (0.42 + 0.05 * (time * 2.2 + phase).sin(), GEM_HALF_HEIGHT * scale, GEM_ASPECT)
            } else {
                (0.45 + 0.03 * (time * 1.3).sin(), ORB_RADIUS * scale, 1.0)
            };
            let half_w = half_h * aspect;
            let center_y = horizon + (0.5 - elevation) * scale;

            let col_start = (center_col - half_w).floor().max(0.0) as usize;
            let col_end = ((center_col + half_w).ceil() as usize).min(width);

            // Sombra elíptica en el suelo, bajo el objeto.
            let floor_y = horizon + 0.5 * scale;
            let shadow_h = (half_w * 0.25).max(1.0);
            let row_start = (floor_y - shadow_h).max(0.0) as usize;
            let row_end = ((floor_y + shadow_h) as usize).min(height);
            for x in col_start..col_end {
                if depth >= z_buffer[x] { continue; }
                let u = (x as f32 + 0.5 - center_col) / half_w;
                for y in row_start..row_end {
                    let v = (y as f32 + 0.5 - floor_y) / shadow_h;
                    let d2 = u * u + v * v;
                    if d2 < 1.0 {
                        let idx = y * width + x;
                        buf[idx] = lerp_color(buf[idx], [0.0, 0.0, 0.0], 0.55 * (1.0 - d2) * (1.0 - fog));
                    }
                }
            }

            let row_start = (center_y - half_h).max(0.0) as usize;
            let row_end = ((center_y + half_h).ceil() as usize).min(height);
            for x in col_start..col_end {
                if depth >= z_buffer[x] { continue; }
                let u = (x as f32 + 0.5 - center_col) / half_w;
                for y in row_start..row_end {
                    let v = (center_y - (y as f32 + 0.5)) / half_h;
                    let shaded = if cell == 5 {
                        shade_gem(u, v, time * 1.6 + phase)
                    } else {
                        shade_orb(u, v, time)
                    };
                    if let Some(color) = shaded {
                        buf[y * width + x] = apply_fog(color, depth);
                    }
                }
            }
        }
    }

    /// Renderiza el minimapa sobre el búfer de doble resolución, centrado en el jugador.
    /// Cada celda del mapa ocupa `MINIMAP_SCALE`×`MINIMAP_SCALE` píxeles cuadrados.
    fn render_minimap(&self, buf: &mut [[f32; 3]], width: usize, height: usize, player: &Player, map: &[Vec<u8>]) {
        let scale = MINIMAP_SCALE;
        let inner = self.minimap_size * scale;
        let total = inner + 2;

        if width < total + 4 || height < total + 4 { return; }

        let origin_x = width - total - 2;
        let origin_y = 2;
        let half_view = self.minimap_size as f32 / 2.0;
        let map_height = map.len();
        let map_width = map.first().map_or(0, |row| row.len());

        for py in 0..total {
            for px in 0..total {
                let idx = (origin_y + py) * width + origin_x + px;

                if px == 0 || py == 0 || px == total - 1 || py == total - 1 {
                    buf[idx] = MINIMAP_BORDER;
                    continue;
                }

                // Coordenadas del mundo en el centro de este píxel.
                let wx = player.position.x - half_view + ((px - 1) as f32 + 0.5) / scale as f32;
                let wy = player.position.y - half_view + ((py - 1) as f32 + 0.5) / scale as f32;

                let color = if wx < 0.0 || wy < 0.0 || wx as usize >= map_width || wy as usize >= map_height {
                    MINIMAP_OUTSIDE
                } else {
                    let (cx, cy) = (wx as usize, wy as usize);
                    match map[cy][cx] {
                        1 => MINIMAP_WALL,
                        2 => MINIMAP_START,
                        3 => MINIMAP_GOAL,
                        // Los ítems se dibujan como un punto en el centro de su celda.
                        5 if !self.collected_positions.contains(&(cx, cy))
                            && (wx.fract() - 0.5).abs() < 0.25
                            && (wy.fract() - 0.5).abs() < 0.25 => MINIMAP_ITEM,
                        _ => MINIMAP_FLOOR,
                    }
                };

                buf[idx] = lerp_color(buf[idx], color, MINIMAP_OPACITY);
            }
        }

        // Jugador en el centro y una línea que indica hacia dónde mira.
        let center_x = (origin_x + 1 + inner / 2) as f32;
        let center_y = (origin_y + 1 + inner / 2) as f32;
        let mut plot = |x: f32, y: f32, color: [f32; 3]| {
            let (x, y) = (x as usize, y as usize);
            if x > origin_x && x < origin_x + total - 1 && y > origin_y && y < origin_y + total - 1 {
                buf[y * width + x] = color;
            }
        };

        for step in 1..=scale * 2 {
            plot(center_x + player.direction.x * step as f32, center_y + player.direction.y * step as f32, MINIMAP_DIRECTION);
        }
        for (dx, dy) in [(-1.0, -1.0), (0.0, -1.0), (-1.0, 0.0), (0.0, 0.0)] {
            plot(center_x + dx, center_y + dy, MINIMAP_PLAYER);
        }
    }

    /// Renderiza los rayos de visión del jugador en la vista cenital.
    fn render_vision_rays(&self, framebuffer: &mut Framebuffer, player: &Player, map: &[Vec<u8>]) {
        let center_x = (player.position.x * self.cell_width as f32) as usize;
        let center_y = (player.position.y * self.cell_height as f32) as usize;

        for i in 0..5 {
            let angle_offset = (i as f32 - 2.0) * (player.fov / 4.0);
            let ray_dir = player.direction.rotate(angle_offset);

            let distance = self.cast_ray_with_type(player.position, ray_dir, map).distance;
            let end_x = player.position.x + ray_dir.x * distance;
            let end_y = player.position.y + ray_dir.y * distance;

            let screen_end_x = (end_x * self.cell_width as f32) as usize;
            let screen_end_y = (end_y * self.cell_height as f32) as usize;

            let safe_end_x = screen_end_x.min(framebuffer.width - 1);
            let safe_end_y = screen_end_y.min(framebuffer.height - 1);

            framebuffer.draw_line(center_x, center_y, safe_end_x, safe_end_y, Pixel::new(Color::DarkYellow, '·'));
        }
    }

    /// Lanza un rayo desde una posición y en una dirección, y devuelve la información del primer impacto.
    fn cast_ray_with_type(&self, start: Vec2, direction: Vec2, map: &[Vec<u8>]) -> RayHit {
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

            if map_x < 0 || map_y < 0 || map_y as usize >= map.len() || map_x as usize >= map[0].len() {
                hit = true;
            } else {
                let cell = map[map_y as usize][map_x as usize];
                if cell == 1 {
                    hit = true;
                }
            }
        }

        let perp_wall_dist = if side == 0 {
            (map_x as f32 - start.x + (1 - step_x) as f32 / 2.0) / direction.x
        } else {
            (map_y as f32 - start.y + (1 - step_y) as f32 / 2.0) / direction.y
        };

        // Punto exacto del impacto a lo largo de la pared, usado como coordenada de textura.
        let wall_x = if side == 0 {
            start.y + perp_wall_dist * direction.y
        } else {
            start.x + perp_wall_dist * direction.x
        };

        RayHit {
            distance: perp_wall_dist.abs(),
            side,
            wall_x: wall_x - wall_x.floor(),
            cell: (map_x, map_y),
        }
    }

    /// Muestra el contenido del framebuffer en la terminal.
    pub fn display_framebuffer(&self, framebuffer: &Framebuffer) {
        // Un búfer grande evita miles de escrituras pequeñas a la terminal por cuadro.
        let mut stdout = BufWriter::with_capacity(1 << 16, stdout().lock());

        let use_offset = framebuffer.width < size().unwrap().0 as usize;

        // Solo se emiten secuencias de color cuando cambian, para no saturar la terminal.
        let mut current: Option<(Color, Color)> = None;

        for (y, row) in framebuffer.pixels.iter().enumerate() {
            let x_pos = if use_offset { self.offset_x } else { 0 };
            let y_pos = if use_offset { self.offset_y + y as u16 } else { y as u16 };

            queue!(stdout, MoveTo(x_pos, y_pos)).unwrap();

            for pixel in row {
                if current != Some((pixel.color, pixel.bg)) {
                    queue!(stdout, SetForegroundColor(pixel.color), SetBackgroundColor(pixel.bg)).unwrap();
                    current = Some((pixel.color, pixel.bg));
                }
                queue!(stdout, Print(pixel.symbol)).unwrap();
            }
        }
        queue!(stdout, ResetColor).unwrap();
        stdout.flush().unwrap();
    }

    /// Muestra la interfaz de usuario (UI) con información del juego.
    pub fn display_ui(
        &self,
        fps: f32,
        px: f32,
        py: f32,
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

    /// Devuelve el ancho de celda calculado.
    pub fn get_cell_width(&self) -> usize {
        self.cell_width
    }

    /// Devuelve la altura de celda calculada.
    pub fn get_cell_height(&self) -> usize {
        self.cell_height
    }

    /// Muestra la pantalla de selección de nivel.
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
                print!("{}", format!("→ {} - {} items requeridos", level.name, level.required_items).with(Color::Yellow).bold());
            } else {
                print!("{}", format!("  {} - {} items requeridos", level.name, level.required_items).with(Color::White));
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
