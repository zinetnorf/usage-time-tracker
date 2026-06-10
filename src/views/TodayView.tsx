import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Bar,
  BarChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { api } from "../lib/api";
import {
  formatDuration,
  topApps,
  type AppDayUsage,
} from "../lib/format";

const REFRESH_MS = 15_000;

interface Props {
  countIdle: boolean;
  topCount: number;
}

export function TodayView({ countIdle, topCount }: Props) {
  const { t } = useTranslation();
  const [rows, setRows] = useState<AppDayUsage[]>([]);

  useEffect(
    function pollTodaySummary() {
      let cancelled = false;
      const load = () =>
        api.todaySummary().then((data) => {
          if (!cancelled) setRows(data);
        });
      load();
      const timer = setInterval(load, REFRESH_MS);
      return function stopPolling() {
        cancelled = true;
        clearInterval(timer);
      };
    },
    [],
  );

  const { top, others } = topApps(rows, topCount, countIdle);
  const totalSec = rows.reduce(
    (acc, r) => acc + r.active_sec + (countIdle ? r.idle_sec : 0),
    0,
  );
  const chartData = [
    ...top.map((r) => ({
      name: r.display_name,
      activo: r.active_sec,
      inactivo: countIdle ? r.idle_sec : 0,
    })),
    ...(others
      ? [
          {
            name: t("today.others"),
            activo: others.active_sec,
            inactivo: countIdle ? others.idle_sec : 0,
          },
        ]
      : []),
  ];

  if (rows.length === 0) {
    return <p className="text-zinc-400 py-12 text-center">{t("today.empty")}</p>;
  }

  return (
    <div className="space-y-6">
      <div>
        <p className="text-sm text-zinc-400">{t("today.total")}</p>
        <p className="text-4xl font-semibold tabular-nums">
          {formatDuration(totalSec)}
        </p>
      </div>

      <div className="h-72">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={chartData} margin={{ top: 4, right: 8, left: 8 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#3f3f46" />
            <XAxis
              dataKey="name"
              tick={{ fill: "#a1a1aa", fontSize: 11 }}
              interval={0}
              angle={-30}
              textAnchor="end"
              height={70}
            />
            <YAxis
              tick={{ fill: "#a1a1aa", fontSize: 11 }}
              tickFormatter={(v: number) => formatDuration(v)}
              width={70}
            />
            <Tooltip
              formatter={(value, name) => [
                formatDuration(Number(value ?? 0)),
                name === "activo" ? t("today.active") : t("today.idle"),
              ]}
              contentStyle={{ background: "#18181b", border: "1px solid #3f3f46" }}
              labelStyle={{ color: "#e4e4e7" }}
            />
            <Bar dataKey="activo" stackId="uso" fill="#34d399" />
            <Bar dataKey="inactivo" stackId="uso" fill="#fbbf24" />
          </BarChart>
        </ResponsiveContainer>
      </div>

      <ul className="divide-y divide-zinc-800">
        {rows.map((r) => (
          <li key={r.app_id} className="flex items-center justify-between py-2">
            <span className="truncate pr-4">{r.display_name}</span>
            <span className="flex gap-4 tabular-nums text-sm shrink-0">
              <span className="text-emerald-400">
                {t("today.active")} {formatDuration(r.active_sec)}
              </span>
              <span className="text-amber-400">
                {t("today.idle")} {formatDuration(r.idle_sec)}
              </span>
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}
