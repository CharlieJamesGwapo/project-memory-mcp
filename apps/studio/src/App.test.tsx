import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import App from "./App";
import { createSampleAdapter } from "./sample";
import type { MemoryAdapter, Page, Project } from "./types";
describe("workspace interactions", () => {
  it("selects full records, copies and removes context, and clears it when changing projects", async () => {
    const a = createSampleAdapter();
    const copy = vi.fn(async (_markdown: string) => {});
    const save = vi.fn(async () => ({ saved: false, file_name: null }));
    a.copy = copy;
    a.save = save;
    const u = userEvent.setup();
    render(<App adapter={a} />);
    await screen.findByRole("heading", { name: "Fieldwork Atlas" });
    await u.click(screen.getByRole("tab", { name: "Notes" }));
    await u.click(
      await screen.findByRole("button", {
        name: /Open record: Search should feel like looking through a notebook/,
      }),
    );
    await screen.findByRole("heading", {
      name: "Search should feel like looking through a notebook",
      level: 2,
    });
    await u.click(screen.getByRole("button", { name: "Add to context brief" }));
    await u.click(screen.getByRole("button", { name: "Close record" }));
    const brief = screen.getByRole("complementary", { name: "Context brief" });
    expect(
      within(brief).getByText(
        "Search should feel like looking through a notebook",
      ),
    ).toBeTruthy();
    await u.click(within(brief).getByRole("button", { name: "Copy Markdown" }));
    expect(copy).toHaveBeenCalledOnce();
    expect(copy.mock.calls[0][0]).toContain("Preserve literal punctuation");
    await u.click(within(brief).getByRole("button", { name: "Save Markdown" }));
    expect(
      await screen.findByText("Save canceled. Your brief is still here."),
    ).toBeTruthy();
    await u.click(
      within(brief).getByRole("button", { name: /Remove .* from context/ }),
    );
    expect(
      within(brief).getByText("Your next conversation starts here."),
    ).toBeTruthy();
    await u.click(
      await screen.findByRole("button", {
        name: /Open record: Search should feel like looking through a notebook/,
      }),
    );
    await u.click(
      await screen.findByRole("button", { name: "Add to context brief" }),
    );
    await u.click(screen.getByRole("button", { name: "Close record" }));
    await u.click(
      screen.getByRole("button", { name: /Open project: Common Ground/ }),
    );
    await screen.findByRole("heading", { name: "Common Ground" });
    expect(
      within(brief).getByText("Your next conversation starts here."),
    ).toBeTruthy();
  });
  it("shows a missing database with explicit sample choice", async () => {
    const a = createSampleAdapter();
    const adapter: MemoryAdapter = {
      ...a,
      sample: false,
      bootstrap: async () => ({
        status: "missing",
        path: null,
        default_path: "/missing",
        session_id: null,
        error: null,
      }),
    };
    render(<App adapter={adapter} />);
    expect(
      await screen.findByRole("heading", {
        name: "A place for project memory.",
      }),
    ).toBeTruthy();
    expect(
      screen.getByRole("button", { name: "Explore sample workspace" }),
    ).toBeTruthy();
  });
  it("does not add an incomplete record when a fetch fails", async () => {
    const a = createSampleAdapter();
    const original = a.query;
    a.query = async (s, op, args) => {
      if (op === "get_record") throw new Error("sensitive internals");
      return original(s, op, args);
    };
    render(<App adapter={a} />);
    const u = userEvent.setup();
    await screen.findByRole("heading", { name: "Fieldwork Atlas" });
    await u.click(screen.getByRole("tab", { name: "Notes" }));
    await u.click(
      await screen.findByRole("button", {
        name: /Open record: Search should feel like looking through a notebook/,
      }),
    );
    expect(
      await screen.findByText("This record could not be loaded. Try again."),
    ).toBeTruthy();
    expect(screen.queryByText("sensitive internals")).toBeNull();
    expect(
      screen.queryByRole("button", { name: "Add to context brief" }),
    ).toBeNull();
  });
  it("ignores old project requests after navigation", async () => {
    const a = createSampleAdapter();
    const conn = await a.bootstrap();
    const projects = await a.query<Page<Project>>(
      conn.session_id!,
      "studio_list_projects",
      {},
    );
    const firstId = projects.items[0].id;
    const original = a.query;
    let release: (() => void) | undefined;
    a.query = async (s, op, args) => {
      if (op === "get_project_context" && args.project_id === firstId)
        await new Promise<void>((resolve) => {
          release = resolve;
        });
      return original(s, op, args);
    };
    render(<App adapter={a} />);
    const u = userEvent.setup();
    await screen.findByRole("heading", { name: "Fieldwork Atlas" });
    await u.click(
      screen.getByRole("button", { name: /Open project: Common Ground/ }),
    );
    await screen.findByRole("heading", { name: "Common Ground" });
    release?.();
    await waitFor(() =>
      expect(
        screen.getByRole("heading", { name: "Common Ground" }),
      ).toBeTruthy(),
    );
    expect(screen.queryByText("Keep the archive offline-first")).toBeNull();
  });
});

