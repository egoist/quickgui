import { defineConfig } from "@quickgui/cli";

export default defineConfig({
  language: "typescript",
  name: "Quick Git",
  identifier: "dev.quickgui.quick-git.typescript",
  entry: "app.tsx",
  macos: {
    category: "public.app-category.developer-tools",
  },
});
