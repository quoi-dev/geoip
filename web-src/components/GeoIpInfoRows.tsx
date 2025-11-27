import React from "react";

export interface GeoIpInfoRowsProps {
	keyPrefix?: string;
	info: any;
}

export const GeoIpInfoRows: React.FC<GeoIpInfoRowsProps> = ({keyPrefix, info}) => {
	if (Array.isArray(info)) {
		return info.map((value, i) => (
			<GeoIpInfoRows
				key={i}
				keyPrefix={`${keyPrefix ?? ""}[${i}]`}
				info={value}
			/>
		));
	}
	if (typeof info === "object") {
		return Object.entries(info).map(([key, value]) => (
			<GeoIpInfoRows
				key={key}
				keyPrefix={`${keyPrefix ?? ""}${keyPrefix !== undefined ? "." : ""}${key}`}
				info={value}
			/>
		));
	}
	return (
		<tr>
			<th>{keyPrefix}</th>
			<td>{info?.toString() ?? "-"}</td>
		</tr>
	);
};
