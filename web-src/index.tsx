import "./index.css";
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./components/App.tsx";

const rootEl = document.getElementById("root");
if (rootEl) {
	const root = ReactDOM.createRoot(rootEl);
	root.render(
		<React.StrictMode>
			<App />
		</React.StrictMode>,
	);
}
