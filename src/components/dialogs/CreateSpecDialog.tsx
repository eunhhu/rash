import { Component, Show, createSignal, createEffect, onCleanup, Switch, Match } from "solid-js";
import { useUiStore } from "../../stores/uiStore";
import { useSpecStore } from "../../stores/specStore";
import {
  defaultRoute,
  defaultSchema,
  defaultModel,
  defaultMiddleware,
  defaultHandler,
} from "../../utils/defaults";
import type { MiddlewareType } from "../../ipc/types";

const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE"] as const;
const MIDDLEWARE_TYPES: MiddlewareType[] = ["request", "response", "error", "composed"];

const KIND_LABELS: Record<string, string> = {
  route: "Route",
  schema: "Schema",
  model: "Model",
  middleware: "Middleware",
  handler: "Handler",
};

export const CreateSpecDialog: Component = () => {
  const { createDialog, setCreateDialog } = useUiStore();
  const { createSpec } = useSpecStore();

  const [name, setName] = createSignal("");
  const [path, setPath] = createSignal("/");
  const [methods, setMethods] = createSignal<Set<string>>(new Set(["GET"]));
  const [tableName, setTableName] = createSignal("");
  const [middlewareType, setMiddlewareType] = createSignal<MiddlewareType>("request");
  const [isAsync, setIsAsync] = createSignal(true);

  // Reset form when dialog opens
  createEffect(() => {
    if (createDialog()) {
      setName("");
      setPath("/");
      setMethods(new Set(["GET"]));
      setTableName("");
      setMiddlewareType("request");
      setIsAsync(true);
    }
  });

  // Auto-generate tableName from model name
  createEffect(() => {
    const n = name();
    if (createDialog()?.kind === "model" && n) {
      setTableName(n.toLowerCase() + "s");
    }
  });

  const handleClose = () => {
    setCreateDialog(null);
  };

  createEffect(() => {
    if (!createDialog()) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") handleClose();
    };
    window.addEventListener("keydown", handler);
    onCleanup(() => window.removeEventListener("keydown", handler));
  });

  const toggleMethod = (method: string) => {
    setMethods((prev) => {
      const next = new Set(prev);
      if (next.has(method)) {
        if (next.size > 1) next.delete(method);
      } else {
        next.add(method);
      }
      return next;
    });
  };

  const handleSubmit = async () => {
    const dialog = createDialog();
    if (!dialog) return;

    const kind = dialog.kind;
    let specName = name().trim();
    let data: unknown;

    switch (kind) {
      case "route": {
        specName = specName || path().replace(/\//g, "_").replace(/^_/, "") || "new-route";
        const route = defaultRoute(path());
        // Build methods object from selected methods
        const methodsObj: Record<string, unknown> = {};
        for (const m of methods()) {
          methodsObj[m] = { handler: { ref: "" } };
        }
        route.methods = methodsObj as typeof route.methods;
        data = route;
        break;
      }
      case "schema":
        if (!specName) return;
        data = defaultSchema(specName);
        break;
      case "model":
        if (!specName) return;
        data = defaultModel(specName);
        (data as { tableName: string }).tableName = tableName() || specName.toLowerCase() + "s";
        break;
      case "middleware":
        if (!specName) return;
        data = defaultMiddleware(specName, middlewareType());
        break;
      case "handler":
        if (!specName) return;
        data = defaultHandler(specName);
        (data as { async: boolean }).async = isAsync();
        break;
      default:
        return;
    }

    try {
      await createSpec(kind as "route" | "schema" | "model" | "middleware" | "handler", specName, data);
      setCreateDialog(null);
    } catch {
      // Error handled by specStore via notifications
    }
  };

  return (
    <Show when={createDialog()}>
      {(dialog) => (
        <div
          class="fixed inset-0 z-[1000] flex items-center justify-center bg-black/55"
          onClick={(e) => {
            if (e.target === e.currentTarget) handleClose();
          }}
        >
          <div class="glass-panel p-0 min-w-[400px] max-w-[480px]">
            <div class="px-4 py-3 border-b border-rash-border">
              <h2 class="text-sm font-semibold text-rash-text">
                New {KIND_LABELS[dialog().kind] ?? dialog().kind}
              </h2>
            </div>

            <div class="px-4 py-4 flex flex-col gap-3">
              <Switch>
                {/* Route form */}
                <Match when={dialog().kind === "route"}>
                  <div class="flex flex-col gap-1">
                    <label class="text-xs font-medium text-rash-text-secondary">Path</label>
                    <input
                      class="carbon-input text-xs"
                      value={path()}
                      onInput={(e) => setPath(e.currentTarget.value)}
                      placeholder="/"
                    />
                  </div>
                  <div class="flex flex-col gap-1">
                    <label class="text-xs font-medium text-rash-text-secondary">Methods</label>
                    <div class="flex gap-2 flex-wrap">
                      {METHODS.map((m) => (
                        <label class="flex items-center gap-1.5 text-xs text-rash-text-secondary cursor-pointer">
                          <input
                            type="checkbox"
                            checked={methods().has(m)}
                            onChange={() => toggleMethod(m)}
                            class="accent-rash-accent"
                          />
                          {m}
                        </label>
                      ))}
                    </div>
                  </div>
                  <div class="flex flex-col gap-1">
                    <label class="text-xs font-medium text-rash-text-secondary">Name (optional)</label>
                    <input
                      class="carbon-input text-xs"
                      value={name()}
                      onInput={(e) => setName(e.currentTarget.value)}
                      placeholder="Auto-generated from path"
                    />
                  </div>
                </Match>

                {/* Schema form */}
                <Match when={dialog().kind === "schema"}>
                  <div class="flex flex-col gap-1">
                    <label class="text-xs font-medium text-rash-text-secondary">Name</label>
                    <input
                      class="carbon-input text-xs"
                      value={name()}
                      onInput={(e) => setName(e.currentTarget.value)}
                      placeholder="e.g. User, Auth"
                      autofocus
                    />
                  </div>
                </Match>

                {/* Model form */}
                <Match when={dialog().kind === "model"}>
                  <div class="flex flex-col gap-1">
                    <label class="text-xs font-medium text-rash-text-secondary">Name</label>
                    <input
                      class="carbon-input text-xs"
                      value={name()}
                      onInput={(e) => setName(e.currentTarget.value)}
                      placeholder="e.g. User, Post"
                      autofocus
                    />
                  </div>
                  <div class="flex flex-col gap-1">
                    <label class="text-xs font-medium text-rash-text-secondary">Table Name</label>
                    <input
                      class="carbon-input text-xs"
                      value={tableName()}
                      onInput={(e) => setTableName(e.currentTarget.value)}
                      placeholder="Auto-generated from name"
                    />
                  </div>
                </Match>

                {/* Middleware form */}
                <Match when={dialog().kind === "middleware"}>
                  <div class="flex flex-col gap-1">
                    <label class="text-xs font-medium text-rash-text-secondary">Name</label>
                    <input
                      class="carbon-input text-xs"
                      value={name()}
                      onInput={(e) => setName(e.currentTarget.value)}
                      placeholder="e.g. auth, cors"
                      autofocus
                    />
                  </div>
                  <div class="flex flex-col gap-1">
                    <label class="text-xs font-medium text-rash-text-secondary">Type</label>
                    <select
                      class="carbon-input text-xs"
                      value={middlewareType()}
                      onChange={(e) => setMiddlewareType(e.currentTarget.value as MiddlewareType)}
                    >
                      {MIDDLEWARE_TYPES.map((t) => (
                        <option value={t}>{t}</option>
                      ))}
                    </select>
                  </div>
                </Match>

                {/* Handler form */}
                <Match when={dialog().kind === "handler"}>
                  <div class="flex flex-col gap-1">
                    <label class="text-xs font-medium text-rash-text-secondary">Name</label>
                    <input
                      class="carbon-input text-xs"
                      value={name()}
                      onInput={(e) => setName(e.currentTarget.value)}
                      placeholder="e.g. getUsers, createPost"
                      autofocus
                    />
                  </div>
                  <div class="flex items-center gap-2">
                    <label class="flex items-center gap-1.5 text-xs text-rash-text-secondary cursor-pointer">
                      <input
                        type="checkbox"
                        checked={isAsync()}
                        onChange={(e) => setIsAsync(e.currentTarget.checked)}
                        class="accent-rash-accent"
                      />
                      Async
                    </label>
                  </div>
                </Match>
              </Switch>
            </div>

            <div class="flex items-center justify-end gap-2 px-4 py-3 border-t border-rash-border">
              <button class="carbon-btn-secondary text-xs" onClick={handleClose}>
                Cancel
              </button>
              <button class="carbon-btn-primary text-xs" onClick={handleSubmit}>
                Create
              </button>
            </div>
          </div>
        </div>
      )}
    </Show>
  );
};
