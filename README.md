# Guía de Ejecución – RoomRTC

Instrucciones para levantar el servidor de señalización y el cliente GUI.

## 1. Prerrequisitos
- Rust toolchain instalado.
- Dependencias nativas de OpenCV/OpenH264.
- Archivos de configuración (opcionales): `server.conf`, `client.conf` (se usan defaults si no existen).

## 2. Servidor de señalización
1. Ubícate en `RoomRTC/`.
2. Revisa/ajusta `server.conf` (opcional):
   ```
   server_addr=0.0.0.0:8443
   users_file=users.txt
   log_file=roomrtc.log
   max_clients=100
   ```
3. Ejecuta:
   ```bash
   cargo run --bin signaling_server -- server.conf
   ```
   - Genera un cert TLS self-signed en caliente.
   - Crea `users.txt` si no existe.
   - Muestra puerto, archivo de usuarios y máximo de clientes.

## 3. Cliente RoomRTC (GUI)
1. Ubícate en `RoomRTC/`.
2. Opcional: ajusta `client.conf` (servidor, cámara, resolución, fps):
   ```
   server_addr=127.0.0.1:8443
   video_width=1280
   video_height=720
   video_fps=30
   ```
3. Ejecuta:
   ```bash
   cargo run --bin roomrtc -- client.conf
   ```
4. Flujo típico:
   - Login: ingresa user/pass; puedes registrar y luego loguear.
   - Lobby: refresca usuarios; inicia llamada con “VideoCall”.
   - Waiting/Join: intercambia SDP/ICE automáticamente; espera conexión ICE+DTLS.
   - VideoCall: previsualización local/remota, métricas y botón de “Hang up”.

## 4. Notas
- La señalización viaja por TLS con cert self-signed (el cliente acepta por verificador inseguro).
- SRTP se activa si el handshake DTLS completa; de lo contrario, el tráfico va en claro.
- Logs: servidor y cliente escriben en `roomrtc.log` (configurable en cada conf).
# 25C2-cargo-y-descargo-calandria
