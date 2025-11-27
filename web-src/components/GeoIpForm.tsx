import React, { type FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import * as api from "../client";
import { GeoIpInfo } from "./GeoIpInfo.tsx";

interface GeoIpFormData {
	ip: string;
	locale: string;
	edition: string;
}

export interface GeoIpFormProps {
	databases: api.GeoIpDatabaseStatus[];
}

export const GeoIpForm: React.FC<GeoIpFormProps> = ({databases}) => {
	const {register, handleSubmit, setValue, watch} = useForm<GeoIpFormData>();
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const [result, setResult] = useState<api.GeoIpLookupResult | null>(null);
	const edition = watch("edition");
	const locale = watch("locale");
	
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
		setValue("edition", databases[0].edition);
	}, [databases]);
	
	const locales = useMemo(
		() => databases.find(
			database => database.edition === edition,
		)?.locales ?? [],
		[databases, edition],
	);
	
	useEffect(() => {
		if (!locales.length || locale !== "") return;
		setValue("locale", locales.indexOf("en") !== undefined ? "en" : locales[0]);
	}, [locales, locale]);
	
	const handleFormSubmit = useCallback((evt: FormEvent) => {
		handleSubmit(async data => {
			try {
				setError(null);
				setResult(null);
				setLoading(true);
				const res = await api.lookupGeoIp({
					query: {
						ip: data.ip,
						edition: data.edition,
						locale: data.locale,
					},
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
					<select className="select w-full md:w-auto join-item" {...register("locale")}>
						{locales.map(locale => (
							<option key={locale} value={locale}>
								{locale}
							</option>
						))}
					</select>
					<select className="select w-full md:w-auto join-item" {...register("edition")}>
						{databases.map(database => (
							<option key={database.edition} value={database.edition}>
								{database.edition}
							</option>
						))}
					</select>
					<button
						type="submit"
						className="btn join-item"
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
