import styles from "./App.module.css";
import { type OverlayMode, useOverlayMode } from "./features/overlay/useOverlayMode";

const MODE_LABELS: Record<OverlayMode, string> = {
  "click-through": "click-through",
  interactive: "interactif",
};

export function App() {
  const mode = useOverlayMode();

  return (
    <main className={styles.hud}>
      <header className={styles.header}>
        <h1 className={styles.title}>AgentHud</h1>
        <span className={styles.mode} data-mode={mode} role="status">
          <span className={styles.dot} aria-hidden="true" />
          {MODE_LABELS[mode]}
        </span>
      </header>
    </main>
  );
}
