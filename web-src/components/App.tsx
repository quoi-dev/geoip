import React, { useCallback, useEffect, useState } from "react";
import * as api from "../client";
import { GeoIpForm } from "./GeoIpForm.tsx";
import { GeoIpStatus } from "./GeoIpStatus.tsx";
import { useShowDialog } from "./DialogProvider.tsx";
import { ApiKeyDialog } from "./ApiKeyDialog.tsx";
import { client } from "../client/client.gen.ts";
import { ConfirmDialog } from "./ConfirmDialog.tsx";

export interface PageCtx {
	recaptcha_site_key?: string;
}

const API_KEY_STORAGE_KEY = "geoip.api-key";

export const App: React.FC = () => {
	const showDialog = useShowDialog();
	const [error, setError] = useState<string | null>(null);
	const [status, setStatus] = useState<api.GeoIpStatus | null>(null);
	const [authenticated, setAuthenticated] = useState(false);
	const [pageCtx, setPageCtx] = useState<PageCtx | null>(null);
	
	useEffect(() => {
		const apiKey = localStorage.getItem(API_KEY_STORAGE_KEY);
		if (apiKey === null) return;
		client.setConfig({auth: apiKey});
		setAuthenticated(true);
	}, []);
	
	useEffect(() => {
		(async () => {
			let ctx: PageCtx | null = null;
			const meta = document.querySelector("meta[name=ctx]") as (HTMLMetaElement | null);
			if (meta) {
				try {
					ctx = JSON.parse(meta.content);
				} catch {
				}
			}
			if (!ctx) {
				try {
					const res = await fetch("/api/ctx");
					if (res.ok) {
						ctx = await res.json();
					}
				} catch (err: any) {
					console.error(err);
				}
			}
			setPageCtx(ctx);
		})();
	}, []);
	
	useEffect(() => {
		if (!pageCtx) return;
		if (!authenticated && pageCtx.recaptcha_site_key !== undefined) {
			if (!document.querySelector("script[id=recaptcha]")) {
				const script = document.createElement("script");
				script.id = "recaptcha";
				script.src = `https://www.google.com/recaptcha/api.js?render=${pageCtx.recaptcha_site_key}`;
				document.body.appendChild(script);
			}
		}
	}, [pageCtx, authenticated]);
	
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
	
	const handleApiKeyClick = useCallback(async (evt: React.MouseEvent) => {
		evt.preventDefault();
		const apiKey = await showDialog(ApiKeyDialog);
		if (apiKey === undefined) return;
		localStorage.setItem(API_KEY_STORAGE_KEY, apiKey);
		client.setConfig({auth: apiKey});
		setAuthenticated(true);
	}, []);
	
	const handleForgetApiKeyClick = useCallback(async (evt: React.MouseEvent) => {
		evt.preventDefault();
		const res = await showDialog(ConfirmDialog, {
			title: "Forget API key",
			messageHtml: "You're going to logout",
			primaryButton: "Logout"
		});
		if (!res) return;
		localStorage.removeItem(API_KEY_STORAGE_KEY);
		client.setConfig({auth: undefined});
		setAuthenticated(false);
	}, []);
	
	const recaptchaFn = useCallback(async () => {
		if (authenticated || pageCtx?.recaptcha_site_key === undefined) return undefined;
		return await grecaptcha.execute(pageCtx.recaptcha_site_key, {action: "submit"});
	}, [pageCtx, authenticated]);
	
	return (
		<div className="container mx-auto p-4">
			<h1 className="text-center text-2xl mb-4">GeoIP</h1>
			{error && (<div className="alert alert-error alert-soft mb-4">
				{error}
			</div>)}
			<GeoIpForm databases={status?.databases ?? []} recaptchaFn={recaptchaFn} />
			{status && <GeoIpStatus status={status} />}
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
					<div className="flex gap-2">
						<a className="link" href="/swagger-ui" target="_blank">
							Swagger UI
						</a>
						<a className="link" href="#" onClick={handleApiKeyClick}>
							API key
						</a>
						{authenticated && (
							<a className="link" href="#" onClick={handleForgetApiKeyClick}>
								Forget API key
							</a>
						)}
					</div>
				</aside>
			</footer>
		</div>
	);
};
