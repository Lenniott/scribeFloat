// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import path from "node:path";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      fallback: "index.html",
    }),
    alias: {
      "@lib": path.resolve("./src/lib"),
      "@ui": path.resolve("./src/lib/ui"),
      "@primitives": path.resolve("./src/lib/ui/1_primitives"),
      "@components": path.resolve("./src/lib/ui/2_components"),
      "@patterns": path.resolve("./src/lib/ui/3_patterns"),
      "@sections": path.resolve("./src/lib/ui/4_sections"),
      "@regions": path.resolve("./src/lib/ui/5_regions"),
      "@views": path.resolve("./src/lib/ui/views"),
      "@utils": path.resolve("./src/lib/utils"),
      "@services": path.resolve("./src/lib/services"),
      "@stores": path.resolve("./src/lib/stores"),
    },
  },
};

export default config;