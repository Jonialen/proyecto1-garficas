/// Enumera los posibles modos de la cámara en el juego.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CameraMode {
    /// Vista desde arriba, mostrando el mapa completo.
    TopDown,
    /// Vista en primera persona, desde la perspectiva del jugador.
    FirstPerson,
}

/// Representa la cámara del juego, que controla la perspectiva de visualización.
pub struct Camera {
    /// El modo actual de la cámara (TopDown o FirstPerson).
    pub mode: CameraMode,
    /// El nivel de zoom de la cámara, aplicable en ciertos modos.
    pub zoom: f32,
}

impl Camera {
    /// Crea una nueva cámara con valores predeterminados.
    pub fn new() -> Self {
        Self {
            mode: CameraMode::TopDown, // Inicia en modo TopDown.
            zoom: 1.0, // Sin zoom inicial.
        }
    }

    /// Cambia entre los modos de cámara disponibles.
    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            CameraMode::TopDown => CameraMode::FirstPerson,
            CameraMode::FirstPerson => CameraMode::TopDown,
        };
    }
}
