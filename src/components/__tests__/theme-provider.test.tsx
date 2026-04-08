import { render, act } from "@testing-library/react";
import { ThemeProvider, useTheme } from "../theme-provider";

function ThemeDisplay() {
  const { theme, resolvedTheme, setTheme } = useTheme();
  return (
    <div>
      <span data-testid="theme">{theme}</span>
      <span data-testid="resolved">{resolvedTheme}</span>
      <button onClick={() => setTheme("dark")}>dark</button>
      <button onClick={() => setTheme("light")}>light</button>
      <button onClick={() => setTheme("system")}>system</button>
    </div>
  );
}

beforeEach(() => {
  localStorage.clear();
  document.documentElement.classList.remove("light", "dark");
});

describe("ThemeProvider", () => {
  it("defaults to system theme", () => {
    const { getByTestId } = render(
      <ThemeProvider>
        <ThemeDisplay />
      </ThemeProvider>,
    );
    expect(getByTestId("theme").textContent).toBe("system");
  });

  it("applies dark class when set to dark", () => {
    const { getByText } = render(
      <ThemeProvider>
        <ThemeDisplay />
      </ThemeProvider>,
    );
    act(() => getByText("dark").click());
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(document.documentElement.classList.contains("light")).toBe(false);
  });

  it("applies light class when set to light", () => {
    const { getByText } = render(
      <ThemeProvider>
        <ThemeDisplay />
      </ThemeProvider>,
    );
    act(() => getByText("dark").click());
    act(() => getByText("light").click());
    expect(document.documentElement.classList.contains("light")).toBe(true);
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("persists theme to localStorage", () => {
    const { getByText } = render(
      <ThemeProvider>
        <ThemeDisplay />
      </ThemeProvider>,
    );
    act(() => getByText("dark").click());
    expect(localStorage.getItem("logi-theme")).toBe("dark");
  });
});
