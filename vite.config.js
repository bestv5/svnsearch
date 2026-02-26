import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: false, // 允许 Vite 自动选择可用端口
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: process.env.TAURI_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
    cacheDir: ".vite/build-cache",
    emptyOutDir: true,
  },
  optimizeDeps: {
    // 预构建依赖
    include: ["vue", "pinia", "@tauri-apps/api"],
    // 启用依赖缓存
    cacheDir: ".vite/cache",
  },
});
