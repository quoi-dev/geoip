import React, { useEffect, useState } from "react";
import * as api from "../client";
import { GeoIpForm } from "./GeoIpForm.tsx";

const App: React.FC = () => {
	const [error, setError] = useState<string | null>(null);
	const [status, setStatus] = useState<api.GeoIpStatus | null>(null);
	
	useEffect(() => {
		(async () => {
			try {
				setError(null);
				const res = await api.getStatus();
				if (res.error) {
					setError(res.error.error ?? `Error ${res.response.status}`);
					return;
				}
				setStatus(res.data);
			} catch (err: any) {
				setError(err.message ?? err.toString());
			}
		})();
	}, []);
	
	return (
		<div className="container mx-auto p-4">
			<h1 className="text-center text-2xl mb-4">GeoIP</h1>
			{error && (<div className="alert alert-error alert-soft mb-4">
				{error}
			</div>)}
			<GeoIpForm editions={status?.databases.map(db => db.edition) ?? []} />
			<footer className="footer sm:footer-horizontal footer-center">
				<aside>
					<div>
						This product uses <strong>GeoLite2 Data</strong> created by <strong>MaxMind</strong>, available from{" "}
						<a className="link" href="https://www.maxmind.com/" target="_blank">www.maxmind.com</a>
					</div>
					<div>
						<a className="link" href="https://github.com/quoi-dev/geoip" target="_blank">
							Project source code on GitHub
						</a>
					</div>
					<div>
						<a className="link" href="/swagger-ui" target="_blank">
							Swagger UI
						</a>
					</div>
				</aside>
			</footer>
		</div>
	);
};

export default App;
