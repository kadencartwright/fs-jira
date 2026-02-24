import { ActionsCard } from "./components/actions-card";
import { PathCard } from "./components/path-card";
import { StatusCard } from "./components/status-card";
import { useAppStatus } from "./hooks/use-app-status";

export default function App() {
  const {
    status,
    loading,
    error,
    canTriggerSync,
    canStartService,
    runAction,
    runStartService,
  } = useAppStatus();

  return (
    <main className="min-h-screen bg-slate-950 p-4 text-slate-100 sm:p-6">
      <div className="mx-auto flex w-full max-w-3xl flex-col gap-4">
        <header>
          <p className="text-xs uppercase tracking-[0.24em] text-slate-400">
            fs-jira desktop
          </p>
          <h1 className="mt-1 text-2xl font-semibold text-slate-50">
            Service Control Panel
          </h1>
        </header>

        {error ? (
          <div className="rounded-lg border border-red-500/30 bg-red-950/40 p-3 text-sm text-red-200">
            {error}
          </div>
        ) : null}
        {loading && !status ? (
          <div className="rounded-lg border border-slate-700 bg-slate-900/70 p-3 text-sm text-slate-200">
            Loading status...
          </div>
        ) : null}

        {status ? (
          <>
            <StatusCard status={status} />
            <PathCard status={status} />
            <ActionsCard
              serviceInstalled={status.service_installed}
              startServiceDisabled={!canStartService}
              syncDisabled={!canTriggerSync}
              onStartService={runStartService}
              onFullResync={() => runAction("full_resync")}
              onResync={() => runAction("resync")}
            />

            {status.errors.length > 0 ? (
              <section className="rounded-xl border border-amber-500/40 bg-amber-950/35 p-4 text-sm text-amber-200">
                <h2 className="mb-2 font-semibold">Diagnostics</h2>
                <ul className="list-disc pl-4">
                  {status.errors.map((item) => (
                    <li key={item}>{item}</li>
                  ))}
                </ul>
              </section>
            ) : null}
          </>
        ) : null}
      </div>
    </main>
  );
}
