import styles from "./App.module.css";
import { type OverlayMode, useOverlayMode } from "./features/overlay/useOverlayMode";
import { ProviderCard } from "./features/usage/ProviderCard";
import { useUsage } from "./features/usage/useUsage";

const MODE_LABELS: Record<OverlayMode, string> = {
  "click-through": "click-through",
  interactive: "interactif",
};

export function App() {
  const mode = useOverlayMode();
  const usage = useUsage();

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
      {usage.map((provider) => (
        <ProviderCard key={provider.provider} usage={provider} />
      ))}
    </main>
  );
}
