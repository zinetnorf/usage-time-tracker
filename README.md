# Usage Tracker

Aplicación de escritorio que mide cuánto tiempo usas cada aplicación. Corre en segundo plano desde la bandeja del sistema, detecta la ventana en primer plano y distingue entre **tiempo activo** (con teclado/mouse) y **tiempo inactivo**. Multiplataforma: Windows 11 y macOS.

## 🔒 100% local, cero red

**El diferenciador de esta app: tus datos nunca salen de tu equipo.**

- Sin telemetría, sin cuentas, sin nube, sin llamadas de red de ningún tipo.
- La base de datos (SQLite) vive en tu máquina y los reportes se generan y consultan solo ahí.
- El código es abierto: puedes verificarlo.

| Plataforma | Ubicación de los datos |
|---|---|
| macOS | `~/Library/Application Support/com.davisg.usage-tracker/usage.db` |
| Windows | `%APPDATA%\com.davisg.usage-tracker\usage.db` |

## Características

- **Tracking automático** de la app en primer plano, con muestreo cada 1.5 s (configurable).
- **Activo vs. inactivo:** tras N segundos sin input (60 por defecto) el tiempo cuenta como inactivo; tú decides si suma en los reportes.
- **Pausa inteligente:** al bloquear la sesión o suspender el equipo el conteo se detiene; el hueco no se rellena.
- **Dashboard** con 4 vistas:
  - **Hoy:** total del día + gráfico por app + lista activo/inactivo.
  - **Histórico:** tendencia de 7/14/30 días + desglose por app.
  - **Apps:** renombrar, fusionar entradas (p. ej. `Code.exe` → "VS Code") y excluir apps del tracking (blacklist).
  - **Ajustes:** umbral de inactividad, retención, autostart, idioma…
- **Export a PDF:** la vista actual o un reporte completo (las apps excluidas no aparecen en los reportes).
- **Bandeja del sistema:** resumen rápido del día, pausar/reanudar, abrir dashboard. Cerrar la ventana NO cierra la app.
- **Resiliente:** si la app muere de golpe (crash, corte de luz), al reabrir recupera el conteo sin inflar ni perder datos.
- **Idiomas:** español (default) e inglés.
- **Retención:** el detalle crudo se purga a los 180 días (configurable); los agregados diarios se conservan.

## Instalación

Descarga el instalador desde [Releases](https://github.com/zinetnorf/usage-time-tracker/releases):

| Plataforma | Archivo |
|---|---|
| macOS (Intel y Apple Silicon) | `Usage Tracker_x.y.z_universal.dmg` |
| Windows 11 | `Usage Tracker_x.y.z_x64-setup.exe` o `.msi` |

### macOS: primer arranque

Los instaladores no están firmados con certificado de Apple (todavía), así que Gatekeeper protestará:

1. Abre el `.dmg` y arrastra **Usage Tracker** a Aplicaciones.
2. La primera vez: **clic derecho → Abrir** (en vez de doble clic).
3. Si macOS dice que la app está "dañada", ejecuta en Terminal:
   ```bash
   xattr -cr "/Applications/Usage Tracker.app"
   ```
4. **Permiso de Accesibilidad:** macOS lo exige para leer el título de la ventana activa. El onboarding de la app te guía para concederlo (Ajustes → Privacidad y seguridad → Accesibilidad) y hay que **relanzar la app** después. Sin el permiso la app funciona igual, solo que sin títulos de ventana.

### Windows: primer arranque

SmartScreen mostrará "editor desconocido" (instalador sin firmar): **Más información → Ejecutar de todas formas**. No requiere permisos especiales.

## Desarrollo

### Prerrequisitos

- [Rust](https://rustup.rs) (stable)
- [Node.js](https://nodejs.org) 22+
- [pnpm](https://pnpm.io) 11+
- Linux no está soportado todavía (el tracking usa APIs de Windows/macOS).

### Comandos

```bash
pnpm install        # dependencias del frontend
pnpm tauri dev      # app en modo desarrollo (hot reload)

# Tests
cd src-tauri && cargo test    # núcleo Rust
pnpm exec vitest run          # utilidades del frontend

# Build de producción
pnpm tauri build                                      # plataforma actual
pnpm tauri build --target universal-apple-darwin      # macOS universal
```

Los instaladores quedan en `src-tauri/target/**/release/bundle/`.

### Releases automáticas

El workflow de GitHub Actions (`.github/workflows/release.yml`) compila macOS (universal) y Windows al pushear un tag:

```bash
git tag v0.1.0 && git push origin v0.1.0
```

Genera un **draft release** con los instaladores adjuntos; revísalo y publícalo desde GitHub.

## Arquitectura

```
┌──────────────────────────────────────────────┐
│ Proceso Rust (siempre vivo)                  │
│  Tracker loop (poll 1.5s) ──▶ SQLite (WAL)   │
│  - ventana activa (x-win)        ▲           │
│  - idle del sistema (user-idle)  │ invoke    │
│  - máquina de estados            │           │
│  Tray + power/lock detection     │           │
└──────────────────────────────────┼───────────┘
                                   │
                  Webview React (solo al abrir)
                  Hoy · Histórico · Apps · Ajustes
```

- **Core:** Tauri v2 + Rust. El motor de tracking corre aunque la ventana esté cerrada; el webview solo se carga al abrir el dashboard.
- **UI:** React + TypeScript + Tailwind CSS + Recharts.
- **Datos:** SQLite vía `rusqlite` (modo WAL). Segmentos crudos + rollup diario por app. Migraciones versionadas.
- **Tracking:** cada cambio de app o de estado (activo/inactivo) cierra un segmento y abre otro; un segmento que cruza medianoche se parte por día. Flush periódico anti-crash del segmento en curso.

## Configuración

Todas las claves se editan desde la vista **Ajustes**:

| Clave | Default | Descripción |
|---|---|---|
| `idle_threshold_sec` | 60 | Segundos sin input para pasar a inactivo |
| `count_idle_as_usage` | sí | Si el tiempo inactivo suma en reportes |
| `track_window_titles` | sí | Guardar título de la ventana activa |
| `poll_interval_ms` | 1500 | Cadencia del muestreo |
| `raw_retention_days` | 180 | Días de detalle crudo antes de purgar |
| `autostart_enabled` | sí | Iniciar al arrancar la sesión |
| `language` | es | Idioma de la UI (`es` / `en`) |

> **Límite conocido:** sin input no es posible distinguir "usuario ausente" de "usuario leyendo/viendo contenido": ambos cuentan como inactivo. Por eso es configurable si el tiempo inactivo suma o no.

## Roadmap

- [x] MVP: tracking + dashboard + tray + onboarding (macOS/Windows)
- [x] Blacklist de apps y export a PDF
- [ ] Desglose por título de ventana / sitio web
- [ ] Categorías y metas (productivo vs. distracción)
- [ ] Firma y notarización de instaladores + auto-update
- [ ] Linux
