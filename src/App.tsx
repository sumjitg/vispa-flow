import { useEffect } from "react";
import { Routes, Route } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import { sendNotification, isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
import Settings from "./pages/Settings";
import TrayPopup from "./components/TrayPopup";

export default function App() {
  useEffect(() => {
    const setup = async () => {
      const unlisten = await listen<string>("vispa-error", async (event) => {
        let granted = await isPermissionGranted();
        if (!granted) {
          const perm = await requestPermission();
          granted = perm === "granted";
        }
        if (granted) {
          sendNotification({ title: "Vispa Flow", body: event.payload });
        }
      });

      return () => {
        unlisten();
      };
    };

    const cleanup = setup();
    return () => {
      cleanup.then((fn) => fn());
    };
  }, []);

  return (
    <Routes>
      <Route path="/settings" element={<Settings />} />
      <Route path="/popup"    element={<TrayPopup />} />
      <Route path="*" element={<div className="hidden" />} />
    </Routes>
  );
}
