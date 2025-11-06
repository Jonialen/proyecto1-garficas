use crossterm::{
    event::{Event, KeyCode, KeyEvent, poll, read},
    terminal::size,
};
use std::time::Duration;

use crossterm::style::Color;
use raytracer_maze::{
    Camera, CameraMode, Entity, EntityType, Framebuffer, GameRenderer, GameState, Pixel, Player,
};

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
            }

            GameState::Playing => {
                if player.is_at_goal(&mapa) {
                    game_state = GameState::Victory;
                    continue;
                }

                // no re crees el framebuffer
                // if hasMove
                let mut framebuffer = match camera.mode {
                    CameraMode::TopDown => Framebuffer::new(
                        map_width * renderer.get_cell_width(),
                        map_height * renderer.get_cell_height(),
                    ),
                    CameraMode::FirstPerson => Framebuffer::new(cols, rows.saturating_sub(4)),
                };

                match camera.mode {
                    CameraMode::TopDown => {
                        renderer.render_top_down(&mut framebuffer, &mapa, &player, &entities);
                    }
                    CameraMode::FirstPerson => {
                        renderer.render_first_person(&mut framebuffer, &player, &mapa);
                    }
                }

                player.hasMove = true;

                renderer.display_framebuffer(&framebuffer);

                let (px, py) = player.get_grid_position();
                println!("\n[CONTROLS] WASD:move, QE:rotate, C:camera mode, X:quit");
                println!(
                    "\n[DEBUG] Player: ({:.1}, {:.1}) grid: ({}, {}), Camera: {:?}",
                    player.position.x, player.position.y, px, py, camera.mode
                );
                println!("\n[MINIMAP] Visible en modo First Person (esquina superior derecha)");

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
    println!("¡Gracias por jugar!");
}
