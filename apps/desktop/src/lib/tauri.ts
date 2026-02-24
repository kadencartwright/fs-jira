import { invoke } from "@tauri-apps/api/core";
import type {
  AppStatusDto,
  StartServiceResultDto,
  TriggerSyncResultDto,
} from "../types";

export async function getAppStatus(): Promise<AppStatusDto> {
  return invoke<AppStatusDto>("get_app_status");
}

export async function triggerSync(
  kind: "resync" | "full_resync",
): Promise<TriggerSyncResultDto> {
  return invoke<TriggerSyncResultDto>("trigger_sync", { kind });
}

export async function startUserService(): Promise<StartServiceResultDto> {
  return invoke<StartServiceResultDto>("start_user_service");
}
