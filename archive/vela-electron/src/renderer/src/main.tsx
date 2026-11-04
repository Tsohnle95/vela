import { launchApplication } from "./launch";
import { migrateVelaStorage } from "./storage-migration";

try {
  migrateVelaStorage(window.localStorage);
} catch {
  // Storage can be unavailable in restricted renderer contexts.
}

void launchApplication(() => import("./mount-app")).catch(() => {
  const launch = document.querySelector("#launch-screen");
  if (launch) {
    launch.setAttribute("role", "alert");
    launch.textContent = "Vela could not start. Please reopen the app.";
  }
});
