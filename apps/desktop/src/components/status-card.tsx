import type { AppStatusDto } from "../types";

type Props = {
  status: AppStatusDto;
};

const STATE_COLOR: Record<AppStatusDto["sync_state"], string> = {
  stopped: "bg-slate-700 text-slate-100",
  running: "bg-emerald-900/70 text-emerald-200",
  syncing: "bg-amber-900/70 text-amber-200",
  degraded: "bg-red-900/70 text-red-200",
};

export function StatusCard({ status }: Props) {
  return (
    <section className="rounded-xl border border-slate-700/80 bg-slate-900/70 p-4 shadow-sm shadow-black/20">
      <div className="mb-4 flex items-center justify-between">
        <h2 className="text-sm font-semibold uppercase tracking-wide text-slate-400">
          Runtime Status
        </h2>
        <span
          className={`rounded-full px-3 py-1 text-xs font-semibold ${STATE_COLOR[status.sync_state]}`}
        >
          {status.sync_state}
        </span>
      </div>

      <dl className="grid grid-cols-1 gap-2 text-sm text-slate-100 sm:grid-cols-2">
        <div>
          <dt className="text-slate-400">Platform</dt>
          <dd>{status.platform}</dd>
        </div>
        <div>
          <dt className="text-slate-400">Service</dt>
          <dd>
            {status.service_running
              ? "Running"
              : status.service_installed
                ? "Stopped"
                : "Not installed"}
          </dd>
        </div>
        <div>
          <dt className="text-slate-400">Last sync</dt>
          <dd>{status.sync.last_sync ?? "unknown"}</dd>
        </div>
        <div>
          <dt className="text-slate-400">Last full sync</dt>
          <dd>{status.sync.last_full_sync ?? "unknown"}</dd>
        </div>
        <div>
          <dt className="text-slate-400">Seconds to next sync</dt>
          <dd>{status.sync.seconds_to_next_sync ?? "unknown"}</dd>
        </div>
        <div>
          <dt className="text-slate-400">Path source</dt>
          <dd>{status.path_source}</dd>
        </div>
      </dl>
    </section>
  );
}
