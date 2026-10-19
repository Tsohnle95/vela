import type { VelaApi } from "../../preload";

declare global {
  interface Window {
    vela: VelaApi;
  }
}

export {};
