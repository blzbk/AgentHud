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
});
