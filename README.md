# Laberinto de Recolección en Terminal

Un juego de laberintos en 3D renderizado en la terminal, construido con Rust. El jugador debe navegar por diferentes laberintos, recolectar todos los ítems y llegar a la meta para avanzar al siguiente nivel.

## Características

- **Motor de Renderizado 3D en la Terminal**: Utiliza raycasting para crear una perspectiva en primera persona directamente en la terminal.
- **Doble Resolución Vertical**: Cada celda de la terminal muestra dos píxeles usando el carácter `▀` (color de primer plano arriba, color de fondo abajo).
- **Color de 24 bits e Iluminación**: Niebla exponencial según la distancia, sombreado distinto para las caras norte/sur y un cielo que se funde con el horizonte.
- **Texturas Procedurales**: Paredes de ladrillo y suelo de baldosas generados en tiempo real (floor casting), sin archivos de imagen.
- **Objetos en 3D**: Los ítems son diamantes que giran y flotan, y la meta es una esfera que pulsa; ambos con iluminación, sombra en el suelo y z-buffer.
- **Efectos**: Destello dorado al recoger un ítem, balanceo de cámara al caminar y transiciones con fundido entre niveles.
- **Minimapa**: Semitransparente, con desplazamiento suave centrado en el jugador.
- **Enemigos**: Fantasmas que deambulan por el laberinto y te persiguen cuando te ven. Tienes 3 vidas por nivel; cada golpe te empuja y te da un momento de invulnerabilidad, y al perderlas todas el nivel se reinicia.
- **Música y Sonido**: Música chiptune y efectos de sonido sintetizados en tiempo real, sin archivos de audio.
- **Doble Cámara**: Cambia entre una vista en primera persona y una vista cenital (top-down) iluminada por la linterna del jugador, con sombras y línea de visión.
- **Múltiples Niveles**: Incluye 3 niveles de dificultad creciente.
- **Interfaz de Usuario en Terminal**: Menú, selección de nivel y pantalla de victoria con títulos animados en letras de bloque.
- **Controles Intuitivos**: Movimiento y rotación estándar para una fácil navegación.

## Cómo Jugar

### Prerrequisitos

- [Rust](https://www.rust-lang.org/tools/install)
- Una terminal con soporte de color de 24 bits (truecolor), por ejemplo Ghostty, Kitty, Alacritty, WezTerm o Foot.
- En Linux, la biblioteca de desarrollo de ALSA para compilar el audio (`alsa-lib` en Arch, `libasound2-dev` en Debian/Ubuntu, `alsa-lib-devel` en Fedora). Si no hay dispositivo de audio, el juego funciona en silencio.

### Compilación y Ejecución

1. Clona el repositorio:
   ```bash
   git clone https://github.com/Jonialen/proyecto1-garficas.git
   cd proyecto1-garficas
   ```

2. Compila y ejecuta el proyecto con Cargo:
   ```bash
   cargo run --release
   ```

### Controles

- **Movimiento**: `WASD` o las teclas de flecha (`↑` `↓` `←` `→`).
- **Rotación de la Cámara**: `Q` y `E`.
- **Cambiar Vista de Cámara**: `C` (entre primera persona y cenital).
- **Silenciar Audio**: `M`.
- **Salir del Juego**: `X` o `ESC`.

## Estructura del Proyecto

El proyecto está organizado en los siguientes módulos principales dentro de `src/`:

- **`main.rs`**: El punto de entrada de la aplicación. Contiene el bucle principal del juego, la gestión de estados y el manejo de eventos.
- **`lib.rs`**: El punto de entrada de la biblioteca del juego, que exporta los módulos `audio`, `game`, `graphics` y `math`.
- **`game/`**: Contiene la lógica y las estructuras de datos del juego.
  - `player.rs`: Define al jugador, su movimiento y estado.
  - `level.rs`: Gestiona la estructura de los niveles, el mapa y los ítems.
  - `camera.rs`: Controla la cámara del juego y sus modos.
  - `enemy.rs`: Define a los enemigos y su comportamiento (deambular, perseguir y buscar al jugador).
  - `state.rs`: Define los diferentes estados del juego (menú, jugando, etc.).
  - `entity.rs`: (Actualmente en desuso) Estructura para futuras entidades en el juego.
- **`graphics/`**: Se encarga de todo el renderizado en la terminal.
  - `renderer.rs`: El motor de renderizado principal. Dibuja el mundo 3D (paredes, suelo, objetos y efectos), el mapa 2D, los menús y la UI.
  - `framebuffer.rs`: Abstracción de un búfer de píxeles para dibujar en la terminal.
  - `pixel.rs`: Representa un único carácter con color en la terminal.
- **`audio/`**: Sintetiza la música y los efectos de sonido con `rodio`.
- **`math/`**: Proporciona herramientas matemáticas básicas.
  - `mod.rs`: Incluye una estructura `Vec2` para operaciones vectoriales en 2D.

[![Ver video en YouTube](https://img.youtube.com/vi/xKysX23nUzs/maxresdefault.jpg)](https://www.youtube.com/watch?v=xKysX23nUzs)


