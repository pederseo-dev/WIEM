# WIEM — WiFi In-Ear Monitor

Sistema de monitoreo de audio en tiempo real por red local. Captura audio desde una interfaz de audio o micrófono conectado a la PC y lo transmite vía WebSocket a cualquier dispositivo en la misma red WiFi, sin necesidad de internet. Diseñado para baja latencia usando Rust con WASAPI en Windows.

## Commands
```shell
# install dependencies and compile
cargo build

# run project
cargo run

# run with compiler optimizations (recommended)
cargo run --release
```

## Mixer Configuration
Los mapeos de consolas se definen en `config/mixers.json`. Si la consola conectada está en el JSON, el servidor muestra solo sus aux configurados. Si no está, muestra todos los dispositivos de Windows como fallback.

Para agregar una consola nueva:
1. Instalá su driver en Windows
2. Ejecutá el servidor y anotá el nombre exacto que aparece en consola
3. Agregá la clave con ese nombre en `mixers.json` y mapeá sus canales

## Supported Mixers
- Soundcraft UI24R (Aux 1-8)
