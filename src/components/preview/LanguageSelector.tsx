import { Component, For } from "solid-js";
import type { Language, Framework } from "../../ipc/types";

interface LanguageSelectorProps {
  language: Language;
  framework: Framework;
  onLanguageChange: (language: Language) => void;
  onFrameworkChange: (framework: Framework) => void;
}

const LANGUAGES: Language[] = ["typescript", "rust", "python", "go"];

const FRAMEWORK_MAP: Record<Language, Framework[]> = {
  typescript: ["express", "fastify", "hono", "elysia", "nestjs"],
  rust: ["actix", "axum", "rocket"],
  python: ["fastapi", "django", "flask"],
  go: ["gin", "echo", "fiber"],
};

export const LanguageSelector: Component<LanguageSelectorProps> = (props) => {
  const frameworks = () => FRAMEWORK_MAP[props.language] ?? [];

  return (
    <div class="language-selector">
      <div class="language-selector-group">
        <label class="language-selector-label">Language</label>
        <select
          value={props.language}
          onChange={(e) => {
            const lang = e.currentTarget.value as Language;
            props.onLanguageChange(lang);
            // Auto-select first framework for new language
            const fws = FRAMEWORK_MAP[lang] ?? [];
            if (fws.length > 0 && !fws.includes(props.framework)) {
              props.onFrameworkChange(fws[0]);
            }
          }}
        >
          <For each={LANGUAGES}>
            {(lang) => <option value={lang}>{lang}</option>}
          </For>
        </select>
      </div>

      <div class="language-selector-group">
        <label class="language-selector-label">Framework</label>
        <select
          value={props.framework}
          onChange={(e) => props.onFrameworkChange(e.currentTarget.value as Framework)}
        >
          <For each={frameworks()}>
            {(fw) => <option value={fw}>{fw}</option>}
          </For>
        </select>
      </div>

      <style>{`
        .language-selector {
          display: flex;
          gap: 12px;
          align-items: center;
        }
        .language-selector-group {
          display: flex;
          align-items: center;
          gap: 6px;
        }
        .language-selector-label {
          font-size: 11px;
          color: var(--rash-text-muted);
          white-space: nowrap;
        }
        .language-selector select {
          font-size: 12px;
          padding: 3px 8px;
        }
      `}</style>
    </div>
  );
};
