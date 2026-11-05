import styles from "./App.module.css";
import { AgentList } from "./features/agents/AgentList";
import { type ProviderAgents, useAgents } from "./features/agents/useAgents";
import { type OverlayMode, useOverlayMode } from "./features/overlay/useOverlayMode";
import { ProviderCard } from "./features/usage/ProviderCard";
import { PROVIDERS, type ProviderUsage, useUsage } from "./features/usage/useUsage";

const MODE_LABELS: Record<OverlayMode, string> = {
  "click-through": "click-through",
  interactive: "interactif",
};

export function App() {
  const mode = useOverlayMode();
  const usage = useUsage();
  const agents = useAgents();

  return (
    <main className={styles.hud}>
      {/* Dragging needs the cursor, so only the interactive mode can move the window. */}
      <header
        className={styles.header}
        data-tauri-drag-region={mode === "interactive" ? "deep" : undefined}
      >
        <h1 className={styles.title}>AgentHud</h1>
        <span className={styles.mode} data-mode={mode} role="status">
          <span className={styles.dot} aria-hidden="true" />
          {MODE_LABELS[mode]}
        </span>
      </header>
      {PROVIDERS.map((provider) => {
        const providerUsage: ProviderUsage = usage.find((entry) => entry.provider === provider) ?? {
          provider,
          status: "unknown",
        };
        const providerAgents: ProviderAgents = agents.find(
          (entry) => entry.provider === provider,
        ) ?? { provider, status: "unknown", agents: [] };
        return (
          <ProviderCard key={provider} usage={providerUsage}>
            <AgentList agents={providerAgents} />
          </ProviderCard>
        );
      })}
    </main>
  );
}
