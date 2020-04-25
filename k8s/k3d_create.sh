#!/bin/bash
k3d create --api-port 6550 --publish 80:80 --publish 443:443 --workers 2
