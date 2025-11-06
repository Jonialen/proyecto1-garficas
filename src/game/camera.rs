#[derive(PartialEq, Debug)]
pub enum CameraMode {
    TopDown,
    FirstPerson,
}

pub struct Camera {
    pub mode: CameraMode,
    pub zoom: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            mode: CameraMode::TopDown,
            zoom: 1.0,
        }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            CameraMode::TopDown => CameraMode::FirstPerson,
            CameraMode::FirstPerson => CameraMode::TopDown,
        };
    }
}
