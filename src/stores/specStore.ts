import * as cmd from "../ipc/commands";
import { useProjectStore } from "./projectStore";
import { useEditorStore } from "./editorStore";
import { useNotificationStore } from "./notificationStore";

type SpecKind = "route" | "schema" | "model" | "middleware" | "handler";

const SPEC_CONFIG: Record<SpecKind, { dir: string; ext: string }> = {
  route: { dir: "routes", ext: "route.json" },
  schema: { dir: "schemas", ext: "schema.json" },
  model: { dir: "models", ext: "model.json" },
  middleware: { dir: "middleware", ext: "middleware.json" },
  handler: { dir: "handlers", ext: "handler.json" },
};

const WRITE_FNS: Record<SpecKind, (filePath: string, value: unknown) => Promise<unknown>> = {
  route: cmd.writeRoute as (fp: string, v: unknown) => Promise<unknown>,
  schema: cmd.writeSchema as (fp: string, v: unknown) => Promise<unknown>,
  model: cmd.writeModel as (fp: string, v: unknown) => Promise<unknown>,
  middleware: cmd.writeMiddleware as (fp: string, v: unknown) => Promise<unknown>,
  handler: cmd.writeHandler as (fp: string, v: unknown) => Promise<unknown>,
};

const DELETE_FNS: Record<SpecKind, (filePath: string) => Promise<void>> = {
  route: cmd.deleteRoute,
  schema: cmd.deleteSchema,
  model: cmd.deleteModel,
  middleware: cmd.deleteMiddleware,
  handler: cmd.deleteHandler,
};

const MOVE_FNS: Record<SpecKind, (oldPath: string, newPath: string) => Promise<void>> = {
  route: cmd.moveRoute,
  schema: cmd.moveSchema,
  model: cmd.moveModel,
  middleware: cmd.moveMiddleware,
  handler: cmd.moveHandler,
};

const READ_FNS: Record<SpecKind, (filePath: string) => Promise<unknown>> = {
  route: cmd.readRoute,
  schema: cmd.readSchema,
  model: cmd.readModel,
  middleware: cmd.readMiddleware,
  handler: cmd.readHandler,
};

function createSpecStore() {
  const toast = useNotificationStore();
  const project = useProjectStore();
  const editor = useEditorStore();

  async function createSpec(kind: SpecKind, name: string, data: unknown): Promise<void> {
    const config = SPEC_CONFIG[kind];
    const filePath = `${config.dir}/${name}.${config.ext}`;
    try {
      await WRITE_FNS[kind](filePath, data);
      await project.refreshTree();
      toast.success(`Created ${kind}: ${name}`);
      editor.openTab(filePath, name, kind, filePath);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : `Failed to create ${kind}`);
      throw err;
    }
  }

  async function deleteSpec(kind: SpecKind, filePath: string, label: string): Promise<void> {
    try {
      await DELETE_FNS[kind](filePath);
      await project.refreshTree();
      editor.closeTab(filePath);
      toast.success(`Deleted ${kind}: ${label}`);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : `Failed to delete ${kind}`);
      throw err;
    }
  }

  async function renameSpec(kind: SpecKind, oldPath: string, newName: string): Promise<void> {
    const config = SPEC_CONFIG[kind];
    const newPath = `${config.dir}/${newName}.${config.ext}`;
    if (oldPath === newPath) return;
    try {
      await MOVE_FNS[kind](oldPath, newPath);
      await project.refreshTree();
      // Update open tab if it was renamed
      editor.updateTabPath(oldPath, newPath, newName);
      toast.success(`Renamed to ${newName}`);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : `Failed to rename ${kind}`);
      throw err;
    }
  }

  async function duplicateSpec(kind: SpecKind, sourcePath: string, newName: string): Promise<void> {
    try {
      const data = await READ_FNS[kind](sourcePath);
      if (data && typeof data === "object" && "name" in data) {
        (data as Record<string, unknown>).name = newName;
      }
      const config = SPEC_CONFIG[kind];
      const newPath = `${config.dir}/${newName}.${config.ext}`;
      await WRITE_FNS[kind](newPath, data);
      await project.refreshTree();
      toast.success(`Duplicated as ${newName}`);
      editor.openTab(newPath, newName, kind, newPath);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : `Failed to duplicate ${kind}`);
      throw err;
    }
  }

  async function moveSpec(kind: SpecKind, oldPath: string, newPath: string): Promise<void> {
    if (oldPath === newPath) return;
    try {
      await MOVE_FNS[kind](oldPath, newPath);
      await project.refreshTree();
      const newName = newPath.split("/").pop()?.replace(/\.[^.]+\.json$/, "") ?? "";
      editor.updateTabPath(oldPath, newPath, newName);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : `Failed to move ${kind}`);
      throw err;
    }
  }

  return { createSpec, deleteSpec, renameSpec, duplicateSpec, moveSpec };
}

let store: ReturnType<typeof createSpecStore> | undefined;

export function useSpecStore() {
  if (!store) {
    store = createSpecStore();
  }
  return store;
}

export type { SpecKind };
