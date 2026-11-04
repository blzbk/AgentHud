import { useEffect, useState } from "react";
import { formatAge, formatPercent, formatReset, nowInSeconds } from "./format";
import styles from "./ProviderCard.module.css";
import type { ProviderUsage, WindowUsage } from "./useUsage";

const PROVIDER_NAMES: Record<ProviderUsage["provider"], string> = {
  codex: "Codex",
  claude: "Claude",
};

/** How often resets and ages are recomputed between two native updates. */
const CLOCK_INTERVAL_MS = 60_000;

function useNow(): number {
  const [now, setNow] = useState(nowInSeconds);
  useEffect(() => {
    const timer = setInterval(() => setNow(nowInSeconds()), CLOCK_INTERVAL_MS);
    return () => clearInterval(timer);
  }, []);
  return now;
}

type WindowRowProps = {
  label: string;
  usage: WindowUsage | undefined;
  now: number;
};

function WindowRow({ label, usage, now }: WindowRowProps) {
  if (!usage) {
    return (
      <div className={styles.row}>
        <span className={styles.label}>{label}</span>
        <span className={styles.unknown}>unknown</span>
      </div>
    );
  }
  const { resetsAt } = usage;
  const pendingReset = resetsAt !== undefined && resetsAt > now ? resetsAt : undefined;
  // Once the window has reset, its usage starts over from 0 %.
  const percent = resetsAt !== undefined && pendingReset === undefined ? 0 : usage.usedPercent;
  return (
    <div className={styles.row}>
      <span className={styles.label}>{label}</span>
      <div
        className={styles.gauge}
        role="meter"
        aria-label={label}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percent}
      >
        <div className={styles.fill} style={{ width: `${percent}%` }} />
      </div>
      <span className={styles.percent}>{formatPercent(percent)}</span>
      {pendingReset !== undefined && (
        <span className={styles.reset}>↻ {formatReset(pendingReset, now)}</span>
      )}
    </div>
  );
}

/** Usage of one provider: a gauge per quota window, its reset and the data age. */
export function ProviderCard({ usage }: { usage: ProviderUsage }) {
  const now = useNow();
  const name = PROVIDER_NAMES[usage.provider];
  const known = usage.status === "known";

  return (
    <section className={styles.card} aria-label={name}>
      <header className={styles.header}>
        <h2 className={styles.name}>{name}</h2>
        {known && usage.observedAt !== undefined && (
          <span className={styles.age}>{formatAge(usage.observedAt, now)}</span>
        )}
      </header>
      <WindowRow label="5h" usage={known ? usage.fiveHours : undefined} now={now} />
      <WindowRow label="hebdo" usage={known ? usage.weekly : undefined} now={now} />
    </section>
  );
}
