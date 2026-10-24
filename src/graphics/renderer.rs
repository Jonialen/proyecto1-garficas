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

/// Duración del destello al recoger un ítem, en segundos.
const FLASH_DURATION: f32 = 0.45;
/// Opacidad máxima del destello en los bordes de la pantalla.
const FLASH_STRENGTH: f32 = 0.6;
const FLASH_COLOR: [f32; 3] = [255.0, 210.0, 70.0];
/// Amplitud del balanceo de cámara, como fracción de la altura de la pantalla.
const BOB_AMPLITUDE: f32 = 0.018;
/// Radianes de balanceo por unidad caminada (un ciclo cada ~0.9 unidades).
const BOB_FREQUENCY: f32 = 7.0;
/// Segundos que tarda la cámara en estabilizarse al dejar de caminar.
const BOB_SETTLE_TIME: f32 = 0.3;

/// Segundos en negro con el título antes de que aparezca la escena.
const INTRO_HOLD: f32 = 0.9;
/// Segundos que tarda la escena en aparecer tras el título.
const INTRO_FADE: f32 = 0.8;
/// Segundos del fundido a negro al completar un nivel.
const FADE_OUT_DURATION: f32 = 0.6;
/// Ancho de la junta entre baldosas, en unidades del mundo.
const FLOOR_GROUT: f32 = 0.04;
const FLOOR_GROUT_COLOR: [f32; 3] = [30.0, 38.0, 30.0];
/// Filas de texto del menú debajo del título.
const MENU_TEXT_ROWS: usize = 17;
/// Filas de texto de la pantalla de victoria debajo del título.
const VICTORY_TEXT_ROWS: usize = 6;
/// Cantidad de destellos animados en la pantalla de victoria.
const SPARKLE_COUNT: usize = 70;
/// Brillo máximo de un canal del fondo para dibujar un destello encima.
const SPARKLE_MAX_BACKGROUND: f32 = 60.0;
/// Lado de cada glifo de la fuente de bloques.
const FONT_SIZE: usize = 5;

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

/// Devuelve el color de la baldosa del suelo en la posición del mundo (x, y).
/// Las juntas se desvanecen con la distancia para evitar aliasing.
fn floor_texel(x: f32, y: f32, distance: f32) -> [f32; 3] {
    let (tx, ty) = (x.floor() as i32, y.floor() as i32);
    let grout = FLOOR_GROUT * (1.0 - distance / 8.0).max(0.0);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    if fx < grout || fy < grout || fx > 1.0 - grout || fy > 1.0 - grout {
        return FLOOR_GROUT_COLOR;
    }
    let tile = hash2(tx * 7 + 3, ty * 11 + 5);
    let grain = hash2((x * 20.0) as i32, (y * 20.0) as i32);
    let checker = if (tx + ty) % 2 == 0 { 1.0 } else { 0.9 };
    let tone = checker * (0.8 + 0.25 * tile + 0.08 * (grain - 0.5));
    [FLOOR_COLOR[0] * tone, FLOOR_COLOR[1] * tone, FLOOR_COLOR[2] * tone]
}

/// Devuelve el glifo de 5×5 de la fuente de bloques para una letra mayúscula.
fn font_glyph(ch: char) -> [&'static str; FONT_SIZE] {
    match ch {
        'L' => ["#....", "#....", "#....", "#....", "#####"],
        'A' => [".###.", "#...#", "#####", "#...#", "#...#"],
        'B' => ["####.", "#...#", "####.", "#...#", "####."],
        'E' => ["#####", "#....", "####.", "#....", "#####"],
        'R' => ["####.", "#...#", "####.", "#..#.", "#...#"],
        'I' => ["#####", "..#..", "..#..", "..#..", "#####"],
        'N' => ["#...#", "##..#", "#.#.#", "#..##", "#...#"],
        'T' => ["#####", "..#..", "..#..", "..#..", "..#.."],
        'O' => [".###.", "#...#", "#...#", "#...#", ".###."],
        'V' => ["#...#", "#...#", "#...#", ".#.#.", "..#.."],
        'S' => [".####", "#....", ".###.", "....#", "####."],
        'C' => [".####", "#....", "#....", "#....", ".####"],
        _ => [".....", ".....", ".....", ".....", "....."],
    }
}

