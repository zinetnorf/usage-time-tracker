import { useTranslation } from "react-i18next";
import { api, type Settings } from "../lib/api";
import { fillRangeGaps, lastNDays } from "../lib/format";
import { exportReport, withoutBlacklisted } from "../lib/pdf";

const NUMBER_KEYS = [
  "idle_threshold_sec",
  "poll_interval_ms",
  "flush_interval_sec",
  "raw_retention_days",
  "top_apps_count",
] as const;

const BOOL_KEYS = [
  "count_idle_as_usage",
  "track_window_titles",
  "autostart_enabled",
  "tracking_paused",
] as const;

interface Props {
  settings: Settings;
  onChange: (key: string, value: string) => void;
}

export function SettingsView({ settings, onChange }: Props) {
  const { t } = useTranslation();

  const exportFull = async () => {
    const countIdle = settings.count_idle_as_usage === "true";
    const days = lastNDays(30);
    const [today, rangeTotals, rangeApps] = await Promise.all([
      api.todaySummary(),
      api.rangeTotals(days[0], days[days.length - 1]),
      api.rangeSummary(days[0], days[days.length - 1]),
    ]);
    const todayClean = await withoutBlacklisted(today);
    const appsClean = await withoutBlacklisted(rangeApps);
    const todayTotal = todayClean.reduce(
      (acc, r) => acc + r.active_sec + (countIdle ? r.idle_sec : 0),
      0,
    );
    await exportReport(
      t("pdf.reportTitle"),
      t("pdf.generated", { date: new Date().toLocaleString() }),
      [
        {
          section: t("pdf.todaySection"),
          total: { label: t("pdf.total"), sec: todayTotal },
          apps: todayClean,
        },
        {
          section: `${t("pdf.trendSection")} — ${days[0]} → ${days[days.length - 1]}`,
          days: fillRangeGaps(days, rangeTotals),
        },
        { section: t("pdf.byAppSection"), apps: appsClean },
      ],
      {
        app: t("pdf.app"),
        active: t("pdf.active"),
        idle: t("pdf.idle"),
        total: t("pdf.total"),
        day: t("pdf.day"),
      },
      `usage-report-${days[days.length - 1]}.pdf`,
    );
  };

  return (
    <div className="space-y-6 max-w-xl">
      <p className="text-xs text-zinc-500 border border-zinc-800 rounded p-3">
        {t("settings.privacy")}
      </p>

      <div className="space-y-3">
        {BOOL_KEYS.map((key) => (
          <label key={key} className="flex items-center justify-between gap-4">
            <span className="text-sm">{t(`settings.${key}`)}</span>
            <input
              type="checkbox"
              checked={settings[key] === "true"}
              onChange={(e) => onChange(key, e.target.checked ? "true" : "false")}
              className="h-4 w-4 accent-emerald-500"
            />
          </label>
        ))}
      </div>

      <p className="text-xs text-zinc-500">{t("settings.idleNote")}</p>

      <div className="space-y-3">
        {NUMBER_KEYS.map((key) => (
          <label key={key} className="flex items-center justify-between gap-4">
            <span className="text-sm">{t(`settings.${key}`)}</span>
            <input
              type="number"
              min={1}
              value={settings[key] ?? ""}
              onChange={(e) => onChange(key, e.target.value)}
              className="w-28 bg-zinc-900 border border-zinc-700 rounded px-2 py-1 text-sm tabular-nums"
            />
          </label>
        ))}
      </div>

      <button
        onClick={exportFull}
        className="px-3 py-1.5 text-sm rounded border border-zinc-700 hover:border-zinc-500"
      >
        {t("pdf.exportFull")}
      </button>

      <label className="flex items-center justify-between gap-4">
        <span className="text-sm">{t("settings.language")}</span>
        <select
          value={settings.language ?? "es"}
          onChange={(e) => onChange("language", e.target.value)}
          className="bg-zinc-900 border border-zinc-700 rounded px-2 py-1 text-sm"
        >
          <option value="es">Español</option>
          <option value="en">English</option>
        </select>
      </label>
    </div>
  );
}
