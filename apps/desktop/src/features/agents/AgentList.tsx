import styles from "./AgentList.module.css";
import type { Agent, ProviderAgents } from "./useAgents";

/** Most agents listed per provider, so the overlay never grows past its window. */
export const MAX_AGENTS = 4;

/** Ids longer than this are shortened to their start and end. */
const MAX_ID_LENGTH = 16;

/** Project of the agent, or a short form of its session id. */
function agentName(agent: Agent): string {
  if (agent.project) return agent.project;
  const id = agent.sessionId;
  return id.length > MAX_ID_LENGTH ? `${id.slice(0, 8)}…${id.slice(-6)}` : id;
}

function Badge({ state }: { state: string }) {
  return (
    <span className={styles.badge} data-state={state}>
      <span className={styles.dot} aria-hidden="true" />
      {state}
    </span>
  );
}

/** State of each active agent of a provider, or a single offline badge. */
export function AgentList({ agents }: { agents: ProviderAgents }) {
  if (agents.status !== "active" || agents.agents.length === 0) {
    const state = agents.status === "unknown" ? "unknown" : "offline";
    return (
      <p className={styles.single}>
        <Badge state={state} />
      </p>
    );
  }
  const shown = agents.agents.slice(0, MAX_AGENTS);
  const hidden = agents.agents.length - shown.length;
  return (
    <ul className={styles.list} aria-label="agents">
      {shown.map((agent) => (
        <li key={agent.sessionId} className={styles.agent}>
          <Badge state={agent.state} />
          <span className={styles.name} title={agent.sessionId}>
            {agentName(agent)}
          </span>
        </li>
      ))}
      {hidden > 0 && <li className={styles.more}>+{hidden}</li>}
    </ul>
  );
}
