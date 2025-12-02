import "./index.css";
import "leaflet/dist/leaflet.css";
import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./components/App.tsx";
import { DialogProvider } from "./components/DialogProvider.tsx";

const rootEl = document.getElementById("root");
if (rootEl) {
	const root = ReactDOM.createRoot(rootEl);
	root.render(
		<React.StrictMode>
			<DialogProvider>
				<App />
			</DialogProvider>
		</React.StrictMode>,
	);
}
