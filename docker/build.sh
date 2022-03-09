#!/usr/bin/env bash
set -e

pushd .

# The following line ensure we run from the project root
PROJECT_ROOT=$(git rev-parse --show-toplevel)
cd $PROJECT_ROOT

# Find the current version from Cargo.toml
VERSION=$(grep "^version" ./bin/node/cli/Cargo.toml | egrep -o "([0-9\.]+)")
GITUSER=parity
GITREPO=substrate

# Build the image
echo "Building ${GITUSER}/${GITREPO}:latest docker image, hang on!"
time docker build -f ./docker/Dockerfile -t cherrylabsorg/cherry-node:latest .
docker tag cherrylabsorg/cherry-node:latest

# Show the list of available images for this repo
echo "Image is ready"
docker images | grep ${GITREPO}

popd
