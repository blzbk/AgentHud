import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { AgentList, MAX_AGENTS } from "./AgentList";
import type { Agent, ProviderAgents } from "./useAgents";

function agent(sessionId: string, state: Agent["state"], project?: string): Agent {
  return { sessionId, project, state, lastWrite: 1_793_831_988 };
}

function active(agents: Agent[]): ProviderAgents {
  return { provider: "claude", status: "active", agents };
}

describe("AgentList", () => {
  it("shows a labelled badge and the project of each agent", () => {
    render(
      <AgentList
        agents={active([agent("a1", "waiting", "AgentHud"), agent("b2", "idle", "PowerKitchen")])}
      />,
    );

    const items = within(screen.getByRole("list", { name: "agents" })).getAllByRole("listitem");
    expect(items).toHaveLength(2);
    expect(items[0]).toHaveTextContent("waiting");
    expect(items[0]).toHaveTextContent("AgentHud");
    expect(items[1]).toHaveTextContent("idle");
    expect(items[1]).toHaveTextContent("PowerKitchen");
  });

  it("shortens a long session id when the project is unknown", () => {
    const id = "rollout-2026-11-04T12-11-56-01a1a642-2af9-74a1-89d0-225d5070bcb2";
    render(<AgentList agents={active([agent(id, "working")])} />);

    expect(screen.getByRole("listitem")).toHaveTextContent("working");
    expect(screen.getByRole("listitem")).toHaveTextContent("rollout-…70bcb2");
  });

  it("shows a single offline badge, without list, when no agent runs", () => {
    render(<AgentList agents={{ provider: "codex", status: "offline", agents: [] }} />);

    expect(screen.getByText("offline")).toBeInTheDocument();
    expect(screen.queryByRole("list")).not.toBeInTheDocument();
  });

  it("shows unknown when the agents cannot be read", () => {
    render(<AgentList agents={{ provider: "codex", status: "unknown", agents: [] }} />);

    expect(screen.getByText("unknown")).toBeInTheDocument();
    expect(screen.queryByRole("list")).not.toBeInTheDocument();
  });

  it("caps the list and counts the hidden agents", () => {
    const many = Array.from({ length: MAX_AGENTS + 2 }, (_, index) => agent(`s${index}`, "idle"));
    render(<AgentList agents={active(many)} />);

    const items = screen.getAllByRole("listitem");
    expect(items).toHaveLength(MAX_AGENTS + 1);
    expect(items[MAX_AGENTS]).toHaveTextContent("+2");
  });
});
