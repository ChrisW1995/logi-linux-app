import { render, screen } from "@testing-library/react";
import App from "../App";

describe("App", () => {
  it("renders without crashing and shows Devices page by default", () => {
    render(<App />);
    expect(
      screen.getByText("Manage your Logitech devices"),
    ).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Devices" })).toBeInTheDocument();
  });
});
