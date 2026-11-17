import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import type React from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import * as fetchModule from "../../api/fetch";
import { WorkItemProblemBanner } from "./WorkItemProblemBanner";

function Wrap({ children }: { children: React.ReactNode }) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return <QueryClientProvider client={qc}>{children}</QueryClientProvider>;
}

describe("WorkItemProblemBanner", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("shows why work-item IDs cannot be trusted", async () => {
    vi.spyOn(fetchModule, "fetchWorkItemConfig").mockResolvedValue({
      problem: "E_REKEY_REQUIRED: 1 work item is not under the ID",
    });

    render(
      <Wrap>
        <WorkItemProblemBanner />
      </Wrap>,
    );

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "E_REKEY_REQUIRED: 1 work item is not under the ID",
    );
  });

  it("says what is switched off and that the problem clears by itself", async () => {
    vi.spyOn(fetchModule, "fetchWorkItemConfig").mockResolvedValue({
      problem: "E_REKEY_REQUIRED: 1 work item is not under the ID",
      restartNeeded: false,
    });

    render(
      <Wrap>
        <WorkItemProblemBanner />
      </Wrap>,
    );

    const banner = await screen.findByRole("alert");
    expect(banner).toHaveTextContent(
      "Work-item IDs are switched off: nothing clusters or links by work-item ID",
    );
    expect(banner).toHaveTextContent(
      "This clears by itself once the work items are fixed.",
    );
  });

  it("says a configuration problem needs a restart", async () => {
    vi.spyOn(fetchModule, "fetchWorkItemConfig").mockResolvedValue({
      problem: "E_WORK_ID_PATTERN_INVALID: work.id_pattern 'x': bad",
      restartNeeded: true,
    });

    render(
      <Wrap>
        <WorkItemProblemBanner />
      </Wrap>,
    );

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Restart the visualiser once the configuration is fixed.",
    );
  });

  it("shows nothing when work-item IDs can be trusted", async () => {
    const spy = vi
      .spyOn(fetchModule, "fetchWorkItemConfig")
      .mockResolvedValue({ key: "PP" });

    render(
      <Wrap>
        <WorkItemProblemBanner />
      </Wrap>,
    );

    await vi.waitFor(() => expect(spy).toHaveBeenCalled());
    expect(screen.queryByRole("alert")).toBeNull();
  });
});
