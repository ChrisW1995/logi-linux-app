import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { ThemeProvider } from "@/components/theme-provider";
import { Sidebar } from "../sidebar";

function renderSidebar() {
  return render(
    <ThemeProvider defaultTheme="light">
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>
    </ThemeProvider>,
  );
}

describe("Sidebar", () => {
  it("renders Devices, Flow, and Settings nav items", () => {
    renderSidebar();
    expect(screen.getByText("Devices")).toBeInTheDocument();
    expect(screen.getByText("Flow")).toBeInTheDocument();
    expect(screen.getByText("Settings")).toBeInTheDocument();
  });

  it("shows 'Soon' badge on Flow", () => {
    renderSidebar();
    expect(screen.getByText("Soon")).toBeInTheDocument();
  });
});
