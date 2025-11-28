import React from "react";
import * as api from "../client";
import { GeoIpDatabaseStatus } from "./GeoIpDatabaseStatus.tsx";

export interface GeoIpStatusProps {
	status: api.GeoIpStatus;
}

export const GeoIpStatus: React.FC<GeoIpStatusProps> = ({status}) => {
	return (
		<div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 items-stretch mb-4">
			{
				(status.databases ?? []).map(database => (
					<GeoIpDatabaseStatus key={database.edition} database={database} />
				))
			}
		</div>
	);
};
