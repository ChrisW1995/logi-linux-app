import { BrowserRouter, Routes, Route } from "react-router-dom";
import { ThemeProvider } from "@/components/theme-provider";
import { Sidebar } from "@/components/layout/sidebar";
import { DevicesPage } from "@/pages/devices";
import { FlowPage } from "@/pages/flow";
import { SettingsPage } from "@/pages/settings";

export default function App() {
  return (
    <ThemeProvider defaultTheme="system">
      <BrowserRouter>
        <div className="flex h-screen bg-background text-foreground">
          <Sidebar />
          <main className="flex-1 overflow-auto p-8">
            <Routes>
              <Route path="/" element={<DevicesPage />} />
              <Route path="/flow" element={<FlowPage />} />
              <Route path="/settings" element={<SettingsPage />} />
            </Routes>
          </main>
        </div>
      </BrowserRouter>
    </ThemeProvider>
  );
}
