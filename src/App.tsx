import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { api, type Settings } from "./lib/api";
import { AppsView } from "./views/AppsView";
import { HistoryView } from "./views/HistoryView";
import { OnboardingView, type OnboardingStatus } from "./views/OnboardingView";
import { SettingsView } from "./views/SettingsView";
import { TodayView } from "./views/TodayView";

const TABS = ["today", "history", "apps", "settings"] as const;
type Tab = (typeof TABS)[number];

export default function App() {
  const { t, i18n } = useTranslation();
  const [tab, setTab] = useState<Tab>("today");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [onboarding, setOnboarding] = useState<OnboardingStatus | null>(null);

  useEffect(function loadSettings() {
    Promise.all([api.settings(), invoke<OnboardingStatus>("get_onboarding")]).then(
      ([loaded, status]) => {
        setSettings(loaded);
        setOnboarding(status);
        i18n.changeLanguage(loaded.language ?? "es");
      },
    );
  }, []);

  const changeSetting = (key: string, value: string) => {
    setSettings((curr) => (curr ? { ...curr, [key]: value } : curr));
    api.setSetting(key, value);
    if (key === "language") i18n.changeLanguage(value);
  };

  if (!settings || !onboarding) return null;

  if (!onboarding.done) {
    return (
      <OnboardingView
        status={onboarding}
        onFinished={() => setOnboarding({ ...onboarding, done: true })}
      />
    );
  }

  const countIdle = settings.count_idle_as_usage === "true";
  const topCount = Number(settings.top_apps_count) || 10;

  return (
    <main className="min-h-screen bg-zinc-950 text-zinc-100 px-6 py-5">
      <nav className="flex gap-1 mb-6 border-b border-zinc-800">
        {TABS.map((name) => (
          <button
            key={name}
            onClick={() => setTab(name)}
            className={`px-4 py-2 text-sm border-b-2 -mb-px ${
              tab === name
                ? "border-emerald-500 text-white"
                : "border-transparent text-zinc-400 hover:text-zinc-200"
            }`}
          >
            {t(`tabs.${name}`)}
          </button>
        ))}
      </nav>

      {tab === "today" ? (
        <TodayView countIdle={countIdle} topCount={topCount} />
      ) : tab === "history" ? (
        <HistoryView countIdle={countIdle} />
      ) : tab === "apps" ? (
        <AppsView />
      ) : (
        <SettingsView settings={settings} onChange={changeSetting} />
      )}
    </main>
  );
}
