import { launchApplication } from "./launch";

void launchApplication(() => import("./mount-app")).catch(() => {
  const launch = document.querySelector("#launch-screen");
  if (launch) {
    launch.setAttribute("role", "alert");
    launch.textContent = "Mira could not start. Please reopen the app.";
  }
});
