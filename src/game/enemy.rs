use crate::math::Vec2;

/// Radio de colisión del enemigo contra las paredes.
const ENEMY_RADIUS: f32 = 0.25;
/// Velocidad al deambular, en unidades por segundo.
const WANDER_SPEED: f32 = 0.9;
/// Velocidad al perseguir al jugador, en unidades por segundo.
const CHASE_SPEED: f32 = 1.7;
/// Distancia máxima a la que el enemigo puede ver al jugador.
const SIGHT_RANGE: f32 = 6.0;
/// Distancia a la que el enemigo golpea al jugador.
pub const ENEMY_HIT_RADIUS: f32 = 0.45;

/// Estado de comportamiento del enemigo.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EnemyState {
    /// Recorre los pasillos sin objetivo.
    Wander,
    /// Persigue al jugador o va hacia el último lugar donde lo vio.
    Chase,
}

/// Un enemigo que deambula por el laberinto y persigue al jugador cuando lo ve.
pub struct Enemy {
    /// Posición en el mundo.
    pub position: Vec2,
    /// Comportamiento actual.
    pub state: EnemyState,
    heading: Vec2,
    last_seen: Option<Vec2>,
    turn_timer: f32,
    rng: u32,
}

impl Enemy {
    /// Crea un enemigo en la posición indicada; `seed` hace que cada uno deambule distinto.
    pub fn new(x: f32, y: f32, seed: u32) -> Self {
        let mut enemy = Self {
            position: Vec2::new(x, y),
            state: EnemyState::Wander,
            heading: Vec2::new(1.0, 0.0),
            last_seen: None,
            turn_timer: 0.0,
            rng: seed.wrapping_mul(0x9e37_79b9) | 1,
        };
        enemy.pick_new_heading();
        enemy
    }

    /// Avanza la simulación `dt` segundos.
    pub fn update(&mut self, dt: f32, player: Vec2, map: &[Vec<u8>]) {
        if can_see(self.position, player, map) {
            self.state = EnemyState::Chase;
            self.last_seen = Some(player);
        }

        match (self.state, self.last_seen) {
            (EnemyState::Chase, Some(target)) => {
                let to_target = Vec2::new(target.x - self.position.x, target.y - self.position.y);
                if to_target.length() < 0.1 {
                    // Llegó al último lugar donde vio al jugador y no lo encontró.
                    self.state = EnemyState::Wander;
                    self.last_seen = None;
                    self.pick_new_heading();
                } else {
                    self.heading = to_target.normalize();
                    self.step(CHASE_SPEED * dt, map);
                }
            }
            _ => {
                self.turn_timer -= dt;
                if !self.step(WANDER_SPEED * dt, map) || self.turn_timer <= 0.0 {
                    self.pick_new_heading();
                }
            }
        }
    }

    /// Mueve al enemigo en su dirección, deslizándose por las paredes.
    /// Devuelve `false` si quedó completamente bloqueado.
    fn step(&mut self, distance: f32, map: &[Vec<u8>]) -> bool {
        let next_x = Vec2::new(self.position.x + self.heading.x * distance, self.position.y);
        let moved_x = is_free(next_x, map);
        if moved_x {
            self.position = next_x;
        }
        let next_y = Vec2::new(self.position.x, self.position.y + self.heading.y * distance);
        let moved_y = is_free(next_y, map);
        if moved_y {
            self.position = next_y;
        }
        moved_x || moved_y
    }

    /// Elige una dirección cardinal al azar y cuánto tiempo mantenerla.
    fn pick_new_heading(&mut self) {
        let directions = [Vec2::new(1.0, 0.0), Vec2::new(-1.0, 0.0), Vec2::new(0.0, 1.0), Vec2::new(0.0, -1.0)];
        self.heading = directions[(self.next_random() * 4.0) as usize % 4];
        self.turn_timer = 1.5 + self.next_random() * 2.5;
    }

    /// Número pseudoaleatorio en [0, 1) (xorshift).
    fn next_random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / 16_777_216.0
    }
}

/// Indica si un círculo de radio `ENEMY_RADIUS` en `pos` no toca ninguna pared.
fn is_free(pos: Vec2, map: &[Vec<u8>]) -> bool {
    [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)].iter().all(|(dx, dy)| {
        let x = pos.x + dx * ENEMY_RADIUS;
        let y = pos.y + dy * ENEMY_RADIUS;
        x >= 0.0 && y >= 0.0 && map.get(y as usize).and_then(|row| row.get(x as usize)).is_some_and(|&c| c != 1)
    })
}

/// Indica si hay línea de visión directa entre dos puntos, dentro del alcance de visión.
fn can_see(from: Vec2, to: Vec2, map: &[Vec<u8>]) -> bool {
    let delta = Vec2::new(to.x - from.x, to.y - from.y);
    let distance = delta.length();
    if distance > SIGHT_RANGE {
        return false;
    }
    let steps = (distance / 0.1).ceil() as usize;
    (1..steps).all(|i| {
        let t = i as f32 / steps as f32;
        let (x, y) = (from.x + delta.x * t, from.y + delta.y * t);
        map.get(y as usize).and_then(|row| row.get(x as usize)).is_some_and(|&c| c != 1)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corridor() -> Vec<Vec<u8>> {
        vec![
            vec![1, 1, 1, 1, 1, 1, 1],
            vec![1, 0, 0, 0, 0, 0, 1],
            vec![1, 1, 1, 0, 1, 1, 1],
            vec![1, 0, 0, 0, 0, 0, 1],
            vec![1, 1, 1, 1, 1, 1, 1],
        ]
    }

    #[test]
    fn chases_visible_player() {
        let map = corridor();
        let mut enemy = Enemy::new(1.5, 1.5, 7);
        let player = Vec2::new(5.5, 1.5);
        let start = enemy.position.x;
        enemy.update(0.5, player, &map);
        assert_eq!(enemy.state, EnemyState::Chase);
        assert!(enemy.position.x > start, "should move toward the player");
    }

    #[test]
    fn does_not_see_through_walls() {
        let map = corridor();
        let mut enemy = Enemy::new(1.5, 1.5, 7);
        enemy.update(0.01, Vec2::new(1.5, 3.5), &map);
        assert_eq!(enemy.state, EnemyState::Wander);
    }

    #[test]
    fn never_enters_walls() {
        let map = corridor();
        let mut enemy = Enemy::new(3.5, 1.5, 42);
        for _ in 0..2_000 {
            enemy.update(0.05, Vec2::new(100.0, 100.0), &map);
            assert!(is_free(enemy.position, &map), "enemy inside a wall at {:?}", (enemy.position.x, enemy.position.y));
        }
    }
}
