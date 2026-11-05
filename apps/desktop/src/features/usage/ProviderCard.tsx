import { type ReactNode, useEffect, useState } from "react";
import { formatAge, formatPercent, formatReset, formatTokens, nowInSeconds } from "./format";
import styles from "./ProviderCard.module.css";
import type { Provider, ProviderUsage, UsageSource, WindowUsage } from "./useUsage";

export const PROVIDER_NAMES: Record<Provider, string> = {
  codex: "Codex",
  claude: "Claude",
};

type WindowKind = "fiveHours" | "weekly";

const WINDOW_LABELS: Record<WindowKind, string> = {
  fiveHours: "5h",
  weekly: "hebdo",
};

const ALL_WINDOWS: readonly WindowKind[] = ["fiveHours", "weekly"];

/** Sources that publish a share of the quota, not an estimate in tokens. */
const EXACT_SOURCES: ReadonlySet<UsageSource> = new Set(["sessions", "app-server", "statusline", "oauth"]);

/** Label of each source, shown next to the provider name. */
export const SOURCE_LABELS: Record<UsageSource, string> = {
  sessions: "session",
  "app-server": "app-server",
  statusline: "statusline",
  oauth: "OAuth ⚠",
  estimate: "estimation",
};

/**
 * Windows to show: Codex always publishes both. Claude shows both only with
 * an exact source, its estimate covers the 5-hour block only.
 */
function windowsOf(usage: ProviderUsage): readonly WindowKind[] {
  if (usage.provider === "codex") return ALL_WINDOWS;
  const exact = usage.status === "known" && usage.source !== undefined && EXACT_SOURCES.has(usage.source);
  return exact ? ALL_WINDOWS : ["fiveHours"];
}

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

function UnknownRow({ label }: { label: string }) {
  return (
    <div className={styles.row}>
      <span className={styles.label}>{label}</span>
      <span className={styles.unknown}>unknown</span>
    </div>
  );
}

function WindowRow({ label, usage, now }: WindowRowProps) {
  if (!usage || (usage.usedPercent === undefined && usage.usedTokens === undefined)) {
    return <UnknownRow label={label} />;
  }
  const { resetsAt, estimated } = usage;
  const pendingReset = resetsAt !== undefined && resetsAt > now ? resetsAt : undefined;
  // Once the window has reset, its usage starts over from 0.
  const reset = resetsAt !== undefined && pendingReset === undefined;
  const resetLabel =
    pendingReset !== undefined && `↻ ${estimated ? "~" : ""}${formatReset(pendingReset, now)}`;

  if (usage.usedTokens !== undefined) {
    return (
      <div className={`${styles.row} ${styles.tokens}`}>
        <span className={styles.label}>{label}</span>
        <span>{formatTokens(reset ? 0 : usage.usedTokens)}</span>
        <span className={styles.reset}>
          {resetLabel}
          {estimated && <span className={styles.estimated}> estimé</span>}
        </span>
      </div>
    );
  }
  const percent = reset ? 0 : (usage.usedPercent ?? 0);
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
      {resetLabel && <span className={styles.reset}>{resetLabel}</span>}
    </div>
  );
}

type ProviderCardProps = {
  usage: ProviderUsage;
  /** Shown below the usage, such as the provider's agents. */
  children?: ReactNode;
};

/**
 * Usage of one provider: a row per quota window, in percent with a gauge or
 * in tokens, its reset, the source and age of the data, and a warning when a
 * login has expired.
 */
export function ProviderCard({ usage, children }: ProviderCardProps) {
  const now = useNow();
  const name = PROVIDER_NAMES[usage.provider];
  const known = usage.status === "known";

  return (
    <section className={styles.card} aria-label={name}>
      <header className={styles.header}>
        <h2 className={styles.name}>{name}</h2>
        {known && usage.source !== undefined && (
          <span className={styles.source} data-source={usage.source}>
            {SOURCE_LABELS[usage.source]}
          </span>
        )}
        {known && usage.observedAt !== undefined && (
          <span className={styles.age}>{formatAge(usage.observedAt, now)}</span>
        )}
      </header>
      {windowsOf(usage).map((kind) => (
        <WindowRow
          key={kind}
          label={WINDOW_LABELS[kind]}
          usage={known ? usage[kind] : undefined}
          now={now}
        />
      ))}
      {usage.authExpired && <p className={styles.warning}>auth expirée</p>}
      {children}
    </section>
  );
}
