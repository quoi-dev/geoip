#!/bin/sh
docker build --platform linux/amd64 -t ghcr.io/quoi-dev/geoip:latest . && \
docker push ghcr.io/quoi-dev/geoip:latest
