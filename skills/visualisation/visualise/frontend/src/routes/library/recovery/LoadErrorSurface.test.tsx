import { screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as fetchModule from "../../../api/fetch";
import { renderWithRouterAndQueryAt } from "../../../test/router-helpers";
import { LoadErrorSurface } from "./LoadErrorSurface";

describe("LoadErrorSurface", () => {
  beforeEach(() => vi.restoreAllMocks());

  it("uses a load-failure H1 distinct from the 404 heading", async () => {
    renderWithRouterAndQueryAt(<LoadErrorSurface knownType="plans" />);
    const h1 = await screen.findByRole("heading", { level: 1 });
    expect(h1).toHaveTextContent(/Something went wrong loading this document/i);
    expect(h1.textContent).not.toBe("Document not found");
  });

  it("never renders a Did you mean… block and never fans out to fetchDocs", async () => {
    const spy = vi.spyOn(fetchModule, "fetchDocs").mockResolvedValue([]);
    renderWithRouterAndQueryAt(<LoadErrorSurface knownType="plans" />);
    await screen.findByRole("heading", { level: 1 });
    expect(screen.queryByRole("heading", { name: /Did you mean/i })).toBeNull();
    expect(spy).not.toHaveBeenCalled();
  });

  it("shows back-to-library always and back-to-type only with a known type", async () => {
    const { unmount } = renderWithRouterAndQueryAt(
      <LoadErrorSurface knownType="plans" />,
    );
    expect(
      (
        await screen.findByRole("link", { name: /Back to library/i })
      ).getAttribute("href"),
    ).toBe("/library");
    expect(
      screen
        .getByRole("link", { name: /Back to plan list/i })
        .getAttribute("href"),
    ).toBe("/library/plans");
    unmount();

    renderWithRouterAndQueryAt(<LoadErrorSurface />);
    expect(
      await screen.findByRole("link", { name: /Back to library/i }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: /Back to .* list/i })).toBeNull();
  });

  it("surfaces a provided errorMessage in a role=alert line", async () => {
    renderWithRouterAndQueryAt(
      <LoadErrorSurface knownType="plans" errorMessage="GET /api/docs: 503" />,
    );
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "GET /api/docs: 503",
    );
  });

  it("renders without an alert line when no errorMessage is given", async () => {
    renderWithRouterAndQueryAt(<LoadErrorSurface knownType="plans" />);
    await screen.findByRole("heading", { level: 1 });
    expect(screen.queryByRole("alert")).toBeNull();
  });
});
