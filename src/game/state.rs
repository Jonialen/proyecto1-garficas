#[derive(PartialEq, Debug)]
pub enum GameState {
    Menu,
    LevelSelect,
    Playing,
    Victory,
}