import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { defaultAdapter } from "./adapter";
import "./styles.css";
const adapter = defaultAdapter();
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App adapter={adapter} />
  </StrictMode>,
);
