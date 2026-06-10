import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";

export interface OnboardingStatus {
  done: boolean;
  is_macos: boolean;
  accessibility_granted: boolean;
}

const RECHECK_MS = 3000;

interface Props {
  status: OnboardingStatus;
  onFinished: () => void;
}

export function OnboardingView({ status, onFinished }: Props) {
  const { t } = useTranslation();
  const [autostart, setAutostart] = useState(true);
  const [granted, setGranted] = useState(status.accessibility_granted);

  useEffect(
    function pollAccessibility() {
      if (!status.is_macos || granted) return;
      const timer = setInterval(() => {
        invoke<OnboardingStatus>("get_onboarding").then((s) =>
          setGranted(s.accessibility_granted),
        );
      }, RECHECK_MS);
      return function stopPolling() {
        clearInterval(timer);
      };
    },
    [status.is_macos, granted],
  );

  const finish = async () => {
    await invoke("finish_onboarding", { autostart });
    onFinished();
  };

  return (
    <main className="min-h-screen bg-zinc-950 text-zinc-100 flex items-center justify-center px-6">
      <div className="max-w-lg space-y-6 py-10">
        <h1 className="text-2xl font-semibold">{t("onboarding.title")}</h1>
        <p className="text-zinc-300">{t("onboarding.what")}</p>
        <p className="text-sm text-zinc-400 border border-zinc-800 rounded p-3">
          {t("onboarding.privacy")}
        </p>

        <label className="flex items-center gap-3">
          <input
            type="checkbox"
            checked={autostart}
            onChange={(e) => setAutostart(e.target.checked)}
            className="h-4 w-4 accent-emerald-500"
          />
          <span className="text-sm">{t("onboarding.autostart")}</span>
        </label>

        {status.is_macos ? (
          <div className="space-y-3 border border-zinc-800 rounded p-4">
            <p className="text-sm font-medium">
              {t("onboarding.axTitle")}{" "}
              {granted ? (
                <span className="text-emerald-400">{t("onboarding.axGranted")}</span>
              ) : (
                <span className="text-amber-400">{t("onboarding.axPending")}</span>
              )}
            </p>
            {!granted ? (
              <>
                <p className="text-sm text-zinc-400">{t("onboarding.axWhy")}</p>
                <button
                  onClick={() => invoke("open_accessibility_settings")}
                  className="px-3 py-1.5 text-sm rounded bg-zinc-800 hover:bg-zinc-700 border border-zinc-700"
                >
                  {t("onboarding.axOpen")}
                </button>
                <p className="text-xs text-amber-400">{t("onboarding.axRelaunch")}</p>
              </>
            ) : null}
          </div>
        ) : null}

        <button
          onClick={finish}
          className="w-full py-2 rounded bg-emerald-600 hover:bg-emerald-500 font-medium"
        >
          {t("onboarding.start")}
        </button>
      </div>
    </main>
  );
}
