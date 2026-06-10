export interface AppDayUsage {
  app_id: number;
  display_name: string;
  active_sec: number;
  idle_sec: number;
}

export interface DayTotals {
  day: string;
  active_sec: number;
  idle_sec: number;
}

/** "45s", "1m", "3h 05m" — para totales del dashboard. */
export function formatDuration(totalSec: number): string {
  if (totalSec < 60) return `${totalSec}s`;
  const hours = Math.floor(totalSec / 3600);
  const minutes = Math.floor((totalSec % 3600) / 60);
  if (hours === 0) return `${minutes}m`;
  return `${hours}h ${String(minutes).padStart(2, "0")}m`;
}

function toDayString(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

/** Últimos n días locales (inclusive hasta `today`), como 'YYYY-MM-DD'. */
export function lastNDays(n: number, today: Date = new Date()): string[] {
  const days: string[] = [];
  for (let i = n - 1; i >= 0; i--) {
    const d = new Date(today.getFullYear(), today.getMonth(), today.getDate() - i);
    days.push(toDayString(d));
  }
  return days;
}

/** Une los días del rango con los datos; días sin filas quedan en cero. */
export function fillRangeGaps(days: string[], rows: DayTotals[]): DayTotals[] {
  const byDay = new Map(rows.map((r) => [r.day, r]));
  return days.map(
    (day) => byDay.get(day) ?? { day, active_sec: 0, idle_sec: 0 },
  );
}

/** Top N apps por uso (activo, + idle si countIdle); el resto agregado. */
export function topApps(
  rows: AppDayUsage[],
  n: number,
  countIdle: boolean,
): { top: AppDayUsage[]; others: { active_sec: number; idle_sec: number } | null } {
  const total = (r: AppDayUsage) => r.active_sec + (countIdle ? r.idle_sec : 0);
  const sorted = [...rows].sort((a, b) => total(b) - total(a));
  const top = sorted.slice(0, n);
  const rest = sorted.slice(n);
  if (rest.length === 0) return { top, others: null };
  return {
    top,
    others: {
      active_sec: rest.reduce((acc, r) => acc + r.active_sec, 0),
      idle_sec: rest.reduce((acc, r) => acc + r.idle_sec, 0),
    },
  };
}
