# Laberinto de Recolección en Terminal

Un juego de laberintos en 3D renderizado en la terminal, construido con Rust. El jugador debe navegar por diferentes laberintos, recolectar todos los ítems y llegar a la meta para avanzar al siguiente nivel.

## Características

- **Motor de Renderizado 3D en la Terminal**: Utiliza raycasting para crear una perspectiva en primera persona directamente en la terminal.
- **Doble Cámara**: Cambia entre una vista en primera persona y una vista cenital (top-down) para navegar por el laberinto.
- **Múltiples Niveles**: Incluye 3 niveles de dificultad creciente.
- **Interfaz de Usuario en Terminal**: Menús, selección de nivel y pantalla de victoria renderizados con `crossterm`.
- **Controles Intuitivos**: Movimiento y rotación estándar para una fácil navegación.

## Cómo Jugar

### Prerrequisitos

- [Rust](https://www.rust-lang.org/tools/install)

### Compilación y Ejecución

1. Clona el repositorio:
   ```bash
   git clone https://github.com/tu_usuario/raytracer-maze.git
   cd raytracer-maze
   ```

2. Compila y ejecuta el proyecto con Cargo:
   ```bash
   cargo run --release
   ```

### Controles

- **Movimiento**: `WASD` o las teclas de flecha (`↑` `↓` `←` `→`).
- **Rotación de la Cámara**: `Q` y `E`.
- **Cambiar Vista de Cámara**: `C` (entre primera persona y cenital).
- **Salir del Juego**: `X` o `ESC`.

## Estructura del Proyecto

El proyecto está organizado en los siguientes módulos principales dentro de `src/`:

- **`main.rs`**: El punto de entrada de la aplicación. Contiene el bucle principal del juego, la gestión de estados y el manejo de eventos.
- **`lib.rs`**: El punto de entrada de la biblioteca del juego, que exporta los módulos `game`, `graphics` y `math`.
- **`game/`**: Contiene la lógica y las estructuras de datos del juego.
  - `player.rs`: Define al jugador, su movimiento y estado.
  - `level.rs`: Gestiona la estructura de los niveles, el mapa y los ítems.
  - `camera.rs`: Controla la cámara del juego y sus modos.
  - `state.rs`: Define los diferentes estados del juego (menú, jugando, etc.).
  - `entity.rs`: (Actualmente en desuso) Estructura para futuras entidades en el juego.
- **`graphics/`**: Se encarga de todo el renderizado en la terminal.
  - `renderer.rs`: El motor de renderizado principal. Dibuja el mundo 3D, el mapa 2D, los menús y la UI.
  - `framebuffer.rs`: Abstracción de un búfer de píxeles para dibujar en la terminal.
  - `pixel.rs`: Representa un único carácter con color en la terminal.
- **`math/`**: Proporciona herramientas matemáticas básicas.
  - `mod.rs`: Incluye una estructura `Vec2` para operaciones vectoriales en 2D.

[![Ver video en YouTube](https://img.youtube.com/vi/xKysX23nUzs/maxresdefault.jpg)](https://www.youtube.com/watch?v=xKysX23nUzs)


