import React, { type FormEvent, useCallback, useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import * as api from "../client";
import { GeoIpInfo } from "./GeoIpInfo.tsx";

interface GeoIpFormData {
	ip: string;
	edition: string;
}

export interface GeoIpFormProps {
	editions: string[];
}

export const GeoIpForm: React.FC<GeoIpFormProps> = ({editions}) => {
	const {register, handleSubmit, setValue} = useForm<GeoIpFormData>();
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);
	const [result, setResult] = useState<api.GeoIpLookupResult | null>(null);
	
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
		if (!editions.length) return;
		setValue("edition", editions[0]);
	}, [editions]);
	
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
					<select className="select w-full md:w-auto join-item" {...register("edition")}>
						{editions.map(edition => (
							<option key={edition} value={edition}>
								{edition}
							</option>
						))}
					</select>
					<button type="submit" className="btn join-item" disabled={loading || !editions.length}>
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
						{result.elapsed * 1000}ms elapsed
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
