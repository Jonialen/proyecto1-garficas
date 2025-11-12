/// Enumera los diferentes estados en los que puede encontrarse el juego.
/// Utilizado para controlar la lógica principal y el renderizado.
#[derive(PartialEq, Debug)]
pub enum GameState {
    Menu,        // Muestra el menú principal.
    LevelSelect, // Muestra la pantalla de selección de nivel.
    Playing,     // El juego está en curso.
    Victory,     // Muestra la pantalla de victoria al completar todos los niveles.
}