it("searches literal phrases and clears search back to all notes", async () => {
  const a = createSampleAdapter();
  const u = userEvent.setup();
  render(<App adapter={a} />);
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await u.click(screen.getByRole("tab", { name: "Notes" }));
  const input = await screen.findByRole("textbox", { name: "Search notes" });
  await u.type(input, ".*");
  await u.click(screen.getByRole("button", { name: "Search" }));
  expect(
    await screen.findByRole("heading", { name: "No notes match that phrase." }),
  ).toBeTruthy();
  await u.click(screen.getByRole("button", { name: "Clear search" }));
  expect(
    await screen.findByRole("button", {
      name: /Open record: Search should feel/,
    }),
  ).toBeTruthy();
});

it("shows replacement history and all handoff sections with recorded verification", async () => {
  const a = createSampleAdapter();
  const u = userEvent.setup();
  render(<App adapter={a} />);
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await u.click(screen.getByRole("tab", { name: "Decisions" }));
  expect(
    await screen.findByRole("button", {
      name: /Open record: Start with fuzzy/,
    }),
  ).toBeTruthy();
  await u.click(screen.getByRole("checkbox", { name: "Include superseded" }));
  await waitFor(() =>
    expect(
      screen.queryByRole("button", { name: /Open record: Start with fuzzy/ }),
    ).toBeNull(),
  );
  await u.click(screen.getByRole("tab", { name: "Handoffs" }));
  await u.click(
    await screen.findByRole("button", {
      name: /Open record: The archive has a working foundation/,
    }),
  );
  for (const name of [
    "Completed",
    "Unfinished",
    "Next actions",
    "Blockers",
    "Recorded verification · historical",
    "Sources",
  ])
    expect(await screen.findByRole("heading", { name })).toBeTruthy();
  expect(
    screen.getByText("Recorded on 26 Nov: 28 unit tests passed."),
  ).toBeTruthy();
});

it("retains the brief and gives a safe error when clipboard access fails", async () => {
  const a = createSampleAdapter();
  a.copy = async () => {
    throw new Error("raw internal clipboard error");
  };
  const u = userEvent.setup();
  render(<App adapter={a} />);
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await u.click(
    await screen.findByRole("button", {
      name: /Open record: Keep the archive offline-first/,
    }),
  );
  await u.click(
    await screen.findByRole("button", { name: "Add to context brief" }),
  );
  await u.click(screen.getByRole("button", { name: "Close record" }));
  await u.click(screen.getByRole("button", { name: "Copy Markdown" }));
  expect(
    await screen.findByText("Copy failed. Try saving your brief as Markdown."),
  ).toBeTruthy();
  expect(screen.queryByText("raw internal clipboard error")).toBeNull();
  expect(
    within(
      screen.getByRole("complementary", { name: "Context brief" }),
    ).getByText("Keep the archive offline-first"),
  ).toBeTruthy();
});

