import i18n from "i18next";
import { initReactI18next } from "react-i18next";

const resources = {
  es: {
    translation: {
      onboarding: {
        title: "Bienvenido a Usage Tracker",
        what: "Esta app mide cuánto tiempo usas cada aplicación: detecta la ventana en primer plano y distingue entre tiempo activo (con teclado/mouse) e inactivo. Vive en la bandeja del sistema y sigue contando con la ventana cerrada.",
        privacy:
          "Todo es 100% local: los datos nunca salen de este equipo. Sin telemetría, sin red, sin cuentas.",
        autostart: "Iniciar automáticamente al arrancar la sesión",
        axTitle: "Permiso de Accesibilidad:",
        axGranted: "concedido ✓",
        axPending: "pendiente",
        axWhy: "macOS lo exige para leer el título de la ventana activa. Sin él la app cuenta el tiempo por aplicación igualmente, pero sin títulos.",
        axOpen: "Abrir Ajustes del sistema",
        axRelaunch: "Tras concederlo hay que relanzar la app para que tome efecto.",
        start: "Empezar",
      },
      tabs: { today: "Hoy", history: "Histórico", apps: "Apps", settings: "Ajustes" },
      today: {
        total: "Uso total de hoy",
        active: "Activo",
        idle: "Inactivo",
        others: "Otras",
        empty: "Sin actividad registrada hoy todavía.",
      },
      history: {
        range7: "7 días",
        range14: "14 días",
        range30: "30 días",
        trend: "Tendencia diaria",
        byApp: "Desglose por app",
        empty: "Sin datos en este rango.",
      },
      apps: {
        title: "Apps detectadas",
        rename: "Renombrar",
        merge: "Fusionar con…",
        mergeInto: "Fusionar «{{from}}» dentro de:",
        mergeConfirm:
          "La historia de «{{from}}» pasará a «{{into}}» y la entrada desaparecerá. ¿Continuar?",
        blacklist: "No trackear",
        blacklistHint:
          "Excluir del tracking. La historia ya registrada se conserva.",
        cancel: "Cancelar",
        save: "Guardar",
        empty: "Aún no hay apps detectadas.",
      },
      settings: {
        idle_threshold_sec: "Segundos sin input para pasar a inactivo",
        count_idle_as_usage: "Contar tiempo inactivo en los reportes",
        track_window_titles: "Guardar títulos de ventana",
        poll_interval_ms: "Intervalo de muestreo (ms)",
        flush_interval_sec: "Frecuencia de guardado del segmento abierto (s)",
        raw_retention_days: "Días de retención del detalle crudo",
        autostart_enabled: "Iniciar al arrancar la sesión",
        language: "Idioma",
        top_apps_count: "Apps mostradas en «Hoy»",
        tracking_paused: "Tracking en pausa",
        privacy:
          "Todos los datos viven solo en este equipo. Esta app no usa la red.",
        idleNote:
          "Sin teclado ni mouse no es posible distinguir «ausente» de «leyendo/viendo contenido»: ambos cuentan como inactivo.",
      },
    },
  },
  en: {
    translation: {
      onboarding: {
        title: "Welcome to Usage Tracker",
        what: "This app measures how long you use each application: it detects the foreground window and distinguishes active time (keyboard/mouse) from idle time. It lives in the system tray and keeps counting with the window closed.",
        privacy:
          "Everything is 100% local: data never leaves this device. No telemetry, no network, no accounts.",
        autostart: "Launch automatically at login",
        axTitle: "Accessibility permission:",
        axGranted: "granted ✓",
        axPending: "pending",
        axWhy: "macOS requires it to read the active window title. Without it the app still counts time per application, just without titles.",
        axOpen: "Open System Settings",
        axRelaunch: "After granting it, relaunch the app for it to take effect.",
        start: "Get started",
      },
      tabs: { today: "Today", history: "History", apps: "Apps", settings: "Settings" },
      today: {
        total: "Total usage today",
        active: "Active",
        idle: "Idle",
        others: "Others",
        empty: "No activity recorded today yet.",
      },
      history: {
        range7: "7 days",
        range14: "14 days",
        range30: "30 days",
        trend: "Daily trend",
        byApp: "Breakdown by app",
        empty: "No data in this range.",
      },
      apps: {
        title: "Detected apps",
        rename: "Rename",
        merge: "Merge into…",
        mergeInto: "Merge “{{from}}” into:",
        mergeConfirm:
          "The history of “{{from}}” will move to “{{into}}” and the entry will disappear. Continue?",
        blacklist: "Don't track",
        blacklistHint: "Exclude from tracking. Already recorded history is kept.",
        cancel: "Cancel",
        save: "Save",
        empty: "No apps detected yet.",
      },
      settings: {
        idle_threshold_sec: "Seconds without input before idle",
        count_idle_as_usage: "Count idle time in reports",
        track_window_titles: "Store window titles",
        poll_interval_ms: "Polling interval (ms)",
        flush_interval_sec: "Open segment save frequency (s)",
        raw_retention_days: "Raw detail retention days",
        autostart_enabled: "Launch at login",
        language: "Language",
        top_apps_count: "Apps shown in “Today”",
        tracking_paused: "Tracking paused",
        privacy: "All data lives on this device only. This app never uses the network.",
        idleNote:
          "Without keyboard or mouse input it is impossible to tell “away” from “reading/watching”: both count as idle.",
      },
    },
  },
};

i18n.use(initReactI18next).init({
  resources,
  lng: "es",
  fallbackLng: "es",
  interpolation: { escapeValue: false },
});

export default i18n;
