use crossterm::{
    event::{Event, KeyCode, KeyEvent, poll, read},
    terminal::size,
    cursor::Hide,
    ExecutableCommand,
};
use std::time::{Duration, Instant};

use raytracer_maze::{
    Camera, CameraMode, Framebuffer, GameRenderer, 
    GameState, Player, Level,
};

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
                vec![1, 2, 0, 0, 5, 1, 0, 0, 0, 1],
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
                vec![1, 0, 0, 0, 1, 1, 1, 0, 0, 5, 0, 1],
                vec![1, 1, 1, 0, 1, 0, 0, 0, 1, 1, 0, 1],
                vec![1, 0, 0, 5, 0, 0, 1, 0, 0, 0, 0, 1],
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
                vec![1, 5, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1],
                vec![1, 0, 1, 0, 0, 0, 0, 1, 1, 1, 0, 1, 5, 1],
                vec![1, 0, 1, 0, 1, 1, 0, 0, 0, 1, 0, 1, 0, 1],
                vec![1, 5, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 5, 1],
                vec![1, 1, 1, 1, 0, 0, 0, 0, 1, 0, 1, 1, 0, 1],
                vec![1, 0, 0, 5, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1],
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

    crossterm::terminal::enable_raw_mode().unwrap();
    let mut stdout = std::io::stdout();
    stdout.execute(Hide).unwrap();

    let target_frame_time = Duration::from_millis(16);
    let mut running = true;
    let mut first_render = true;

    let mut renderer = GameRenderer::new(
        cols,
        rows,
        levels[current_level].get_width(),
        levels[current_level].get_height(),
    );
    let mut fb_topdown = Framebuffer::new(
        levels[current_level].get_width() * renderer.get_cell_width(),
        levels[current_level].get_height() * renderer.get_cell_height(),
    );
    let mut fb_firstperson = Framebuffer::new(cols, rows.saturating_sub(4));

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
                                player = Player::from_map(&levels[current_level].map);
                                
                                renderer = GameRenderer::new(
                                    cols,
                                    rows,
                                    levels[current_level].get_width(),
                                    levels[current_level].get_height(),
                                );
                                fb_topdown = Framebuffer::new(
                                    levels[current_level].get_width() * renderer.get_cell_width(),
                                    levels[current_level].get_height() * renderer.get_cell_height(),
                                );
                                
                                game_state = GameState::Playing;
                                first_render = true;
                            },
                            KeyCode::Esc => game_state = GameState::Menu,
                            _ => {}
                        }
                    }
                }
            }

            GameState::Playing => {
                let level = &mut levels[current_level];
                
                // Verificar si el jugador está en un item y recogerlo
                let (px, py) = player.get_grid_position();
                if level.collect_item(px, py) {
                    collected_items += 1;
                    player.has_moved = true; // Forzar re-render
                }

                // Verificar victoria del nivel
                if player.is_at_goal(&level.map) && collected_items >= level.required_items {
                    if current_level < levels.len() - 1 {
                        // Siguiente nivel
                        current_level += 1;
                        collected_items = 0;
                        player = Player::from_map(&levels[current_level].map);
                        
                        renderer = GameRenderer::new(
                            cols,
                            rows,
                            levels[current_level].get_width(),
                            levels[current_level].get_height(),
                        );
                        fb_topdown = Framebuffer::new(
                            levels[current_level].get_width() * renderer.get_cell_width(),
                            levels[current_level].get_height() * renderer.get_cell_height(),
                        );
                        
                        first_render = true;
                    } else {
                        // Juego completado
                        game_state = GameState::Victory;
                        continue;
                    }
                }

                if player.has_moved || first_render {
                    let level_ref = &levels[current_level];
                    match camera.mode {
                        CameraMode::TopDown => {
                            renderer.render_top_down(
                                &mut fb_topdown,
                                &level_ref.map,
                                &player,
                                &[],
                            );
                            renderer.display_framebuffer(&fb_topdown);
                        }
                        CameraMode::FirstPerson => {
                            renderer.render_first_person(
                                &mut fb_firstperson,
                                &player,
                                &level_ref.map,
                            );
                            renderer.display_framebuffer(&fb_firstperson);
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
                    );

                    player.has_moved = false;
                    first_render = false;
                }

                fps_counter.update();

                if poll(Duration::from_millis(1)).unwrap() {
                    if let Ok(Event::Key(KeyEvent { code, .. })) = read() {
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
                                player.has_moved = true;
                            }
                            KeyCode::Char('x') | KeyCode::Esc => running = false,
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