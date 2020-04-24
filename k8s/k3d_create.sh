#!/bin/bash
k3d create --api-port 6550 --publish 8081:80 --workers 2
