import React, { type FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import * as api from "../client";
import { GeoIpInfo } from "./GeoIpInfo.tsx";

const EDITION_STORAGE_KEY = "geoip.edition";
const LOCALE_STORAGE_KEY = "geoip.locale";

interface GeoIpFormData {
	ip: string;
}

export interface GeoIpFormProps {
	databases: api.GeoIpDatabaseStatus[];
	recaptchaFn?: () => Promise<string | undefined>;
}

export const GeoIpForm: React.FC<GeoIpFormProps> = ({databases, recaptchaFn}) => {
	const {register, handleSubmit, setValue} = useForm<GeoIpFormData>();
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const [result, setResult] = useState<api.GeoIpLookupResult | null>(null);
	const [edition, setEdition] = useState("");
	const [locale, setLocale] = useState("");
	
	useEffect(() => {
		(async () => {
			try {
				setError(null);
				setLoading(true);
				const res = await api.detectIp();
				if (res.error) {
					setError(res.error.error ?? `Error ${res.response.status}`);
					return;
				}
				setValue("ip", res.data.ip);
			} catch (err: any) {
				setError(err.message ?? err.toString());
			} finally {
				setLoading(false);
			}
		})();
	}, []);
	
	useEffect(() => {
		if (!databases.length) return;
		const storedEdition = localStorage.getItem(EDITION_STORAGE_KEY);
		const storedLocale = localStorage.getItem(LOCALE_STORAGE_KEY);
		if (storedEdition !== null) {
			const database = databases.find(
				database => database.edition === storedEdition,
			);
			if (database) {
				setEdition(database.edition);
				if (storedLocale !== null && (database.locales?.indexOf(storedLocale) ?? -1) >= 0) {
					setLocale(storedLocale);
				}
				return;
			}
		}
		setEdition(databases[0].edition);
	}, [databases]);
	
	const locales = useMemo(
		() => databases.find(
			database => database.edition === edition,
		)?.locales ?? [],
		[databases, edition],
	);
	
	useEffect(() => {
		if (!locales.length || !!locale) return;
		setLocale(locales.indexOf("en") >= 0 ? "en" : locales[0]);
	}, [locales, locale]);
	
	useEffect(() => {
		if (!databases.length || !edition) return;
		localStorage.setItem(EDITION_STORAGE_KEY, edition);
	}, [databases, edition]);
	
	useEffect(() => {
		if (!databases.length || !locales.length || !locale) return;
		localStorage.setItem(LOCALE_STORAGE_KEY, locale);
	}, [databases, locales, locale]);
	
	const handleFormSubmit = useCallback((evt: FormEvent) => {
		handleSubmit(async data => {
			try {
				setError(null);
				setResult(null);
				setLoading(true);
				const recaptchaToken = recaptchaFn ? await recaptchaFn() : undefined;
				const headers: Record<string, string> = {};
				if (recaptchaToken !== undefined) {
					headers["X-Recaptcha-Token"] = recaptchaToken;
				}
				const res = await api.lookupGeoIp({
					query: {
						ip: data.ip,
						edition,
						locale,
					},
					headers,
				});
				if (res.error) {
					setError(res.error.error ?? `Error ${res.response.status}`);
					return;
				}
				setResult(res.data);
			} catch (err: any) {
				setError(err.message ?? err.toString());
			} finally {
				setLoading(false);
			}
		})(evt);
	}, [edition, locale, recaptchaFn]);

	const handleEditionChange = useCallback((evt: React.ChangeEvent<HTMLSelectElement>) => {
		setEdition(evt.target.value);
	}, []);
	
	const handleLocaleChange = useCallback((evt: React.ChangeEvent<HTMLSelectElement>) => {
		setLocale(evt.target.value);
	}, []);
	
	return (
		<>
			<form onSubmit={handleFormSubmit} className="mb-4">
				<div className="flex content-stretch gap-4 flex-col md:flex-row md:gap-0 md:join w-full">
					<input
						type="text"
						placeholder="Enter IP address"
						className="input join-item w-full md:flex-1"
						{...register("ip", {required: true})}
						required
					/>
					<select
						className="select w-full md:w-auto join-item"
						value={locale}
						onChange={handleLocaleChange}
					>
						{locales.map(locale => (
							<option key={locale} value={locale}>
								{locale}
							</option>
						))}
					</select>
					<select
						className="select w-full md:w-auto join-item"
						value={edition}
						onChange={handleEditionChange}
					>
						{databases.map(database => (
							<option key={database.edition} value={database.edition}>
								{database.edition}
							</option>
						))}
					</select>
					<button
						type="submit"
						className="btn btn-neutral join-item"
						disabled={loading || !databases.length || !locales.length}
					>
						Lookup
					</button>
				</div>
			</form>
			{error && (<div className="alert alert-error alert-soft mb-4">
				{error}
			</div>)}
			{result && (
				<>
					<div className="alert alert-success alert-soft mb-4">
						{(result.elapsed * 1000).toFixed(3)}ms elapsed
					</div>
					<div className="card bg-base-200 w-full shadow-sm mb-4">
						<div className="card-body">
							{result.info ? (<GeoIpInfo info={result.info} />) : "No result"}
						</div>
					</div>
				</>
			)}
		</>
	);
};
