import { invoke } from "@tauri-apps/api/core";
import type {
  AppStatusDto,
  LogLineDto,
  ServiceActionResultDto,
  TriggerSyncResultDto,
  WorkspaceJqlInputDto,
  WorkspaceJqlValidationDto,
} from "../types";

export async function getAppStatus(): Promise<AppStatusDto> {
  return invoke<AppStatusDto>("get_app_status");
}

export async function triggerSync(
  kind: "resync" | "full_resync",
): Promise<TriggerSyncResultDto> {
  return invoke<TriggerSyncResultDto>("trigger_sync", { kind });
}

export async function ensureServiceRunningOrRestart(): Promise<ServiceActionResultDto> {
  return invoke<ServiceActionResultDto>("ensure_service_running_or_restart");
}

export async function getSessionLogs(): Promise<LogLineDto[]> {
  return invoke<LogLineDto[]>("get_session_logs");
}

export async function getWorkspaceJqlConfig(): Promise<WorkspaceJqlInputDto[]> {
  return invoke<WorkspaceJqlInputDto[]>("get_workspace_jql_config");
}

export async function validateWorkspaceJqls(
  workspaces: WorkspaceJqlInputDto[],
): Promise<WorkspaceJqlValidationDto[]> {
  return invoke<WorkspaceJqlValidationDto[]>("validate_workspace_jqls", {
    workspaces,
  });
}

export async function saveWorkspaceJqlConfig(
  workspaces: WorkspaceJqlInputDto[],
): Promise<void> {
  return invoke<void>("save_workspace_jql_config", { workspaces });
}
