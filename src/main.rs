use crossterm::{
    event::{Event, KeyCode, KeyEvent, poll, read},
    terminal::size,
    cursor::Hide,
    ExecutableCommand,
};
use std::time::{Duration, Instant};

use raytracer_maze::{
    Audio, Camera, CameraMode, Enemy, Framebuffer, GameRenderer,
    GameState, Player, Level, Sfx, Vec2, ENEMY_HIT_RADIUS,
};

/// Vidas con las que empieza cada nivel.
const MAX_LIVES: u32 = 3;
/// Segundos de invulnerabilidad tras recibir un golpe.
const HIT_INVULNERABILITY: f32 = 1.5;
/// Segundos de invulnerabilidad al empezar un nivel, mientras dura la presentación.
const START_INVULNERABILITY: f32 = 2.0;
/// Distancia a la que un golpe empuja al jugador lejos del enemigo.
const KNOCKBACK: f32 = 0.5;

/// Prepara todo lo que se reinicia al empezar un nivel: jugador, enemigos y renderizador.
/// Muestra `title` en la presentación del nivel.
fn start_level(level: &Level, title: &str, cols: usize, rows: usize) -> (Player, Vec<Enemy>, GameRenderer) {
    let mut player = Player::from_map(&level.map);
    player.invulnerable = START_INVULNERABILITY;
    let enemies = level
        .enemy_spawns
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| Enemy::new(x, y, i as u32 + 1))
        .collect();

    let mut renderer = GameRenderer::new(cols, rows, level.get_width(), level.get_height());
    renderer.start_level_intro(title);
    renderer.clear_screen();
    (player, enemies, renderer)
}

struct FpsCounter {
    frame_count: u32,
    last_time: Instant,
    fps: f32,
}

impl FpsCounter {
    fn new() -> Self {
        Self {
            frame_count: 0,
            last_time: Instant::now(),
            fps: 0.0,
        }
    }

    fn update(&mut self) {
        self.frame_count += 1;
        let elapsed = self.last_time.elapsed();
        
        if elapsed >= Duration::from_millis(500) {
            self.fps = self.frame_count as f32 / elapsed.as_secs_f32();
            self.frame_count = 0;
            self.last_time = Instant::now();
        }
    }

    fn get_fps(&self) -> f32 {
        self.fps
    }
}

fn create_levels() -> Vec<Level> {
    vec![
        Level::new(
            vec![
                vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
                vec![1, 2, 0, 0, 5, 1, 0, 4, 0, 1],
                vec![1, 0, 1, 0, 0, 0, 0, 1, 0, 1],
                vec![1, 0, 0, 5, 0, 1, 0, 0, 0, 1],
                vec![1, 0, 1, 0, 0, 0, 5, 0, 0, 1],
                vec![1, 0, 0, 0, 1, 0, 0, 1, 3, 1],
                vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
            ],
            3,
            "Nivel 1: El Comienzo"
        ),
        Level::new(
            vec![
                vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
                vec![1, 2, 0, 0, 5, 1, 0, 5, 0, 0, 0, 1],
                vec![1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1],
                vec![1, 5, 1, 0, 0, 0, 0, 0, 1, 0, 1, 1],
                vec![1, 0, 0, 0, 1, 1, 1, 0, 0, 5, 4, 1],
                vec![1, 1, 1, 0, 1, 0, 0, 0, 1, 1, 0, 1],
                vec![1, 0, 0, 5, 0, 0, 1, 4, 0, 0, 0, 1],
                vec![1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 3, 1],
                vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
            ],
            5,
            "Nivel 2: El Laberinto"
        ),
        Level::new(
            vec![
                vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
                vec![1, 2, 0, 0, 5, 0, 0, 1, 0, 5, 0, 0, 0, 1],
                vec![1, 0, 1, 1, 0, 1, 0, 1, 0, 1, 1, 1, 0, 1],
                vec![1, 5, 0, 0, 0, 1, 0, 0, 0, 0, 4, 0, 0, 1],
                vec![1, 0, 1, 0, 0, 0, 0, 1, 1, 1, 0, 1, 5, 1],
                vec![1, 0, 1, 0, 1, 1, 0, 0, 0, 1, 0, 1, 4, 1],
                vec![1, 5, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 5, 1],
                vec![1, 1, 1, 1, 0, 0, 0, 0, 1, 0, 1, 1, 0, 1],
                vec![1, 0, 0, 5, 0, 1, 4, 0, 1, 0, 0, 0, 0, 1],
                vec![1, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 3, 1],
                vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
            ],
            8,
            "Nivel 3: El Desafío Final"
        ),
    ]
}

