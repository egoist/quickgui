import { defineConfig } from "@quickgui/cli";

export default defineConfig({
  name: "Quick Git",
  identifier: "dev.quickgui.quick-git-go",
  language: "go",
  entry: ".",
  macos: {
    category: "public.app-category.developer-tools",
  },
});
