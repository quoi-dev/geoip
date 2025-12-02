import React, { useEffect, useMemo } from "react";
import {
	Circle,
	MapContainer,
	TileLayer, useMap,
} from "react-leaflet";
import L, { type LatLngExpression } from "leaflet";
import classNames from "classnames";
import * as api from "../client";

export interface GeoIpMapProps {
	info?: api.GeoIpInfo;
	osmTilesUrl: string;
	className?: string;
}

export const GeoIpMap: React.FC<GeoIpMapProps> = ({className, info, osmTilesUrl}) => {
	const center = useMemo<LatLngExpression | undefined>(() => (
		info?.latitude !== undefined && info?.longitude !== undefined ? [
			info.latitude,
			info.longitude,
		] : undefined
	), [info?.latitude, info?.longitude]);
	
	const radius = info?.accuracy_radius !== undefined ? info.accuracy_radius * 1000 : undefined;
	
	return (
		<div className={classNames(className, {"hidden": !center || radius === undefined})}>
			<div className={classNames("card", "bg-base-200", "shadow-sm")}>
				<div className="card-body">
					{
						(center && radius !== undefined) && (
							<MapContainer
								center={center}
								zoom={4}
								style={{width: "100%", height: "400px"}}
							>
								<TileLayer
									url={osmTilesUrl}
									attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
								/>
								<Circle center={center} radius={radius} />
								<FitCircle center={center} radius={radius} />
							</MapContainer>
						)
					}
				</div>
			</div>
		</div>
	);
};

interface FitCircleProps {
	center: LatLngExpression;
	radius: number;
}

const FitCircle: React.FC<FitCircleProps> = ({center, radius}) => {
	const map = useMap();
	
	useEffect(() => {
		if (!map) return;
		const latLng = L.latLng(center);
		const bounds = latLng.toBounds(radius);
		map.fitBounds(bounds, {padding: [32, 32]});
	}, [map, center, radius]);
	
	return null;
};
