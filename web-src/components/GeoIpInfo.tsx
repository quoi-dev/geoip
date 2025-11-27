import React from "react";
import * as api from "../client";
import { GeoIpInfoRows } from "./GeoIpInfoRows.tsx";

export interface GeoIpInfoProps {
	info: api.GeoIpInfo;
}

export const GeoIpInfo: React.FC<GeoIpInfoProps> = ({info}) => {
	return (
		<div className="overflow-x-auto">
			<table className="table">
				<tbody>
					<GeoIpInfoRows info={info} />
				</tbody>
			</table>
		</div>
	);
};
