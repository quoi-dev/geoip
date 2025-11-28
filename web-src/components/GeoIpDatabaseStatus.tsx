import React from "react";
import * as api from "../client";

export interface GeoIpDatabaseStatusProps {
	database: api.GeoIpDatabaseStatus;
}

export const GeoIpDatabaseStatus: React.FC<GeoIpDatabaseStatusProps> = ({database}) => {
	return (
		<div className="card bg-base-200 shadow-sm">
			<div className="card-body">
				<h2 className="card-title">{database.edition}</h2>
				<table className="table table-sm">
					<tbody>
						<tr>
							<th>Last update check</th>
							<td>{database.last_update_check ?? "-"}</td>
						</tr>
						<tr>
							<th>Timestamp</th>
							<td>{database.timestamp ?? "-"}</td>
						</tr>
						<tr>
							<th>Archive file size</th>
							<td>{database.archive_file_size ?? "-"} bytes</td>
						</tr>
						<tr>
							<th>File size</th>
							<td>{database.file_size ?? "-"} bytes</td>
						</tr>
						<tr>
							<th>Locales</th>
							<td>{database.locales?.length ?? "-"}</td>
						</tr>
					</tbody>
				</table>
				{database.error !== undefined && (
					<div className="alert alert-error">
						{database.error}
					</div>
				)}
			</div>
		</div>
	);
};
