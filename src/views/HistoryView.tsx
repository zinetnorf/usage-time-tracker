import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { api } from "../lib/api";
import {
  fillRangeGaps,
  formatDuration,
  lastNDays,
  type AppDayUsage,
  type DayTotals,
} from "../lib/format";
import { exportReport, withoutBlacklisted } from "../lib/pdf";

const RANGES = [7, 14, 30] as const;
type RangeDays = (typeof RANGES)[number];

interface Props {
  countIdle: boolean;
}

export function HistoryView({ countIdle }: Props) {
  const { t } = useTranslation();
  const [rangeDays, setRangeDays] = useState<RangeDays>(7);
  const [totals, setTotals] = useState<DayTotals[]>([]);
  const [byApp, setByApp] = useState<AppDayUsage[]>([]);
  const chartRef = useRef<HTMLDivElement>(null);

  const exportPdf = async () => {
    const clean = await withoutBlacklisted(byApp);
    const days = lastNDays(rangeDays);
    await exportReport(
      t("pdf.reportTitle"),
      t("pdf.generated", { date: new Date().toLocaleString() }),
      [
        {
          section: `${t("pdf.trendSection")} — ${days[0]} → ${days[days.length - 1]}`,
          chart: chartRef.current?.querySelector("svg"),
          days: totals,
        },
        { section: t("pdf.byAppSection"), apps: clean },
      ],
      {
        app: t("pdf.app"),
        active: t("pdf.active"),
        idle: t("pdf.idle"),
        total: t("pdf.total"),
        day: t("pdf.day"),
      },
      `usage-${days[0]}-${days[days.length - 1]}.pdf`,
    );
  };

  useEffect(
    function loadRange() {
      let cancelled = false;
      const days = lastNDays(rangeDays);
      const from = days[0];
      const to = days[days.length - 1];
      Promise.all([api.rangeTotals(from, to), api.rangeSummary(from, to)]).then(
        ([totalRows, appRows]) => {
          if (cancelled) return;
          setTotals(fillRangeGaps(days, totalRows));
          setByApp(appRows);
        },
      );
      return function cancelLoad() {
        cancelled = true;
      };
    },
    [rangeDays],
  );

  const chartData = totals.map((d) => ({
    day: d.day.slice(5), // 'MM-DD'
    total: d.active_sec + (countIdle ? d.idle_sec : 0),
  }));
  const hasData = chartData.some((d) => d.total > 0);

  return (
    <div className="space-y-6">
      <div className="flex gap-2 items-center">
        {RANGES.map((n) => (
          <button
            key={n}
            onClick={() => setRangeDays(n)}
            className={`px-3 py-1 rounded-full text-sm border ${
              rangeDays === n
                ? "bg-emerald-600 border-emerald-600 text-white"
                : "border-zinc-700 text-zinc-300 hover:border-zinc-500"
            }`}
          >
            {t(`history.range${n}`)}
          </button>
        ))}
        {hasData ? (
          <button
            onClick={exportPdf}
            className="ml-auto px-3 py-1.5 text-sm rounded border border-zinc-700 hover:border-zinc-500"
          >
            {t("pdf.export")}
          </button>
        ) : null}
      </div>

      {hasData ? (
        <>
          <div>
            <p className="text-sm text-zinc-400 mb-2">{t("history.trend")}</p>
            <div className="h-56" ref={chartRef}>
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={chartData} margin={{ top: 4, right: 8, left: 8 }}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#3f3f46" />
                  <XAxis dataKey="day" tick={{ fill: "#a1a1aa", fontSize: 11 }} />
                  <YAxis
                    tick={{ fill: "#a1a1aa", fontSize: 11 }}
                    tickFormatter={(v: number) => formatDuration(v)}
                    width={70}
                  />
                  <Tooltip
                    formatter={(value) => [formatDuration(Number(value ?? 0)), ""]}
                    contentStyle={{
                      background: "#18181b",
                      border: "1px solid #3f3f46",
                    }}
                    labelStyle={{ color: "#e4e4e7" }}
                  />
                  <Line
                    type="monotone"
                    dataKey="total"
                    stroke="#34d399"
                    strokeWidth={2}
                    dot={false}
                  />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </div>

          <div>
            <p className="text-sm text-zinc-400 mb-2">{t("history.byApp")}</p>
            <ul className="divide-y divide-zinc-800">
              {byApp.map((r) => (
                <li
                  key={r.app_id}
                  className="flex items-center justify-between py-2"
                >
                  <span className="truncate pr-4">{r.display_name}</span>
                  <span className="flex gap-4 tabular-nums text-sm shrink-0">
                    <span className="text-emerald-400">
                      {formatDuration(r.active_sec)}
                    </span>
                    <span className="text-amber-400">
                      {formatDuration(r.idle_sec)}
                    </span>
                  </span>
                </li>
              ))}
            </ul>
          </div>
        </>
      ) : (
        <p className="text-zinc-400 py-12 text-center">{t("history.empty")}</p>
      )}
    </div>
  );
}
