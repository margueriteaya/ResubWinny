import type { Inspection } from "../../backend";

export type TaskRecord = {
  name: string;
  path: string;
  size: number;
  container: string;
  status: "Completed" | "Warning" | "In Progress";
  time: string;
  warnings: number;
  captions?: number;
  jobId?: string;
};

export const formatBytes = (value: number) =>
  value
    ? `${(value / 1024 ** 3).toFixed(value > 100 * 1024 ** 3 ? 1 : 2)} GB`
    : "—";

export const basename = (path: string) => path.split(/[\\/]/).pop() ?? path;

export function routeLabel(
  routeCode: Inspection["routeCode"] | undefined,
  message: (key: string) => string,
) {
  if (routeCode === "mpeg_ts_b24_verified") return message("route.mpegTs");
  if (routeCode === "mpeg_ts_ttml_candidate") return message("route.mpegTsTtml");
  if (routeCode === "mpeg_ts_192_ttml_verified") return message("route.m2ts");
  if (routeCode === "tlv_mmtp_experimental") return message("route.tlv");
  return message("route.unknown");
}

export function upsertHistory(history: TaskRecord[], record: TaskRecord) {
  return [record, ...history.filter((item) => item.path !== record.path)].slice(0, 25);
}
