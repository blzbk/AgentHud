import { act, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ProviderCard } from "./ProviderCard";
import type { ProviderUsage } from "./useUsage";

/** 2026-11-04 13:00 local time, in seconds. */
const NOW = new Date(2026, 10, 4, 13, 0).getTime() / 1000;

function at(hour: number, minute: number): number {
  return new Date(2026, 10, 4, hour, minute).getTime() / 1000;
}

function row(label: string): HTMLElement {
  const gauge = screen.queryByRole("meter", { name: label });
  const cell = gauge ?? screen.getByText(label);
  const element = cell.parentElement;
  if (!element) throw new Error(`no row for ${label}`);
  return element;
}

describe("ProviderCard", () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ["Date", "setInterval", "clearInterval"] });
    vi.setSystemTime(NOW * 1000);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  const known: ProviderUsage = {
    provider: "codex",
    status: "known",
    fiveHours: { usedPercent: 12, resetsAt: at(17, 40) },
    weekly: { usedPercent: 51, resetsAt: NOW + 5 * 86400 },
    observedAt: NOW - 3 * 60,
  };

  it("shows a gauge, a percent and a reset per window", () => {
    render(<ProviderCard usage={known} />);

    expect(screen.getByRole("heading", { name: "Codex" })).toBeInTheDocument();
    expect(screen.getByRole("meter", { name: "5h" })).toHaveAttribute("aria-valuenow", "12");
    expect(row("5h")).toHaveTextContent("12 %");
    expect(row("5h")).toHaveTextContent("↻ 17:40");
    expect(screen.getByRole("meter", { name: "hebdo" })).toHaveAttribute("aria-valuenow", "51");
    expect(row("hebdo")).toHaveTextContent("51 %");
  });

  it("shows the age of the data", () => {
    render(<ProviderCard usage={known} />);

    expect(screen.getByText("il y a 3 min")).toBeInTheDocument();
  });

  it("shows unknown, never 0 %, when the usage is unknown", () => {
    render(<ProviderCard usage={{ provider: "codex", status: "unknown" }} />);

    const card = screen.getByRole("region", { name: "Codex" });
    expect(within(card).getAllByText("unknown")).toHaveLength(2);
    expect(within(card).queryByRole("meter")).not.toBeInTheDocument();
    expect(card).not.toHaveTextContent("%");
  });

  it("shows unknown for a window the provider did not report", () => {
    render(<ProviderCard usage={{ ...known, weekly: undefined }} />);

    expect(row("hebdo")).toHaveTextContent("unknown");
    expect(row("5h")).toHaveTextContent("12 %");
  });

  it("drops a window to 0 % once its reset time passes", () => {
    render(<ProviderCard usage={{ ...known, fiveHours: { usedPercent: 12, resetsAt: NOW + 30 } }} />);
    expect(row("5h")).toHaveTextContent("12 %");

    act(() => {
      vi.advanceTimersByTime(60_000);
    });

    expect(screen.getByRole("meter", { name: "5h" })).toHaveAttribute("aria-valuenow", "0");
    expect(row("5h")).toHaveTextContent("0 %");
    expect(row("5h")).not.toHaveTextContent("↻");
  });

  const claude: ProviderUsage = {
    provider: "claude",
    status: "known",
    fiveHours: { usedTokens: 1_200_000, resetsAt: at(17, 0), estimated: true },
    observedAt: NOW - 10,
  };

  it("shows Claude in estimated tokens, without a gauge nor a weekly window", () => {
    render(<ProviderCard usage={claude} />);

    const card = screen.getByRole("region", { name: "Claude" });
    expect(row("5h")).toHaveTextContent("1,2 M tokens");
    expect(row("5h")).toHaveTextContent("↻ ~17:00 estimé");
    expect(within(card).queryByRole("meter")).not.toBeInTheDocument();
    expect(within(card).queryByText("hebdo")).not.toBeInTheDocument();
  });

  it("drops the estimated tokens to 0 once the block ends", () => {
    const ended = { usedTokens: 1_200_000, resetsAt: NOW - 1, estimated: true };
    render(<ProviderCard usage={{ ...claude, fiveHours: ended }} />);

    expect(row("5h")).toHaveTextContent("0 tokens");
    expect(row("5h")).toHaveTextContent("estimé");
    expect(row("5h")).not.toHaveTextContent("↻");
  });

  it("shows Claude unknown, never 0 tokens, when its usage is unknown", () => {
    render(<ProviderCard usage={{ provider: "claude", status: "unknown" }} />);

    const card = screen.getByRole("region", { name: "Claude" });
    expect(within(card).getAllByText("unknown")).toHaveLength(1);
    expect(card).not.toHaveTextContent("tokens");
  });

  it("shows its children below the usage", () => {
    render(
      <ProviderCard usage={claude}>
        <p>agents</p>
      </ProviderCard>,
    );

    const card = screen.getByRole("region", { name: "Claude" });
    expect(within(card).getByText("agents")).toBeInTheDocument();
  });

  const claudeExact: ProviderUsage = {
    provider: "claude",
    status: "known",
    fiveHours: { usedPercent: 30, resetsAt: at(18, 0) },
    weekly: { usedPercent: 60, resetsAt: NOW + 3 * 86400 },
    observedAt: NOW - 10,
    source: "statusline",
  };

  it("shows Claude gauges like Codex with an exact source", () => {
    render(<ProviderCard usage={claudeExact} />);

    const card = screen.getByRole("region", { name: "Claude" });
    expect(screen.getByRole("meter", { name: "5h" })).toHaveAttribute("aria-valuenow", "30");
    expect(screen.getByRole("meter", { name: "hebdo" })).toHaveAttribute("aria-valuenow", "60");
    expect(row("5h")).toHaveTextContent("↻ 18:00");
    expect(row("5h")).not.toHaveTextContent("~");
    expect(card).not.toHaveTextContent("tokens");
    expect(within(card).getByText("statusline")).toBeInTheDocument();
  });

  it("flags the OAuth source as risky", () => {
    render(<ProviderCard usage={{ ...claudeExact, source: "oauth" }} />);

    expect(screen.getByText("OAuth ⚠")).toBeInTheDocument();
    expect(screen.getByRole("meter", { name: "hebdo" })).toBeInTheDocument();
  });

  it("names the estimate of Claude", () => {
    render(<ProviderCard usage={{ ...claude, source: "estimate" }} />);

    const card = screen.getByRole("region", { name: "Claude" });
    expect(within(card).getByText("estimation")).toBeInTheDocument();
    expect(within(card).queryByText("hebdo")).not.toBeInTheDocument();
    expect(row("5h")).toHaveTextContent("1,2 M tokens");
  });

  it("shows unknown for an exact Claude window that is missing", () => {
    render(<ProviderCard usage={{ ...claudeExact, fiveHours: undefined }} />);

    expect(row("5h")).toHaveTextContent("unknown");
    expect(row("hebdo")).toHaveTextContent("60 %");
  });

  it("names the Codex source", () => {
    const { rerender } = render(<ProviderCard usage={{ ...known, source: "app-server" }} />);
    expect(screen.getByText("app-server")).toBeInTheDocument();

    rerender(<ProviderCard usage={{ ...known, source: "sessions" }} />);
    expect(screen.getByText("session")).toBeInTheDocument();
  });

  it("warns when a login has expired, even without usage", () => {
    render(<ProviderCard usage={{ provider: "claude", status: "unknown", authExpired: true }} />);

    const card = screen.getByRole("region", { name: "Claude" });
    expect(within(card).getByText("auth expirée")).toBeInTheDocument();
    expect(within(card).getAllByText("unknown")).toHaveLength(1);
  });

  it("shows no source nor warning when they are not reported", () => {
    render(<ProviderCard usage={known} />);

    const card = screen.getByRole("region", { name: "Codex" });
    expect(card).not.toHaveTextContent("auth expirée");
    expect(card).not.toHaveTextContent("session");
  });
});
