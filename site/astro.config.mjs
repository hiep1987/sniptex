// @ts-check
import { defineConfig } from "astro/config";
import tailwind from "@astrojs/tailwind";
import mdx from "@astrojs/mdx";

// GitHub Pages on a project repo serves at https://<user>.github.io/<repo>/
// so we need both `site` (absolute URL) and `base` (repo-relative path).
export default defineConfig({
  site: "https://hiep1987.github.io",
  base: "/sniptex/",
  trailingSlash: "ignore",
  integrations: [tailwind({ applyBaseStyles: false }), mdx()],
  build: {
    inlineStylesheets: "auto",
  },
});
