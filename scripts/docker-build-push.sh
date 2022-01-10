# !/bin/bash

docker build -t charmitro/cherry-node:latest -f Dockerfile ../
docker push charmitro/cherry-node:latest