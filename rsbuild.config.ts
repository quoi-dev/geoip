import { defineConfig } from '@rsbuild/core';
import { pluginReact } from '@rsbuild/plugin-react';

// Docs: https://rsbuild.rs/config/
export default defineConfig({
	plugins: [pluginReact()],
	source: {
		entry: {
			index: "./web-src/index.tsx",
		},
	},
	html: {
		title: "GeoIP",
		favicon: "./public/favicon.ico",
	},
	server: {
		proxy: {
			"/api": "http://localhost:8080",
			"/swagger-ui": "http://localhost:8080",
		},
	},
});
