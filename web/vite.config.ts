import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { VitePWA } from "vite-plugin-pwa";

const nodeMajor = Number.parseInt(process.versions.node.split(".")[0] ?? "0", 10);
const disablePwa =
  process.env.RELAY_DISABLE_PWA === "1" || (nodeMajor >= 25 && process.env.RELAY_FORCE_PWA !== "1");

export default defineConfig({
  build: {
    // Prefer modern JS output to avoid downlevel private-field helpers that can be brittle in some environments.
    target: "es2022",
    // Include sourcemaps for debugging production deployments (personal-use friendly).
    sourcemap: true,
    // Work around occasional minifier issues in some environments; personal-use deploys can afford larger assets.
    minify: false,
  },
  plugins: [
    svelte(),
    VitePWA({
      disable: disablePwa,
      registerType: "autoUpdate",
      manifest: {
        name: "relay",
        short_name: "relay",
        start_url: "/",
        display: "standalone",
        background_color: "#0b1020",
        theme_color: "#0b1020",
        icons: [],
      },
    }),
  ],
});
