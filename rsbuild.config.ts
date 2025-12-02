import { defineConfig } from '@rsbuild/core';
import { pluginReact } from '@rsbuild/plugin-react';
import CompressionPlugin from "compression-webpack-plugin";

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
		template: "./web-src/index.html",
	},
	server: {
		proxy: {
			"/api": "http://localhost:8080",
			"/files": "http://localhost:8080",
			"/swagger-ui": "http://localhost:8080",
		},
	},
	tools: {
		rspack: config => {
			config.plugins ||= [];
			config.plugins.push(new CompressionPlugin({
				test: /\.(js|css)$/,
				filename: "[path][base].gz",
				algorithm: "gzip",
				compressionOptions: { level: 9 },
				minRatio: 0.99,
				threshold: 1024,
			}));
		},
	},
	performance: {
		printFileSize: {
			exclude: asset => /\.(?:map|LICENSE\.txt|d\.ts|js.gz|css.gz)$/.test(asset.name),
		},
	},
});
