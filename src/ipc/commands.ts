import { invoke } from "./invoke";
import type {
  CreateProjectArgs,
  ProjectTree,
  ValidationReport,
  RouteSpec,
  SchemaSpec,
  ModelSpec,
  MiddlewareSpec,
  HandlerSpec,
  PreviewCodeArgs,
  Language,
  Framework,
  ImportResult,
} from "./types";

// ---------------------------------------------------------------------------
// Project commands
// ---------------------------------------------------------------------------

export function createProject(args: CreateProjectArgs): Promise<ProjectTree> {
  return invoke<ProjectTree>("create_project", { args });
}

export function openProject(path: string): Promise<ProjectTree> {
  return invoke<ProjectTree>("open_project", { path });
}

export function closeProject(): Promise<void> {
  return invoke<void>("close_project");
}

export function getProjectTree(): Promise<ProjectTree> {
  return invoke<ProjectTree>("get_project_tree");
}

// ---------------------------------------------------------------------------
// Spec read commands
// ---------------------------------------------------------------------------

export function readRoute(filePath: string): Promise<RouteSpec> {
  return invoke<RouteSpec>("read_route", { filePath });
}

export function readSchema(filePath: string): Promise<SchemaSpec> {
  return invoke<SchemaSpec>("read_schema", { filePath });
}

export function readModel(filePath: string): Promise<ModelSpec> {
  return invoke<ModelSpec>("read_model", { filePath });
}

export function readMiddleware(filePath: string): Promise<MiddlewareSpec> {
  return invoke<MiddlewareSpec>("read_middleware", { filePath });
}

export function readHandler(filePath: string): Promise<HandlerSpec> {
  return invoke<HandlerSpec>("read_handler", { filePath });
}

// ---------------------------------------------------------------------------
// Spec write commands
// ---------------------------------------------------------------------------

export function writeRoute(filePath: string, value: RouteSpec): Promise<void> {
  return invoke<void>("write_route", { filePath, value });
}

export function writeSchema(filePath: string, value: SchemaSpec): Promise<void> {
  return invoke<void>("write_schema", { filePath, value });
}

export function writeModel(filePath: string, value: ModelSpec): Promise<void> {
  return invoke<void>("write_model", { filePath, value });
}

export function writeMiddleware(filePath: string, value: MiddlewareSpec): Promise<void> {
  return invoke<void>("write_middleware", { filePath, value });
}

export function writeHandler(filePath: string, value: HandlerSpec): Promise<void> {
  return invoke<void>("write_handler", { filePath, value });
}

// ---------------------------------------------------------------------------
// Spec delete commands
// ---------------------------------------------------------------------------

export function deleteRoute(filePath: string): Promise<void> {
  return invoke<void>("delete_route", { filePath });
}

export function deleteSchema(filePath: string): Promise<void> {
  return invoke<void>("delete_schema", { filePath });
}

export function deleteModel(filePath: string): Promise<void> {
  return invoke<void>("delete_model", { filePath });
}

export function deleteMiddleware(filePath: string): Promise<void> {
  return invoke<void>("delete_middleware", { filePath });
}

export function deleteHandler(filePath: string): Promise<void> {
  return invoke<void>("delete_handler", { filePath });
}

// ---------------------------------------------------------------------------
// Codegen commands
// ---------------------------------------------------------------------------

export function validateProject(): Promise<ValidationReport> {
  return invoke<ValidationReport>("validate_project");
}

export function previewCode(args: PreviewCodeArgs): Promise<Record<string, string>> {
  return invoke<Record<string, string>>("preview_code", { args });
}

export function generateProject(
  outputDir: string,
  language: Language,
  framework: Framework,
): Promise<{ outputDir: string; fileCount: number }> {
  return invoke<{ outputDir: string; fileCount: number }>("generate_project", {
    outputDir,
    language,
    framework,
  });
}

// ---------------------------------------------------------------------------
// Runtime commands
// ---------------------------------------------------------------------------

export type ServerStatus = "stopped" | "starting" | "running" | "stopping" | "errored";

export interface DetectedRuntime {
  name: string;
  version: string;
  path: string;
}

export interface PreflightReport {
  ok: boolean;
  checks: PreflightCheck[];
}

export interface PreflightCheck {
  name: string;
  passed: boolean;
  message: string;
}

export interface LogEntry {
  timestamp: string;
  level: "info" | "warn" | "error" | "debug";
  message: string;
  source: "stdout" | "stderr";
}

export function detectRuntimes(): Promise<DetectedRuntime[]> {
  return invoke<DetectedRuntime[]>("detect_runtimes");
}

export function runPreflight(): Promise<PreflightReport> {
  return invoke<PreflightReport>("run_preflight");
}

export function startServer(): Promise<number> {
  return invoke<number>("start_server");
}

export function stopServer(): Promise<void> {
  return invoke<void>("stop_server");
}

export function restartServer(): Promise<number> {
  return invoke<number>("restart_server");
}

export function getServerStatus(): Promise<ServerStatus> {
  return invoke<ServerStatus>("get_server_status");
}

// ---------------------------------------------------------------------------
// OpenAPI commands
// ---------------------------------------------------------------------------

export function exportOpenapi(): Promise<string> {
  return invoke<string>("export_openapi");
}

export function importOpenapi(openapiJson: string, targetDir: string): Promise<ImportResult> {
  return invoke<ImportResult>("import_openapi", { openapiJson, targetDir });
}

export function importFromCode(sourcePath: string, targetDir: string): Promise<ImportResult> {
  return invoke<ImportResult>("import_from_code", { sourcePath, targetDir });
}

// ---------------------------------------------------------------------------
// HMU commands (F1)
// ---------------------------------------------------------------------------

export interface FileChange {
  path: string;
  action: "create" | "update" | "delete";
  content: string;
  oldHash?: string;
  newHash: string;
}

export interface HmuResultPayload {
  id: string;
  status: "success" | "partial" | "failed";
  applied: string[];
  failed: string[];
  requiresRestart: boolean;
}

export function applyHmu(changes: FileChange[]): Promise<HmuResultPayload> {
  return invoke<HmuResultPayload>("apply_hmu", { changes });
}
