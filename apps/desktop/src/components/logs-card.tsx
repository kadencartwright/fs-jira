import type { LogLineDto } from "../types";

type Props = {
  logs: LogLineDto[];
  loading: boolean;
  error: string | null;
};

export function LogsCard({ logs, loading, error }: Props) {
  return (
    <section className="rounded-xl border border-slate-700/80 bg-slate-900/70 p-4 shadow-sm shadow-black/20">
      <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-slate-400">
        Session Logs
      </h2>

      {loading ? (
        <p className="text-sm text-slate-300">Loading logs...</p>
      ) : null}
      {error ? (
        <p className="rounded-md border border-red-500/40 bg-red-950/40 p-2 text-xs text-red-200">
          {error}
        </p>
      ) : null}
      {!loading && !error && logs.length === 0 ? (
        <p className="text-sm text-slate-400">
          No logs captured in this desktop session yet.
        </p>
      ) : null}

      {logs.length > 0 ? (
        <div className="max-h-72 overflow-y-auto rounded-md border border-slate-700/70 bg-slate-950/60 p-2 font-mono text-xs text-slate-200">
          {logs.map((entry, idx) => (
            <div
              className="whitespace-pre-wrap break-words py-0.5"
              key={`${idx}-${entry.line}`}
            >
              <span className="text-slate-500">[{entry.source}] </span>
              {entry.line}
            </div>
          ))}
        </div>
      ) : null}
    </section>
  );
}