it("retains current workspace after canceled and failed database selection", async () => {
  const a = createSampleAdapter();
  a.sample = false;
  const choose = vi
    .fn()
    .mockResolvedValueOnce(null)
    .mockRejectedValueOnce({ code: "BAD_DB", message: "raw path error" });
  a.chooseDatabase = choose;
  const u = userEvent.setup();
  render(<App adapter={a} />);
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await u.click(screen.getByRole("button", { name: "Open database" }));
  expect(
    await screen.findByText(
      "Open canceled. Your current workspace is unchanged.",
    ),
  ).toBeTruthy();
  await u.click(screen.getByRole("button", { name: "Open database" }));
  expect(
    await screen.findByText(
      "That archive could not be opened. Choose an existing Project Memory database.",
    ),
  ).toBeTruthy();
  expect(screen.getByRole("heading", { name: "Fieldwork Atlas" })).toBeTruthy();
  expect(screen.queryByText("raw path error")).toBeNull();
});

it("paginates notes without dropping earlier records", async () => {
  const a = createSampleAdapter();
  const original = a.query;
  a.query = (s, op, args) =>
    original(s, op, {
      ...args,
      ...(op === "studio_list_records" && args.kind === "note"
        ? { limit: 1 }
        : {}),
    });
  const u = userEvent.setup();
  render(<App adapter={a} />);
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await u.click(screen.getByRole("tab", { name: "Notes" }));
  await screen.findByRole("button", {
    name: /Open record: Search should feel/,
  });
  await u.click(screen.getByRole("button", { name: "Load more notes" }));
  expect(
    await screen.findByRole("button", { name: /Open record: A small brief/ }),
  ).toBeTruthy();
  expect(
    screen.getByRole("button", { name: /Open record: Search should feel/ }),
  ).toBeTruthy();
});

it("uses native clipboard and save dialogs while exploring samples inside the desktop app", async () => {
  const a = createSampleAdapter();
  const copy = vi.fn(async (_markdown: string) => {});
  const save = vi.fn(async (_markdown: string) => ({
    saved: true,
    file_name: "brief.md",
  }));
  const adapter: MemoryAdapter = {
    ...a,
    sample: false,
    copy,
    save,
    bootstrap: async () => ({
      status: "missing",
      path: null,
      default_path: "/missing",
      session_id: null,
      error: null,
    }),
  };
  const u = userEvent.setup();
  render(<App adapter={adapter} />);
  await u.click(
    await screen.findByRole("button", { name: "Explore sample workspace" }),
  );
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await u.click(
    await screen.findByRole("button", {
      name: /Open record: Keep the archive offline-first/,
    }),
  );
  await u.click(
    await screen.findByRole("button", { name: "Add to context brief" }),
  );
  await u.click(screen.getByRole("button", { name: "Close record" }));
  await u.click(screen.getByRole("button", { name: "Copy Markdown" }));
  expect(copy).toHaveBeenCalledOnce();
  await u.click(screen.getByRole("button", { name: "Save Markdown" }));
  expect(save).toHaveBeenCalledOnce();
  expect(await screen.findByText("Saved as brief.md.")).toBeTruthy();
});

it("discards a late full-record response after closing and changing projects", async () => {
  const a = createSampleAdapter();
  const original = a.query;
  let release: (() => void) | undefined;
  a.query = async (s, op, args) => {
    if (op === "get_record")
      await new Promise<void>((resolve) => {
        release = resolve;
      });
    return original(s, op, args);
  };
  const u = userEvent.setup();
  render(<App adapter={a} />);
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await u.click(
    await screen.findByRole("button", {
      name: /Open record: Keep the archive offline-first/,
    }),
  );
  await screen.findByText("Loading the complete record…");
  await u.click(screen.getByRole("button", { name: "Close record" }));
  await u.click(
    screen.getByRole("button", { name: /Open project: Common Ground/ }),
  );
  release?.();
  await screen.findByRole("heading", { name: "Common Ground" });
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(
    within(
      screen.getByRole("complementary", { name: "Context brief" }),
    ).getByText("Your next conversation starts here."),
  ).toBeTruthy();
});

