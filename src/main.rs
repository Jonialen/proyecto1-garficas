use crossterm::{
    event::{Event, KeyCode, KeyEvent, poll, read},
    terminal::size,
    cursor::Hide,
    ExecutableCommand,
};
use std::time::{Duration, Instant};

use crossterm::style::Color;
use raytracer_maze::{
    Camera, CameraMode, Entity, EntityType, Framebuffer, GameRenderer, GameState, Pixel, Player,
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

fn main() {
    let mapa: Vec<Vec<u8>> = vec![
        vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        vec![1, 2, 0, 0, 0, 1, 0, 0, 0, 0, 4, 1],
        vec![1, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1],
        vec![1, 0, 1, 0, 0, 0, 0, 0, 1, 0, 1, 1],
        vec![1, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 1],
        vec![1, 1, 1, 0, 1, 0, 0, 0, 1, 1, 0, 1],
        vec![1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1],
        vec![1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 3, 1],
        vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
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
    let mut fps_counter = FpsCounter::new();

    // Pre-crear framebuffers para evitar allocaciones constantes
    let mut fb_topdown = Framebuffer::new(
        map_width * renderer.get_cell_width(),
        map_height * renderer.get_cell_height(),
    );
    let mut fb_firstperson = Framebuffer::new(cols, rows.saturating_sub(4));

    let entities = vec![
        Entity::new(3.5, 3.5, Pixel::new(Color::Magenta, '♦'), EntityType::Item),
        Entity::new(7.5, 1.5, Pixel::new(Color::Yellow, '☺'), EntityType::Enemy),
        Entity::new(5.5, 5.5, Pixel::new(Color::Cyan, '♣'), EntityType::Decoration),
    ];

    crossterm::terminal::enable_raw_mode().unwrap();
    let mut stdout = std::io::stdout();
    stdout.execute(Hide).unwrap();

    let target_frame_time = Duration::from_millis(16); // ~60 FPS
    let mut running = true;
    let mut first_render = true;

    while running {
        let frame_start = Instant::now();

        match game_state {
            GameState::Menu => {
                renderer.show_menu();

                if poll(Duration::from_millis(16)).unwrap() {
                    if let Ok(Event::Key(KeyEvent { code, .. })) = read() {
                        match code {
                            KeyCode::Enter => {
                                game_state = GameState::Playing;
                                first_render = true;
                            },
                            KeyCode::Char('x') | KeyCode::Esc => running = false,
                            _ => {}
                        }
                    }
                }
            }

            GameState::Playing => {
                if player.is_at_goal(&mapa) {
                    game_state = GameState::Victory;
                    continue;
                }

                // Solo renderizar si el jugador se movió o es el primer frame
                if player.has_moved || first_render {
                    match camera.mode {
                        CameraMode::TopDown => {
                            renderer.render_top_down(&mut fb_topdown, &mapa, &player, &entities);
                            renderer.display_framebuffer(&fb_topdown);
                        }
                        CameraMode::FirstPerson => {
                            renderer.render_first_person(&mut fb_firstperson, &player, &mapa);
                            renderer.display_framebuffer(&fb_firstperson);
                        }
                    }

                    let (px, py) = player.get_grid_position();
                    renderer.display_ui(
                        fps_counter.get_fps(),
                        player.position.x,
                        player.position.y,
                        px,
                        py,
                        camera.mode,
                    );

                    player.has_moved = false;
                    first_render = false;
                }

                fps_counter.update();

                if poll(Duration::from_millis(1)).unwrap() {
                    if let Ok(Event::Key(KeyEvent { code, .. })) = read() {
                        match code {
                            KeyCode::Char('w') | KeyCode::Up => player.move_forward(0.15, &mapa),
                            KeyCode::Char('s') | KeyCode::Down => player.move_backward(0.15, &mapa),
                            KeyCode::Char('a') | KeyCode::Left => player.strafe(-0.15, &mapa),
                            KeyCode::Char('d') | KeyCode::Right => player.strafe(0.15, &mapa),
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

                // Control de frame rate
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