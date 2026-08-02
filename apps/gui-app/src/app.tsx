import { BrowserRouter, Route, Routes } from "react-router-dom";
import { AppLayout } from "./layouts/app-layout";
import { MixerPage } from "./pages/mixer-page";
import { SettingsPage } from "./pages/settings-page";

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route element={<AppLayout />}>
          <Route index element={<MixerPage />} />
          <Route path="settings" element={<SettingsPage />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}

export default App;
