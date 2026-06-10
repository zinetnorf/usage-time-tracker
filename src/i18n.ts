import i18n from "i18next";
import { initReactI18next } from "react-i18next";

const resources = {
  es: {
    translation: {
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