/// Escribe texto centrado en una fila del framebuffer, mezclando su color con el fondo según `alpha`.
fn draw_text_centered(framebuffer: &mut Framebuffer, y: usize, text: &str, color: [f32; 3], alpha: f32) {
    if y >= framebuffer.height { return; }
    let len = text.chars().count();
    let start_x = framebuffer.width.saturating_sub(len) / 2;
    for (i, ch) in text.chars().enumerate() {
        let x = start_x + i;
        if x >= framebuffer.width { break; }
        // El fondo de la celda es la mitad superior del píxel doble que ya estaba dibujado.
        let bg = match framebuffer.pixels[y][x].color {
            Color::Rgb { r, g, b } => [r as f32, g as f32, b as f32],
            _ => [0.0, 0.0, 0.0],
        };
        let fg = lerp_color(bg, color, alpha);
        framebuffer.set_pixel(x, y, Pixel::with_bg(to_rgb(fg), to_rgb(bg), ch));
    }
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
    last_frame: Instant, // Momento del último cuadro, para animaciones dependientes del tiempo.
    last_position: Option<Vec2>, // Posición del jugador en el cuadro anterior.
    walk_distance: f32, // Distancia total caminada; define la fase del balanceo de cámara.
    bob_strength: f32, // Intensidad actual del balanceo (1 = caminando, decae a 0 al detenerse).
    pickup_flash: Option<Instant>, // Momento en que se recogió el último ítem.
    level_intro: Option<(Instant, String)>, // Inicio y título de la presentación del nivel.
    fade_out: Option<Instant>, // Inicio del fundido a negro al completar el nivel.
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
            last_frame: Instant::now(),
            last_position: None,
            walk_distance: 0.0,
            bob_strength: 0.0,
            pickup_flash: None,
            level_intro: None,
            fade_out: None,
        }
    }

    /// Limpia toda la terminal, para no dejar restos de la pantalla anterior.
    pub fn clear_screen(&self) {
        let mut stdout = stdout();
        stdout.execute(ResetColor).unwrap();
        stdout.execute(Clear(ClearType::All)).unwrap();
    }

    /// Muestra el menú principal del juego, con un título animado en letras de bloque.
    /// Se dibuja en un framebuffer de pantalla completa (sin limpiar la terminal) para evitar parpadeos.
    pub fn show_menu(&self) {
        let (cols, rows) = size().unwrap();
        let framebuffer = self.render_menu(cols as usize, rows as usize);
        self.display_framebuffer(&framebuffer);
    }

    /// Dibuja el fondo con viñeta y un título animado en letras de bloque, centrando verticalmente
    /// el título junto con `text_rows` filas de texto. Devuelve el framebuffer y la fila donde
    /// empieza el texto.
    fn render_title_screen(&self, width: usize, height: usize, title: &str, text_rows: usize) -> (Framebuffer, usize) {
        let hi_height = height * 2;
        let time = self.start_time.elapsed().as_secs_f32();

        // Fondo: degradado vertical con viñeta.
        let mut hi_res = vec![[0.0f32; 3]; width * hi_height];
        for y in 0..hi_height {
            for x in 0..width {
                let dx = (x as f32 / width as f32 - 0.5) * 2.0;
                let dy = (y as f32 / hi_height as f32 - 0.5) * 2.0;
                let vignette = 1.0 - 0.6 * ((dx * dx + dy * dy) / 2.0).sqrt();
                let base = lerp_color(SKY_TOP, FOG_COLOR, y as f32 / hi_height as f32);
                hi_res[y * width + x] = [base[0] * vignette, base[1] * vignette, base[2] * vignette];
            }
        }

        // Título en letras de bloque; cada punto de la fuente ocupa `scale`×`scale` píxeles.
        let glyph_cols = title.len() * (FONT_SIZE + 1) - 1;
        let scale = if width >= glyph_cols * 2 + 4 { 2 } else { 1 };
        let title_w = glyph_cols * scale;
        let title_h = FONT_SIZE * scale;
        let title_x = width.saturating_sub(title_w) / 2;
        // Centra verticalmente el bloque completo: título + filas de texto.
        let content_rows = title_h / 2 + text_rows;
        let title_y = (height.saturating_sub(content_rows) / 2 * 2).max(2);

        for (i, ch) in title.chars().enumerate() {
            let glyph = font_glyph(ch);
            for (gy, line) in glyph.iter().enumerate() {
                for (gx, cell) in line.chars().enumerate() {
                    if cell != '#' { continue; }
                    for sy in 0..scale {
                        for sx in 0..scale {
                            let x = title_x + (i * (FONT_SIZE + 1) + gx) * scale + sx;
                            let y = title_y + gy * scale + sy;
                            // Sombra desplazada y luego el relleno con brillo que recorre el título.
                            if x + 1 < width && y + 1 < hi_height {
                                hi_res[(y + 1) * width + x + 1] = [10.0, 6.0, 4.0];
                            }
                            if x < width && y < hi_height {
                                let v = (y - title_y) as f32 / title_h as f32;
                                let shine = ((x as f32 * 0.08 - time * 3.0).sin() * 0.5 + 0.5).powf(6.0);
                                let base = lerp_color([255.0, 214.0, 90.0], [200.0, 90.0, 30.0], v);
                                hi_res[y * width + x] = lerp_color(base, [255.0, 250.0, 230.0], shine * 0.8);
                            }
                        }
                    }
                }
            }
        }

        let mut framebuffer = Framebuffer::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let top = to_rgb(hi_res[(2 * y) * width + x]);
                let bottom = to_rgb(hi_res[(2 * y + 1) * width + x]);
                framebuffer.set_pixel(x, y, Pixel::with_bg(top, bottom, '▀'));
            }
        }

        (framebuffer, (title_y + title_h) / 2 + 2)
    }

    /// Dibuja el menú principal en un framebuffer del tamaño indicado.
    pub fn render_menu(&self, width: usize, height: usize) -> Framebuffer {
        let time = self.start_time.elapsed().as_secs_f32();
        let (mut framebuffer, text_y) = self.render_title_screen(width, height, "LABERINTO", MENU_TEXT_ROWS);
        let white = [225.0, 225.0, 235.0];
        let dim = [150.0, 150.0, 170.0];
        let gold = [255.0, 200.0, 70.0];
        draw_text_centered(&mut framebuffer, text_y, "D E   R E C O L E C C I Ó N", [235.0, 150.0, 70.0], 1.0);
        draw_text_centered(&mut framebuffer, text_y + 2, "◆ ─────────────────────── ◆", gold, 0.7);
        draw_text_centered(&mut framebuffer, text_y + 4, "CONTROLES", gold, 1.0);
        let controls = [
            ("WASD / ↑↓←→", "Mover"),
            ("Q / E", "Rotar"),
            ("C", "Cambiar vista"),
            ("X / ESC", "Salir"),
        ];
        for (i, (keys, action)) in controls.iter().enumerate() {
            // Columnas de ancho fijo para que todas las filas queden alineadas al centrarlas.
            let line = format!("{:<14}{:<13}", keys, action);
            draw_text_centered(&mut framebuffer, text_y + 6 + i, &line, white, 1.0);
        }
        draw_text_centered(&mut framebuffer, text_y + 11, "Recolecta ◆ y llega a la meta ● en 3 niveles", dim, 1.0);

        let pulse = 0.55 + 0.45 * (time * 3.0).sin();
        draw_text_centered(&mut framebuffer, text_y + 14, "▶  Presiona ENTER para jugar  ◀", [120.0, 255.0, 150.0], pulse);

        framebuffer
    }

    /// Muestra la pantalla de victoria cuando se completan todos los niveles.
    pub fn show_victory(&self) {
        let (cols, rows) = size().unwrap();
        let framebuffer = self.render_victory(cols as usize, rows as usize);
        self.display_framebuffer(&framebuffer);
    }

    /// Dibuja la pantalla de victoria, con destellos dorados que caen.
    pub fn render_victory(&self, width: usize, height: usize) -> Framebuffer {
        let time = self.start_time.elapsed().as_secs_f32();
        let (mut framebuffer, text_y) = self.render_title_screen(width, height, "VICTORIA", VICTORY_TEXT_ROWS);

        // Destellos: posición pseudoaleatoria estable por índice, caen y titilan con el tiempo.
        for i in 0..SPARKLE_COUNT as i32 {
            let x = (hash2(i, 1) * width as f32) as usize;
            let speed = 2.0 + 4.0 * hash2(i, 2);
            let y = ((hash2(i, 3) * height as f32 + time * speed) % height as f32) as usize;
            let twinkle = 0.5 + 0.5 * (time * 5.0 + hash2(i, 4) * 6.28).sin();
            let symbol = if hash2(i, 5) < 0.5 { '✦' } else { '·' };
            if x >= width || y >= height || framebuffer.pixels[y][x].symbol != '▀' { continue; }
            let (top, bg) = match (framebuffer.pixels[y][x].color, framebuffer.pixels[y][x].bg) {
                (Color::Rgb { r, g, b }, Color::Rgb { r: br, g: bg, b: bb }) => {
                    ([r as f32, g as f32, b as f32], [br as f32, bg as f32, bb as f32])
                }
                _ => continue,
            };
            // Solo sobre el fondo oscuro: así nunca tapan las letras del título.
            if top.iter().chain(bg.iter()).any(|&c| c > SPARKLE_MAX_BACKGROUND) { continue; }
            let fg = lerp_color(bg, FLASH_COLOR, twinkle);
            framebuffer.set_pixel(x, y, Pixel::with_bg(to_rgb(fg), to_rgb(bg), symbol));
        }

        let gold = [255.0, 200.0, 70.0];
        draw_text_centered(&mut framebuffer, text_y, "◆ ─────────────────────── ◆", gold, 0.7);
        draw_text_centered(&mut framebuffer, text_y + 2, "¡Completaste todos los niveles!", [235.0, 235.0, 245.0], 1.0);
        let pulse = 0.55 + 0.45 * (time * 3.0).sin();
        draw_text_centered(&mut framebuffer, text_y + 5, "Presiona cualquier tecla para salir", [120.0, 255.0, 150.0], pulse);
        framebuffer
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

    /// Inicia la presentación del nivel: pantalla negra con el título y luego aparición gradual.
    pub fn start_level_intro(&mut self, title: &str) {
        self.level_intro = Some((Instant::now(), title.to_string()));
    }

    /// Inicia el fundido a negro al completar un nivel.
    pub fn start_fade_out(&mut self) {
        self.fade_out = Some(Instant::now());
    }

    /// Indica si el fundido a negro ya terminó.
    pub fn fade_out_finished(&self) -> bool {
        self.fade_out.is_some_and(|start| start.elapsed().as_secs_f32() >= FADE_OUT_DURATION)
    }

    /// Devuelve cuánto oscurecer la escena (0 = normal, 1 = negro) según las transiciones activas.
    fn transition_darkness(&self) -> f32 {
        let intro = self.level_intro.as_ref().map_or(0.0, |(start, _)| {
            let t = start.elapsed().as_secs_f32();
            1.0 - ((t - INTRO_HOLD) / INTRO_FADE).clamp(0.0, 1.0)
        });
        let outro = self.fade_out.map_or(0.0, |start| (start.elapsed().as_secs_f32() / FADE_OUT_DURATION).min(1.0));
        intro.max(outro)
    }

    /// Dibuja el título del nivel centrado durante la presentación, con aparición y desvanecimiento.
    fn render_level_title(&self, framebuffer: &mut Framebuffer) {
        let Some((start, title)) = &self.level_intro else { return };
        let t = start.elapsed().as_secs_f32();
        let fade_in = (t / 0.3).min(1.0);
        let fade_out = 1.0 - ((t - INTRO_HOLD - INTRO_FADE * 0.3) / (INTRO_FADE * 0.5)).clamp(0.0, 1.0);
        let alpha = fade_in.min(fade_out);
        if alpha <= 0.0 { return; }

        let center_y = framebuffer.height / 2;
        let rule = "◆ ───────────── ◆";
        draw_text_centered(framebuffer, center_y.saturating_sub(2), rule, FLASH_COLOR, alpha * 0.8);
        draw_text_centered(framebuffer, center_y, title, [255.0, 240.0, 200.0], alpha);
        draw_text_centered(framebuffer, center_y + 2, rule, FLASH_COLOR, alpha * 0.8);
    }

    /// Dispara el destello de pantalla y el resaltado del contador al recoger un ítem.
    pub fn trigger_pickup_flash(&mut self) {
        self.pickup_flash = Some(Instant::now());
    }

    /// Devuelve la intensidad actual del destello de recolección, de 1 (recién recogido) a 0.
    fn pickup_flash_intensity(&self) -> f32 {
        self.pickup_flash.map_or(0.0, |start| {
            let t = start.elapsed().as_secs_f32() / FLASH_DURATION;
            if t >= 1.0 { 0.0 } else { (1.0 - t) * (1.0 - t) }
        })
    }

    /// Actualiza el balanceo de cámara según cuánto se movió el jugador desde el último cuadro
    /// y devuelve el desplazamiento vertical del horizonte en píxeles.
    fn update_head_bob(&mut self, player: &Player, hi_height: usize) -> f32 {
        let dt = self.last_frame.elapsed().as_secs_f32();
        self.last_frame = Instant::now();

        let moved = self.last_position.map_or(0.0, |last| {
            Vec2::new(player.position.x - last.x, player.position.y - last.y).length()
        });
        self.last_position = Some(player.position);

        // Un salto grande (cambio de nivel o reinicio) no cuenta como caminar.
        if moved > 0.0 && moved < 1.0 {
            self.walk_distance += moved;
            self.bob_strength = 1.0;
        } else {
            self.bob_strength = (self.bob_strength - dt / BOB_SETTLE_TIME).max(0.0);
        }

        (self.walk_distance * BOB_FREQUENCY).sin() * BOB_AMPLITUDE * self.bob_strength * hi_height as f32
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
        let horizon = hi_height as f32 / 2.0 + self.update_head_bob(player, hi_height);
        let mut hi_res = vec![[0.0f32; 3]; width * hi_height];

        // Lanzamiento de rayos (Raycasting) para renderizar las paredes.
        let ray_angle_step = player.fov / width as f32;
        let start_angle = -player.fov / 2.0;

        // Cielo y suelo: el cielo se funde con la niebla en el horizonte; el suelo se proyecta
        // (floor casting) para texturizarlo con baldosas y se oscurece con la misma niebla.
        let rays: Vec<(Vec2, f32)> = (0..width)
            .map(|i| {
                let angle = start_angle + i as f32 * ray_angle_step;
                (player.direction.rotate(angle), angle.cos())
            })
            .collect();
        for y in 0..hi_height {
            let row_center = y as f32 + 0.5;
            let row = &mut hi_res[y * width..(y + 1) * width];
            if row_center < horizon {
                let t = row_center / horizon;
                row.fill(lerp_color(SKY_TOP, FOG_COLOR, t * t));
                continue;
            }
            let row_distance = hi_height as f32 / (2.0 * (row_center - horizon));
            for (x, (ray_dir, cos)) in rays.iter().enumerate() {
                let along_ray = row_distance / cos;
                let wx = player.position.x + ray_dir.x * along_ray;
                let wy = player.position.y + ray_dir.y * along_ray;
                row[x] = apply_fog(floor_texel(wx, wy, row_distance), row_distance);
            }
        }
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

        self.render_sprites(&mut hi_res, &z_buffer, width, hi_height, horizon, player, map);
        self.render_pickup_flash(&mut hi_res, width, hi_height);

        self.render_minimap(&mut hi_res, width, hi_height, player, map);

        let darkness = self.transition_darkness();
        if darkness > 0.0 {
            for pixel in hi_res.iter_mut() {
                *pixel = lerp_color(*pixel, [0.0, 0.0, 0.0], darkness);
            }
        }

        // Combina cada par de píxeles verticales en una sola celda de la terminal.
        for y in 0..framebuffer.height {
            for x in 0..width {
                let top = to_rgb(hi_res[(2 * y) * width + x]);
                let bottom = to_rgb(hi_res[(2 * y + 1) * width + x]);
                framebuffer.set_pixel(x, y, Pixel::with_bg(top, bottom, '▀'));
            }
        }

        self.render_level_title(framebuffer);
    }

    /// Renderiza los objetos del mapa (ítems y meta) como sprites 3D, de atrás hacia adelante,
    /// respetando el z-buffer de las paredes.
    #[allow(clippy::too_many_arguments)]
    fn render_sprites(&self, buf: &mut [[f32; 3]], z_buffer: &[f32], width: usize, height: usize, horizon: f32, player: &Player, map: &[Vec<u8>]) {
        let time = self.start_time.elapsed().as_secs_f32();
        let dir = player.direction.normalize();
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

    /// Tiñe la escena de dorado tras recoger un ítem, con más intensidad en los bordes.
    fn render_pickup_flash(&self, buf: &mut [[f32; 3]], width: usize, height: usize) {
        let intensity = self.pickup_flash_intensity();
        if intensity <= 0.0 { return; }

        let (cx, cy) = (width as f32 / 2.0, height as f32 / 2.0);
        for y in 0..height {
            for x in 0..width {
                let dx = (x as f32 - cx) / cx;
                let dy = (y as f32 - cy) / cy;
                let edge = ((dx * dx + dy * dy) / 2.0).sqrt();
                let idx = y * width + x;
                buf[idx] = lerp_color(buf[idx], FLASH_COLOR, intensity * FLASH_STRENGTH * (0.35 + 0.65 * edge));
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
        print!("{}", format!("Nivel {}/{}: {} | ", level, total_levels, level_name).with(Color::Yellow));
        let items = format!("Items: {}/{} ◆", collected, required);
        if self.pickup_flash_intensity() > 0.0 {
            print!("{}", items.with(Color::Rgb { r: 255, g: 240, b: 160 }).bold());
        } else {
            print!("{}", items.with(Color::Yellow));
        }
        print!("{}", format!(" | Pos: ({:.1}, {:.1})", px, py).with(Color::Yellow));
        
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
        let (cols, rows) = size().unwrap();
        let framebuffer = self.render_level_select(cols as usize, rows as usize, selected, levels);
        self.display_framebuffer(&framebuffer);
    }

    /// Dibuja la pantalla de selección de nivel, resaltando el nivel elegido.
    pub fn render_level_select(&self, width: usize, height: usize, selected: usize, levels: &[Level]) -> Framebuffer {
        let time = self.start_time.elapsed().as_secs_f32();
        let text_rows = levels.len() * 2 + 9;
        let (mut framebuffer, text_y) = self.render_title_screen(width, height, "NIVELES", text_rows);

        let gold = [255.0, 200.0, 70.0];
        let dim = [150.0, 150.0, 170.0];
        draw_text_centered(&mut framebuffer, text_y, "◆ ─────────────────────── ◆", gold, 0.7);

        // Todas las filas tienen el mismo ancho para que queden alineadas al centrarlas.
        let name_width = levels.iter().map(|l| l.name.chars().count()).max().unwrap_or(0);
        let pulse = 0.7 + 0.3 * (time * 4.0).sin();
        for (i, level) in levels.iter().enumerate() {
            let (marker, color, alpha) = if i == selected { ('▶', [255.0, 225.0, 120.0], pulse) } else { (' ', dim, 1.0) };
            let line = format!("{} {:<w$}   {} ◆", marker, level.name, level.required_items, w = name_width);
            draw_text_centered(&mut framebuffer, text_y + 2 + i * 2, &line, color, alpha);
        }

        let help_y = text_y + 3 + levels.len() * 2;
        draw_text_centered(&mut framebuffer, help_y, "↑/↓ o W/S: elegir   ·   1/2/3: ir directo", dim, 1.0);
        draw_text_centered(&mut framebuffer, help_y + 3, "▶  ENTER para jugar  ◀", [120.0, 255.0, 150.0], 0.55 + 0.45 * (time * 3.0).sin());
        draw_text_centered(&mut framebuffer, help_y + 5, "ESC: volver al menú", dim, 0.8);
        framebuffer
    }
}