fn main() {
    let mut levels = create_levels();
    let mut current_level = 0;
    let mut collected_items = 0;
    let mut selected_level = 0;

    let (cols, rows) = size().unwrap();
    let cols = cols as usize;
    let rows = rows as usize;

    let mut player = Player::from_map(&levels[current_level].map);
    let mut camera = Camera::new();
    let mut game_state = GameState::Menu;
    let mut fps_counter = FpsCounter::new();

    // El audio es opcional: sin dispositivo de salida el juego sigue en silencio.
    let mut audio = Audio::new();

    crossterm::terminal::enable_raw_mode().unwrap();
    let mut stdout = std::io::stdout();
    stdout.execute(Hide).unwrap();

    let target_frame_time = Duration::from_millis(16);
    let mut running = true;
    let mut level_complete = false;
    let mut enemies: Vec<Enemy> = Vec::new();
    let mut lives = MAX_LIVES;
    let mut last_update = Instant::now();

    let mut renderer = GameRenderer::new(
        cols,
        rows,
        levels[current_level].get_width(),
        levels[current_level].get_height(),
    );
    // Las dos cámaras dibujan a pantalla completa, dejando las últimas filas para la UI.
    let mut fb_game = Framebuffer::new(cols, rows.saturating_sub(4));

    while running {
        let frame_start = Instant::now();

        match game_state {
            GameState::Menu => {
                renderer.show_menu();

                if poll(Duration::from_millis(16)).unwrap() {
                    if let Ok(Event::Key(KeyEvent { code, .. })) = read() {
                        match code {
                            KeyCode::Enter => {
                                game_state = GameState::LevelSelect;
                            },
                            KeyCode::Char('x') | KeyCode::Esc => running = false,
                            KeyCode::Char('m') => {
                                if let Some(audio) = audio.as_mut() { audio.toggle_mute(); }
                            }
                            _ => {}
                        }
                    }
                }
            }

            GameState::LevelSelect => {
                renderer.show_level_select(selected_level, &levels);

                if poll(Duration::from_millis(16)).unwrap() {
                    if let Ok(Event::Key(KeyEvent { code, .. })) = read() {
                        match code {
                            KeyCode::Up | KeyCode::Char('w') => {
                                if selected_level > 0 {
                                    selected_level -= 1;
                                }
                            },
                            KeyCode::Down | KeyCode::Char('s') => {
                                if selected_level < levels.len() - 1 {
                                    selected_level += 1;
                                }
                            },
                            KeyCode::Char('1') => selected_level = 0,
                            KeyCode::Char('2') => selected_level = 1.min(levels.len() - 1),
                            KeyCode::Char('3') => selected_level = 2.min(levels.len() - 1),
                            KeyCode::Enter => {
                                // Reiniciar el nivel seleccionado
                                levels = create_levels(); // Recrear todos los niveles
                                current_level = selected_level;
                                collected_items = 0;
                                lives = MAX_LIVES;
                                let level = &levels[current_level];
                                (player, enemies, renderer) = start_level(level, &level.name, cols, rows);
                                level_complete = false;
                                last_update = Instant::now();
                                game_state = GameState::Playing;
                            },
                            KeyCode::Esc => game_state = GameState::Menu,
                            _ => {}
                        }
                    }
                }
            }

            GameState::Playing => {
                let dt = last_update.elapsed().as_secs_f32().min(0.1);
                last_update = Instant::now();

                // Enemigos: se quedan quietos durante el fundido de fin de nivel.
                player.invulnerable = (player.invulnerable - dt).max(0.0);
                if !level_complete {
                    for enemy in &mut enemies {
                        enemy.update(dt, player.position, &levels[current_level].map);
                    }

                    let hit = enemies.iter().find(|e| {
                        Vec2::new(e.position.x - player.position.x, e.position.y - player.position.y).length() < ENEMY_HIT_RADIUS
                    });
                    if let (Some(enemy), true) = (hit, player.invulnerable <= 0.0) {
                        lives -= 1;
                        if lives == 0 {
                            // Sin vidas: el nivel se reinicia desde cero, con sus ítems.
                            if let Some(audio) = &audio { audio.play(Sfx::GameOver); }
                            levels[current_level] = create_levels().swap_remove(current_level);
                            collected_items = 0;
                            lives = MAX_LIVES;
                            let level = &levels[current_level];
                            let title = format!("Sin vidas · {}", level.name);
                            (player, enemies, renderer) = start_level(level, &title, cols, rows);
                        } else {
                            if let Some(audio) = &audio { audio.play(Sfx::Hurt); }
                            renderer.trigger_damage_flash();
                            player.invulnerable = HIT_INVULNERABILITY;
                            let away = Vec2::new(player.position.x - enemy.position.x, player.position.y - enemy.position.y).normalize();
                            player.push(Vec2::new(away.x * KNOCKBACK, away.y * KNOCKBACK), &levels[current_level].map);
                        }
                    }
                }

                let level = &mut levels[current_level];

                // Verificar si el jugador está en un item y recogerlo
                let (px, py) = player.get_grid_position();
                if level.collect_item(px, py) {
                    collected_items += 1;
                    renderer.trigger_pickup_flash();
                    if let Some(audio) = &audio { audio.play(Sfx::Pickup); }
                    player.has_moved = true; // Forzar re-render
                }

                // Verificar victoria del nivel: primero se funde a negro y luego se cambia de nivel.
                if !level_complete && player.is_at_goal(&level.map) && collected_items >= level.required_items {
                    level_complete = true;
                    renderer.start_fade_out();
                    if let Some(audio) = &audio { audio.play(Sfx::LevelComplete); }
                }
                if level_complete && renderer.fade_out_finished() {
                    level_complete = false;
                    if current_level < levels.len() - 1 {
                        // Siguiente nivel
                        current_level += 1;
                        collected_items = 0;
                        lives = MAX_LIVES;
                        let level = &levels[current_level];
                        (player, enemies, renderer) = start_level(level, &level.name, cols, rows);
                    } else {
                        // Juego completado
                        game_state = GameState::Victory;
                        continue;
                    }
                }

                // Hay animaciones y enemigos en movimiento, así que se redibuja en cada cuadro.
                {
                    let level_ref = &levels[current_level];
                    match camera.mode {
                        CameraMode::TopDown => {
                            renderer.render_top_down(
                                &mut fb_game,
                                &level_ref.map,
                                &player,
                                &enemies,
                            );
                            renderer.display_framebuffer(&fb_game);
                        }
                        CameraMode::FirstPerson => {
                            renderer.render_first_person(
                                &mut fb_game,
                                &player,
                                &level_ref.map,
                                &enemies,
                            );
                            renderer.display_framebuffer(&fb_game);
                        }
                    }

                    renderer.display_ui(
                        fps_counter.get_fps(),
                        player.position.x,
                        player.position.y,
                        collected_items,
                        level_ref.required_items,
                        current_level + 1,
                        levels.len(),
                        &level_ref.name,
                        lives,
                        MAX_LIVES,
                    );

                    player.has_moved = false;
                }

                fps_counter.update();

                if poll(Duration::from_millis(1)).unwrap() {
                    if let Ok(Event::Key(KeyEvent { code, .. })) = read() {
                        // Durante el fundido de fin de nivel solo se permite salir o silenciar.
                        let code = if level_complete && !matches!(code, KeyCode::Char('x') | KeyCode::Esc | KeyCode::Char('m')) {
                            KeyCode::Null
                        } else {
                            code
                        };
                        match code {
                            KeyCode::Char('w') | KeyCode::Up => 
                                player.move_forward(0.15, &levels[current_level].map),
                            KeyCode::Char('s') | KeyCode::Down => 
                                player.move_backward(0.15, &levels[current_level].map),
                            KeyCode::Char('a') | KeyCode::Left => 
                                player.strafe(-0.15, &levels[current_level].map),
                            KeyCode::Char('d') | KeyCode::Right => 
                                player.strafe(0.15, &levels[current_level].map),
                            KeyCode::Char('q') => player.rotate(-0.08),
                            KeyCode::Char('e') => player.rotate(0.08),
                            KeyCode::Char('c') => {
                                camera.toggle_mode();
                                renderer.clear_screen();
                                player.has_moved = true;
                            }
                            KeyCode::Char('x') | KeyCode::Esc => running = false,
                            KeyCode::Char('m') => {
                                if let Some(audio) = audio.as_mut() { audio.toggle_mute(); }
                            }
                            _ => {}
                        }
                    }
                }

                let frame_time = frame_start.elapsed();
                if frame_time < target_frame_time {
                    std::thread::sleep(target_frame_time - frame_time);
                }
            }

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
    println!("\n¡Gracias por jugar! FPS promedio: {:.1}", fps_counter.get_fps());
}