it("ignores an older search response when a new literal query finishes first", async () => {
  const a = createSampleAdapter();
  const original = a.query;
  let release: (() => void) | undefined;
  a.query = async (s, op, args) => {
    if (op === "search_notes" && args.query === "notebook")
      await new Promise<void>((resolve) => {
        release = resolve;
      });
    return original(s, op, args);
  };
  const u = userEvent.setup();
  render(<App adapter={a} />);
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await u.click(screen.getByRole("tab", { name: "Notes" }));
  const input = screen.getByRole("textbox", { name: "Search notes" });
  await u.type(input, "notebook");
  await u.click(screen.getByRole("button", { name: "Search" }));
  await waitFor(() => expect(release).toBeTypeOf("function"));
  await u.clear(input);
  await u.type(input, "small brief");
  await u.click(screen.getByRole("button", { name: "Search" }));
  await screen.findByRole("button", { name: /Open record: A small brief/ });
  release?.();
  await waitFor(() =>
    expect(
      screen.queryByRole("button", { name: /Open record: Search should feel/ }),
    ).toBeNull(),
  );
});

it("settles a pending database picker after an old query forces session reconnection", async () => {
  const a = createSampleAdapter();
  a.sample = false;
  const originalQuery = a.query;
  const originalBootstrap = a.bootstrap;
  a.bootstrap = async () => ({ ...(await originalBootstrap()) });
  let rejectQuery: (reason: unknown) => void = () => {};
  let resolveChoose: (
    value: Awaited<ReturnType<MemoryAdapter["bootstrap"]>>,
  ) => void = () => {};
  let delayed = false;
  a.query = async (session, operation, args) => {
    if (
      operation === "studio_list_records" &&
      args.kind === "note" &&
      args.limit === 20 &&
      !delayed
    ) {
      delayed = true;
      await new Promise<void>((_resolve, reject) => {
        rejectQuery = reject;
      });
    }
    return originalQuery(session, operation, args);
  };
  a.chooseDatabase = () =>
    new Promise((resolve) => {
      resolveChoose = resolve;
    });
  const u = userEvent.setup();
  render(<App adapter={a} />);
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await u.click(screen.getByRole("tab", { name: "Notes" }));
  await waitFor(() => expect(delayed).toBe(true));
  await u.click(screen.getByRole("button", { name: "Open database" }));
  rejectQuery({ code: "STALE_SESSION" });
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  resolveChoose(await a.bootstrap());
  await waitFor(() =>
    expect(
      (
        screen.getByRole("button", {
          name: /Open database|Choosing archive/,
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(false),
  );
  await waitFor(() =>
    expect(
      (screen.getByRole("button", { name: "Refresh" }) as HTMLButtonElement)
        .disabled,
    ).toBe(false),
  );
});

it("retries the first project page after a refresh failure instead of reusing a previous cursor", async () => {
  const a = createSampleAdapter();
  const originalQuery = a.query;
  const originalBootstrap = a.bootstrap;
  a.bootstrap = async () => ({ ...(await originalBootstrap()) });
  let firstPages = 0;
  a.query = async (session, operation, args) => {
    if (operation === "studio_list_projects" && !args.cursor) {
      if (++firstPages === 2) throw new Error("transient database busy");
      return originalQuery(session, operation, { ...args, limit: 1 });
    }
    return originalQuery(session, operation, args);
  };
  const u = userEvent.setup();
  render(<App adapter={a} />);
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await u.click(screen.getByRole("button", { name: "Refresh" }));
  await screen.findByText("Projects could not be loaded.");
  await u.click(screen.getByRole("button", { name: "Try again" }));
  await waitFor(() =>
    expect(
      screen.queryByRole("button", { name: "Open project: Fieldwork Atlas" }),
    ).not.toBeNull(),
  );
});

it("refreshes the actual sample adapter without losing the project list", async () => {
  const adapter = createSampleAdapter();
  const user = userEvent.setup();
  render(<App adapter={adapter} />);
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  await user.click(screen.getByRole("button", { name: "Refresh" }));
  await screen.findByRole("heading", { name: "Fieldwork Atlas" });
  expect(
    screen.getByRole("button", { name: "Open project: Common Ground" }),
  ).toBeTruthy();
  expect(
    screen.queryByRole("heading", { name: "Every project begins somewhere." }),
  ).toBeNull();
});
