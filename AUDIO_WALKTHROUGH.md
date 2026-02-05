# Audio Implementation Walkthrough

## Resumen

Se implementó transmisión de audio bidireccional en tiempo real usando:
- **rodio**: Reproducción de audio robusta (compatible con PipeWire/ALSA).
- **cpal**: Captura de micrófono (input).
- **audiopus**: Encoding/decoding Opus (20ms frames, 48kHz).

## Arquitectura

```
┌─────────────────────────────────────────────────────────────────┐
│                          WorkerAudio                             │
├─────────────────────────────────────────────────────────────────┤
│  Micrófono (cpal) → AudioCapture → OpusEncoder → RTP (SSRC 2000)│
│                                                                  │
│  RTP Listener → Decoder → AudioPlayback (rodio) → Parlantes      │
└─────────────────────────────────────────────────────────────────┘
```

## Solución de Problemas Encontrados

### 1. Audio Playback en Linux (PipeWire)
**Problema**: `cpal` no ejecutaba los callbacks de audio en algunos sistemas con PipeWire, causando que el buffer se llenara sin reproducirse.
**Solución**: Migración a `rodio`, que maneja mejor la abstracción de backend y funcionó correctamente.

### 2. "Playback disconnected"
**Problema**: El objeto `AudioPlayback` era droppeado inmediatamente después de crearse en `WorkerAudio`, cerrando el canal de comunicación.
**Solución**: Se agregó el campo `playback` a la estructura `WorkerAudio` para mantener vivo el objeto durante toda la llamada.

### 3. Routing de Paquetes
**Problema**: El listener no sabía dónde enviar los paquetes de audio.
**Solución**: Implementación de routing por SSRC en `P2PClient`:
- SSRC 1000 -> Video
- SSRC 2000 -> Audio

## Archivos Clave

| Archivo | Rol |
|---------|-----|
| `webrtc/src/audio/audio_playback.rs` | Reproducción usando `rodio` |
| `webrtc/src/worker_thread/worker_audio.rs` | Orquestador de captura, encoding y playback |
| `RoomRTC/src/client/p2p_client.rs` | Routing RTP y manejo de conexión |

## Cómo Probar

```bash
cd /home/alenk/25C2-cargo-y-descargo-calandria
cargo run --release --bin roomrtc
```

1. Iniciar en dos terminales/PCs.
2. Ingresar mismos credenciales.
3. El audio inicia automáticamente (verifica permisos de micrófono).
4. Botón 🎤 para mutear/desmutear.
