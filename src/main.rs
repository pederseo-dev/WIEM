// ============================================================
// DEPENDENCIAS: traemos los módulos que vamos a usar
// ============================================================
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;
use tokio::sync::broadcast;

// ============================================================
// MAIN: punto de entrada del programa, corre el runtime async
// ============================================================
#[tokio::main]
async fn main() {
    // Canal de broadcast: el audio capturado se envía aquí
    // y todos los clientes WebSocket conectados lo reciben
    // capacidad 16 = cuántos chunks pueden estar en cola
    let (tx, _rx) = broadcast::channel::<Vec<f32>>(16);
    let tx = Arc::new(tx);

    // --------------------------------------------------------
    // CAPTURA DE AUDIO con cpal
    // --------------------------------------------------------
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("No se encontró dispositivo de entrada de audio");

    println!("Dispositivo de audio: {}", device.name().unwrap_or_default());

    let config = device
        .default_input_config()
        .expect("No se pudo obtener la configuración de audio");

    println!("Config de audio: {:?}", config);

    let tx_audio = tx.clone();

    // Construimos el stream de captura
    // cada vez que llegan muestras, las enviamos al canal broadcast
    let mut stream_config: cpal::StreamConfig = config.into();
    stream_config.buffer_size = cpal::BufferSize::Fixed(128);

    let stream = device
        .build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mono: Vec<f32> = data
                    .chunks(2)
                    .map(|par| (par[0] + par[1]) / 2.0)
                    .collect();
                // Imprimimos el tamaño del chunk para diagnosticar
                // println!("Chunk: {} muestras = {:.1}ms", mono.len(), mono.len() as f32 / 48.0);
                let _ = tx_audio.send(mono);
            },
            |err| eprintln!("Error en stream de audio: {}", err),
            None,
        )
        .expect("No se pudo construir el stream de audio");

    // Iniciamos la captura
    stream.play().expect("No se pudo iniciar la captura");

    // --------------------------------------------------------
    // SERVIDOR HTTP con axum
    // --------------------------------------------------------
    let tx_ws = tx.clone();

    let app = Router::new()
        // ruta raíz: sirve la página HTML del cliente
        .route("/", get(pagina_principal))
        // ruta websocket: aquí se conectan los clientes para recibir audio
        .route(
            "/ws",
            get(move |ws: WebSocketUpgrade| {
                let tx = tx_ws.clone();
                async move { ws.on_upgrade(move |socket| manejar_websocket(socket, tx)) }
            }),
        );

    // Escuchamos en todas las interfaces para que otros dispositivos
    // en la red WiFi puedan conectarse usando la IP de esta PC
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("No se pudo iniciar el servidor");

    println!("Servidor corriendo en http://0.0.0.0:3000");
    println!("Conectate desde otro dispositivo usando la IP de esta PC");

    axum::serve(listener, app)
        .await
        .expect("Error en el servidor");
}

// ============================================================
// PÁGINA PRINCIPAL: devuelve el HTML del cliente de audio
// ============================================================
async fn pagina_principal() -> impl IntoResponse {
    Html(include_str!("index.html"))
}

// ============================================================
// MANEJADOR DE WEBSOCKET: recibe un cliente y le envía audio
// ============================================================
async fn manejar_websocket(mut socket: WebSocket, tx: Arc<broadcast::Sender<Vec<f32>>>) {
    // Nos suscribimos al canal para recibir chunks de audio
    let mut rx = tx.subscribe();

    loop {
        // Esperamos el próximo chunk de audio del canal
        match rx.recv().await {
            Ok(chunk) => {
                // Convertimos los f32 a bytes para enviar por WebSocket
                let bytes: Vec<u8> = chunk
                    .iter()
                    .flat_map(|sample| sample.to_le_bytes())
                    .collect();

                // Enviamos como mensaje binario al cliente
                if socket.send(Message::Binary(bytes.into())).await.is_err() {
                    // El cliente se desconectó, salimos del loop
                    break;
                }
            }
            Err(_) => {
                // El canal se cerró, salimos
                break;
            }
        }
    }
}