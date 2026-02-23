import type { AppStatusDto } from "../types";

type Props = {
  status: AppStatusDto;
};

function PathRow({ label, value }: { label: string; value: string | null }) {
  return (
    <div className="rounded-md border border-slate-700/70 bg-slate-900/65 p-3">
      <p className="mb-1 text-xs uppercase tracking-wide text-slate-400">{label}</p>
      <p className="break-all font-mono text-xs text-slate-100">
        {value ?? "unresolved"}
      </p>
    </div>
  );
}

export function PathCard({ status }: Props) {
  return (
    <section className="rounded-xl border border-slate-700/80 bg-slate-900/70 p-4 shadow-sm shadow-black/20">
      <h2 className="mb-3 text-sm font-semibold uppercase tracking-wide text-slate-400">
        Paths
      </h2>
      <div className="grid grid-cols-1 gap-3">
        <PathRow label="Config path" value={status.config_path} />
        <PathRow label="Mountpoint" value={status.mountpoint} />
      </div>
    </section>
  );
}
