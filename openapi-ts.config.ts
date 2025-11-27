import { defineConfig } from "@hey-api/openapi-ts";

export default defineConfig({
	input: "./openapi.yaml",
	output: "web-src/client",
});
