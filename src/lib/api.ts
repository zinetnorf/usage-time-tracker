import { invoke } from "@tauri-apps/api/core";
import type { AppDayUsage, DayTotals } from "./format";

export interface AppRow {
  id: number;
  identity: string;
  display_name: string;
  process_name: string | null;
  exe_path: string | null;
  bundle_id: string | null;
}

export type Settings = Record<string, string>;

export const api = {
  todaySummary: () => invoke<AppDayUsage[]>("get_today_summary"),
  daySummary: (day: string) => invoke<AppDayUsage[]>("get_day_summary", { day }),
  rangeTotals: (fromDay: string, toDay: string) =>
    invoke<DayTotals[]>("get_range_totals", { fromDay, toDay }),
  rangeSummary: (fromDay: string, toDay: string) =>
    invoke<AppDayUsage[]>("get_range_summary", { fromDay, toDay }),
  apps: () => invoke<AppRow[]>("get_apps"),
  renameApp: (appId: number, name: string) =>
    invoke<void>("rename_app", { appId, name }),
  mergeApps: (fromId: number, intoId: number) =>
    invoke<void>("merge_apps", { fromId, intoId }),
  settings: () => invoke<Settings>("get_settings"),
  setSetting: (key: string, value: string) =>
    invoke<void>("set_setting", { key, value }),
};
