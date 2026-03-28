// ============================================================
// DEPENDENCIAS
// ============================================================

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path,
    },
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

// ============================================================
// ESTRUCTURAS: definen el formato del JSON de configuración
// ============================================================
#[derive(serde::Deserialize, Clone)]
struct CanalConfig {
    nombre: String,
    dispositivo: String,
    canal: u16,
}

#[derive(serde::Deserialize, Clone)]
struct ConsolaConfig {
    consola: String,
    canales: Vec<CanalConfig>,
}

// ============================================================
// ESTRUCTURA: canal disponible que enviamos al frontend
// ============================================================
#[derive(serde::Serialize, Clone)]
struct CanalDisponible {
    id: usize,
    nombre: String,
}

// ============================================================
// ESTADO GLOBAL
// ============================================================
struct EstadoApp {
    canales: Vec<CanalDisponible>,
    transmisores: HashMap<usize, broadcast::Sender<Vec<f32>>>,
}

// ============================================================
// MAIN
// ============================================================
#[tokio::main]
async fn main() {



    let host = cpal::default_host();

    // --------------------------------------------------------
    // CARGAMOS el archivo de configuración de consolas
    // --------------------------------------------------------
    let mixers_json = std::fs::read_to_string("config/mixers.json")
        .expect("No se pudo leer config/mixers.json");

    let mixers: HashMap<String, ConsolaConfig> = serde_json::from_str(&mixers_json)
        .expect("Error al parsear config/mixers.json");

    // --------------------------------------------------------
    // DETECTAMOS todos los dispositivos de entrada disponibles
    // --------------------------------------------------------
    let dispositivos_cpal: Vec<cpal::Device> = host
        .input_devices()
        .expect("No se pudieron listar los dispositivos")
        .collect();
    // TEMPORAL: ver nombres exactos de los dispositivos
    for device in dispositivos_cpal.iter() {
        println!("Nombre exacto: '{}'", device.name().unwrap_or_default());
    }
    // Buscamos si algún dispositivo conectado coincide con una consola conocida
    let mut consola_detectada: Option<ConsolaConfig> = None;

    for device in dispositivos_cpal.iter() {
        let nombre = device.name().unwrap_or_default();
        if let Some(config) = mixers.iter().find(|(clave, _)| nombre.contains(clave.as_str())).map(|(_, v)| v) {
            println!("Consola detectada: {}", config.consola);
            consola_detectada = Some(config.clone());
            break;
        }
    }

    // --------------------------------------------------------
    // CONSTRUIMOS el mapa de transmisores según la configuración
    // --------------------------------------------------------
    let mut canales_disponibles: Vec<CanalDisponible> = Vec::new();
    let mut transmisores: HashMap<usize, broadcast::Sender<Vec<f32>>> = HashMap::new();
    let mut streams: Vec<cpal::Stream> = Vec::new();

    match consola_detectada {
        Some(config) => {
            // Agrupamos los canales por dispositivo para abrir un solo stream por dispositivo
            let mut canales_por_dispositivo: HashMap<String, Vec<(usize, CanalConfig)>> = HashMap::new();

            for (id, canal_config) in config.canales.iter().enumerate() {
                canales_por_dispositivo
                    .entry(canal_config.dispositivo.clone())
                    .or_default()
                    .push((id, canal_config.clone()));

                canales_disponibles.push(CanalDisponible {
                    id,
                    nombre: canal_config.nombre.clone(),
                });

                let (tx, _rx) = broadcast::channel::<Vec<f32>>(8);
                transmisores.insert(id, tx);
            }

            // Ordenamos los canales disponibles por id para que aparezcan en orden
            canales_disponibles.sort_by_key(|c| c.id);

            // Abrimos un stream por dispositivo
            for (nombre_dispositivo, canales) in canales_por_dispositivo.iter() {
                let device = dispositivos_cpal
                    .iter()
                    .find(|d| d.name().unwrap_or_default() == *nombre_dispositivo);

                let device = match device {
                    Some(d) => d,
                    None => {
                        println!("No se encontró dispositivo: {}", nombre_dispositivo);
                        continue;
                    }
                };

                let cpal_config = match device.default_input_config() {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                let canales_totales = cpal_config.channels() as usize;

                // Construimos un mapa canal_idx -> transmisor para este dispositivo
                let mut tx_por_canal: HashMap<usize, broadcast::Sender<Vec<f32>>> = HashMap::new();
                for (id, canal_config) in canales.iter() {
                    if let Some(tx) = transmisores.get(id) {
                        tx_por_canal.insert(canal_config.canal as usize, tx.clone());
                    }
                }

                let mut stream_config: cpal::StreamConfig = cpal_config.into();
                stream_config.buffer_size = cpal::BufferSize::Fixed(256);

                let stream = device
                    .build_input_stream(
                        &stream_config,
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            // Para cada canal que nos interesa extraemos sus muestras
                            for (canal_idx, tx) in tx_por_canal.iter() {
                                let muestras: Vec<f32> = data
                                    .iter()
                                    .skip(*canal_idx)
                                    .step_by(canales_totales)
                                    .copied()
                                    .collect();
                                let _ = tx.send(muestras);
                            }
                        },
                        |err| eprintln!("Error en stream: {}", err),
                        None,
                    )
                    .expect("No se pudo construir el stream");

                stream.play().expect("No se pudo iniciar el stream");
                streams.push(stream);

                println!("Stream abierto para: {}", nombre_dispositivo);
            }
        }
        None => {
            // Consola desconocida: mostramos todos los dispositivos disponibles
            println!("Ninguna consola conocida detectada, mostrando todos los dispositivos");

            for (id, device) in dispositivos_cpal.iter().enumerate() {
                let nombre = device.name().unwrap_or_else(|_| "Desconocido".to_string());

                let cpal_config = match device.default_input_config() {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                let canales_totales = cpal_config.channels() as usize;
                let (tx, _rx) = broadcast::channel::<Vec<f32>>(16);
                transmisores.insert(id, tx.clone());

                canales_disponibles.push(CanalDisponible {
                    id,
                    nombre: nombre.clone(),
                });

                let mut stream_config: cpal::StreamConfig = cpal_config.into();
                stream_config.buffer_size = cpal::BufferSize::Fixed(256);

                let stream = device
                    .build_input_stream(
                        &stream_config,
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            // Mezclamos todos los canales a mono
                            let mono: Vec<f32> = data
                                .chunks(canales_totales)
                                .map(|frame| frame.iter().sum::<f32>() / canales_totales as f32)
                                .collect();
                            let _ = tx.send(mono);
                        },
                        |err| eprintln!("Error en stream: {}", err),
                        None,
                    )
                    .expect("No se pudo construir el stream");

                stream.play().expect("No se pudo iniciar el stream");
                streams.push(stream);
            }
        }
    }

    // --------------------------------------------------------
    // ESTADO COMPARTIDO
    // --------------------------------------------------------
    let estado = Arc::new(RwLock::new(EstadoApp {
        canales: canales_disponibles,
        transmisores,
    }));

    // --------------------------------------------------------
    // SERVIDOR HTTP
    // --------------------------------------------------------
    let estado_canales = estado.clone();
    let estado_ws = estado.clone();

    let app = Router::new()
        .route("/", get(pagina_principal))
        .route(
            "/canales",
            get(move || {
                let estado = estado_canales.clone();
                async move {
                    let estado = estado.read().await;
                    Json(estado.canales.clone())
                }
            }),
        )
        .route(
            "/ws/{canal_id}",
            get(move |
                Path(canal_id): Path<usize>,
                ws: WebSocketUpgrade,
            | {
                let estado = estado_ws.clone();
                async move {
                    let tx = {
                        let estado = estado.read().await;
                        estado.transmisores.get(&canal_id).cloned()
                    };

                    match tx {
                        Some(tx) => ws.on_upgrade(move |socket| {
                            manejar_websocket(socket, tx)
                        }),
                        None => ws.on_upgrade(|mut socket| async move {
                            let _ = socket.send(Message::Text("Canal no encontrado".into())).await;
                        }),
                    }
                }
            }),
        );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("No se pudo iniciar el servidor");

    println!("Servidor corriendo en http://0.0.0.0:3000");

    let _streams = streams;

    axum::serve(listener, app)
        .await
        .expect("Error en el servidor");
}

// ============================================================
// PÁGINA PRINCIPAL
// ============================================================
async fn pagina_principal() -> impl IntoResponse {
    Html(include_str!("index.html"))
}

// ============================================================
// MANEJADOR DE WEBSOCKET
// ============================================================
async fn manejar_websocket(mut socket: WebSocket, tx: broadcast::Sender<Vec<f32>>) {
    let mut rx = tx.subscribe();

    loop {
        match rx.recv().await {
            Ok(chunk) => {
                let bytes: Vec<u8> = chunk
                    .iter()
                    .flat_map(|sample| sample.to_le_bytes())
                    .collect();

                if socket.send(Message::Binary(bytes.into())).await.is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}