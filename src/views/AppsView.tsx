import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { api, type AppRow } from "../lib/api";

export function AppsView() {
  const { t } = useTranslation();
  const [apps, setApps] = useState<AppRow[]>([]);
  const [renaming, setRenaming] = useState<AppRow | null>(null);
  const [merging, setMerging] = useState<AppRow | null>(null);
  const [draft, setDraft] = useState("");

  const reload = () => api.apps().then(setApps);

  useEffect(function loadApps() {
    api.apps().then(setApps);
  }, []);

  const saveRename = async () => {
    if (renaming && draft.trim()) {
      await api.renameApp(renaming.id, draft.trim());
      setRenaming(null);
      await reload();
    }
  };

  const doMerge = async (into: AppRow) => {
    if (!merging) return;
    const message = t("apps.mergeConfirm", {
      from: merging.display_name,
      into: into.display_name,
    });
    if (window.confirm(message)) {
      await api.mergeApps(merging.id, into.id);
      setMerging(null);
      await reload();
    }
  };

  if (apps.length === 0) {
    return <p className="text-zinc-400 py-12 text-center">{t("apps.empty")}</p>;
  }

  return (
    <div className="space-y-4">
      <p className="text-sm text-zinc-400">{t("apps.title")}</p>
      <ul className="divide-y divide-zinc-800">
        {apps.map((app) => (
          <li key={app.id} className="py-2 space-y-2">
            <div className="flex items-center justify-between gap-4">
              <div className="min-w-0">
                <p className="truncate">{app.display_name}</p>
                <p className="truncate text-xs text-zinc-500">{app.identity}</p>
              </div>
              <div className="flex gap-2 shrink-0">
                <button
                  onClick={() => {
                    setMerging(null);
                    setRenaming(app);
                    setDraft(app.display_name);
                  }}
                  className="px-2 py-1 text-xs rounded border border-zinc-700 hover:border-zinc-500"
                >
                  {t("apps.rename")}
                </button>
                <button
                  onClick={() => {
                    setRenaming(null);
                    setMerging(app);
                  }}
                  className="px-2 py-1 text-xs rounded border border-zinc-700 hover:border-zinc-500"
                >
                  {t("apps.merge")}
                </button>
              </div>
            </div>

            {renaming?.id === app.id ? (
              <div className="flex gap-2">
                <input
                  value={draft}
                  onChange={(e) => setDraft(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && saveRename()}
                  autoFocus
                  className="flex-1 bg-zinc-900 border border-zinc-700 rounded px-2 py-1 text-sm"
                />
                <button
                  onClick={saveRename}
                  className="px-3 py-1 text-xs rounded bg-emerald-600 hover:bg-emerald-500"
                >
                  {t("apps.save")}
                </button>
                <button
                  onClick={() => setRenaming(null)}
                  className="px-3 py-1 text-xs rounded border border-zinc-700"
                >
                  {t("apps.cancel")}
                </button>
              </div>
            ) : null}

            {merging?.id === app.id ? (
              <div className="space-y-1">
                <p className="text-xs text-zinc-400">
                  {t("apps.mergeInto", { from: app.display_name })}
                </p>
                <div className="flex flex-wrap gap-2">
                  {apps
                    .filter((candidate) => candidate.id !== app.id)
                    .map((candidate) => (
                      <button
                        key={candidate.id}
                        onClick={() => doMerge(candidate)}
                        className="px-2 py-1 text-xs rounded border border-zinc-700 hover:border-emerald-500"
                      >
                        {candidate.display_name}
                      </button>
                    ))}
                  <button
                    onClick={() => setMerging(null)}
                    className="px-2 py-1 text-xs rounded border border-zinc-700 text-zinc-400"
                  >
                    {t("apps.cancel")}
                  </button>
                </div>
              </div>
            ) : null}
          </li>
        ))}
      </ul>
    </div>
  );
}
