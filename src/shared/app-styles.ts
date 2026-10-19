export const APP_STYLES_START = "/* vela-app-styles:start */";
export const APP_STYLES_END = "/* vela-app-styles:end */";

export interface AppStylesheetUpdate {
  originalURL: string;
  url: string;
  revision: number;
}
