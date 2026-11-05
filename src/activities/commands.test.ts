import { invoke } from "@tauri-apps/api/core";
import { describe, expect, it, vi } from "vitest";
import { deleteActivity } from "./commands";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(async () => undefined) }));

describe("deleteActivity", () => {
  it("names the Activity as the Rust command expects", async () => {
    await deleteActivity(7);

    expect(invoke).toHaveBeenCalledWith("delete_activity", { id: 7 });
  });
});
