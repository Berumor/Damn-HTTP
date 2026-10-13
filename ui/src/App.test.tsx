import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "./App";
import { commands } from "./bindings";
import { watchSystemTheme } from "./lib/theme";
import { useApp } from "./stores/app";
import { useSettings } from "./stores/settings";

// The generated bindings call into Rust; tests replace them.
vi.mock("./bindings", () => ({
  commands: { appInfo: vi.fn() },
}));

describe("App", () => {
  beforeEach(() => {
    useApp.setState({ info: null });
    useSettings.setState({ theme: "system" });
    vi.mocked(commands.appInfo).mockResolvedValue({
      name: "Damn HTTP",
      version: "1.2.3",
      identifier: "com.damnhttp.app",
      platform: "linux",
    });
  });

  it("shows the app name and the version reported by the backend", async () => {
    render(<App />);

    expect(screen.getByText("Damn HTTP")).toBeInTheDocument();
    expect(await screen.findByText("Version 1.2.3")).toBeInTheDocument();
    expect(commands.appInfo).toHaveBeenCalledTimes(1);
  });

  it("switches and remembers the theme", () => {
    const stop = watchSystemTheme();
    render(<App />);
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(screen.getByRole("radio", { name: "System" })).toBeChecked();

    fireEvent.click(screen.getByRole("radio", { name: "Dark" }));

    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(screen.getByRole("radio", { name: "Dark" })).toBeChecked();
    expect(window.localStorage.getItem("damnhttp.theme")).toBe("dark");
    stop();
  });

  it("follows the operating system while the preference is System", () => {
    vi.mocked(window.matchMedia).mockImplementation(
      (query: string) =>
        ({
          matches: query.includes("dark"),
          media: query,
          addEventListener: vi.fn(),
          removeEventListener: vi.fn(),
        }) as unknown as MediaQueryList,
    );
    const stop = watchSystemTheme();

    expect(document.documentElement.dataset.theme).toBe("dark");
    stop();
  });
});
