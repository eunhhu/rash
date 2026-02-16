import { createSignal } from "solid-js";

export interface AutoSave {
  trigger: () => void;
  flush: () => Promise<void>;
  cancel: () => void;
  saving: () => boolean;
}

export function createAutoSave(saveFn: () => Promise<void>, delay = 1500): AutoSave {
  const [saving, setSaving] = createSignal(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let pendingSave: Promise<void> | null = null;

  async function doSave() {
    if (saving()) return;
    setSaving(true);
    try {
      await saveFn();
    } finally {
      setSaving(false);
      pendingSave = null;
    }
  }

  function trigger() {
    clearTimeout(timer);
    timer = setTimeout(() => {
      pendingSave = doSave();
    }, delay);
  }

  async function flush() {
    clearTimeout(timer);
    if (pendingSave) {
      await pendingSave;
    } else {
      await doSave();
    }
  }

  function cancel() {
    clearTimeout(timer);
  }

  return { trigger, flush, cancel, saving };
}
