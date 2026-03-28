¡Claro! Aquí tienes un **README breve y claro** con el esquema y arquitectura que hemos discutido hasta ahora:

---

# IEM Wi-Fi usando WebRTC – MVP

Este proyecto propone un **sistema IEM (In-Ear Monitor) usando WebRTC** para transmitir audio desde una computadora a múltiples celulares vía Wi-Fi, sin necesidad de internet ni apps externas.

---

## 🔹 Arquitectura

```
🎚 Consola de audio
       │
       ▼
   PC (emisor)
   ├─ Captura AUX USB
   ├─ Página web / servidor local
   ├─ WebRTC + Opus
   └─ Wi-Fi local
       │
       ▼
📱 Celulares (receptores)
   ├─ Escanean QR de la página
   ├─ Abren navegador (Chrome/Safari)
   └─ Reproducen audio en tiempo real
```

---

## 🔹 Componentes clave

| Componente          | Función                                                                              |
| ------------------- | ------------------------------------------------------------------------------------ |
| **PC / Página web** | Captura audio, crea PeerConnection WebRTC, codifica en Opus y transmite a receptores |
| **Celulares**       | Conectan al WebRTC del PC, decodifican Opus y reproducen audio                       |
| **WebRTC**          | Maneja transmisión en tiempo real, jitter buffer, NAT traversal, y baja latencia     |
| **Opus**            | Códec de audio de baja latencia, optimizado para voz y música en tiempo real         |
| **Wi-Fi local**     | Red de transmisión, suficiente para latencia < 40 ms en LAN                          |

---

## 🔹 Flujo de audio

1. Consola → AUX USB → PC
2. PC → captura audio → WebRTC + Opus → Wi-Fi local
3. Celular → navegador → WebRTC → decodifica Opus → altavoz

---

## 🔹 Beneficios

* Latencia baja (~20–40 ms en Wi-Fi LAN)
* No requiere Internet ni apps externas
* Multi-receptor (varios celulares al mismo tiempo)
* Escalable y portable

---

## 🔹 Próximos pasos

* Captura de múltiples canales AUX y mezcla por usuario
* Control de volumen individual en los celulares
* Automatizar señalización (sin copy-paste) usando WebSocket o Firebase
* Optimización de buffers y frames Opus para latencia mínima

---
