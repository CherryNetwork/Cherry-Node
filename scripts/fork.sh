#!/bin/bash
set -o pipefail

RED="31"
GREEN="32"

BOLD="\e[1;m"

BOLDGREEN="\e[1;${GREEN}m"
BOLDRED="\e[1;${RED}m"
ITALICRED="\e[3;${RED}m"

ENDCOLOR="\e[0m"

echo -e "${BOLDGREEN}Starting Cherry Node hard-fork.${ENDCOLOR}"

if [ -z "$NODE_1_ADDR" ]
then
  echo -e "${BOLDRED}Error:${ENDCOLOR} ${BOLD}Environment variable NODE_1_ADDR is not set.${ENDCOLOR}"
fi
if [ -z "$NODE_2_ADDR"]
then
  echo -e "${BOLDRED}Error:${ENDCOLOR} ${BOLD}Environment variable NODE_2_ADDR is not set.${ENDCOLOR}"

fi

if [ -z "$GRAN1" ]
then
  echo -e "${BOLDRED}Error:${ENDCOLOR} ${BOLD}Environment variable GRAN1 is not set.${ENDCOLOR}"
fi
if [ -z "$GRAN2"]
then
  echo -e "${BOLDRED}Error:${ENDCOLOR} ${BOLD}Environment variable GRAND2 is not set.${ENDCOLOR}"
fi


if [ -z "$BABE1" ]
then
  echo -e "${BOLDRED}Error:${ENDCOLOR} ${BOLD}Environment variable BABE1 is not set.${ENDCOLOR}"
fi
if [ -z "$BABE2"]
then
  echo -e "${BOLDRED}Error:${ENDCOLOR} ${BOLD}Environment variable BABE2 is not set.${ENDCOLOR}"
fi

if [ -z "$IMOL1" ]
then
  echo -e "${BOLDRED}Error:${ENDCOLOR} ${BOLD}Environment variable IMOL1 is not set.${ENDCOLOR}"
fi
if [ -z "$IMOL2"]
then
  echo -e "${BOLDRED}Error:${ENDCOLOR} ${BOLD}Environment variable IMOL2 is not set.${ENDCOLOR}"
fi

if [ -z "$AUTH1" ]
then
  echo -e "${BOLDRED}Error:${ENDCOLOR} ${BOLD}Environment variable AUTH1 is not set.${ENDCOLOR}"
fi
if [ -z "$AUTH2"]
then
  echo -e "${BOLDRED}Error:${ENDCOLOR} ${BOLD}Environment variable AUTH2 is not set.${ENDCOLOR}"
fi

echo -e "${BOLDGREEN}Creating.${ENDCOLOR}"

../target/release/cherry \
  build-spec \
  --chain ../customSpec.json \
  --raw \
  --disable-default-bootnode \
  > ../customSpecRaw.json


# TODO: find out how to parse spinup message and get the it's address(NODE_2_ADDR) - @charmitro
../target/release/cherry \
  --base-path /tmp/node03 \
  --chain ../customSpecRaw.json \
  --port 30335 \
  --ws-port 9947 \
  --rpc-port 9935 \
  --telemetry-url "wss://telemetry.polkadot.io/submit/ 0" \
  --validator \
  --rpc-methods Unsafe \
  --name MyNode03 \
  --bootnodes $NODE_1_ADDR

../target/release/cherry \
  --base-path /tmp/node04 \
  --chain ./customSpecRaw.json \
  --port 30334 \
  --ws-port 9946 \
  --rpc-port 9934 \
  --telemetry-url "wss://telemetry.polkadot.io/submit/ 0" \
  --validator \
  --rpc-methods Unsafe \
  --name MyNode02 \
  --bootnodes $NODE_2_ADDR

../target/release/cherry \
  --base-path /tmp/node05 \
  --chain ./customSpecRaw.json \
  --port 30334 \
  --ws-port 9946 \
  --rpc-port 9934 \
  --telemetry-url "wss://telemetry.polkadot.io/submit/ 0" \
  --validator \
  --rpc-methods Unsafe \
  --name MyNode02 \
  --bootnodes $NODE_2_ADDR


# ----- NODE4 -----
../target/release/cherry key \
                         insert \
                         --base-path /tmp/node04 \
                         --chain customSpecRaw.json \
                         --scheme Ed25519 \
                         --suri $GRAN1 \
                         --key-type gran

../target/release/cherry key \
                         insert \
                         --base-path /tmp/node04 \
                         --chain customSpecRaw.json \
                         --scheme Sr25519 \
                         --suri $BABE1 \
                         --key-type babe

../target/release/cherry key \
                         insert \
                         --base-path /tmp/node04 \
                         --chain customSpecRaw.json \
                         --scheme Sr25519 \
                         --suri $IMOL1 \
                         --key-type imol


../target/release/cherry key \
                         insert \
                         --base-path /tmp/node04 \
                         --chain customSpecRaw.json \
                         --scheme Sr25519 \
                         --suri $AUTH1 \
                         --key-type auth

# ------ NODE5 -----
../target/release/cherry key \
                         insert \
                         --base-path /tmp/node04 \
                         --chain customSpecRaw.json \
                         --scheme Ed25519 \
                         --suri $GRAN2 \
                         --key-type gran

../target/release/cherry key \
                         insert \
                         --base-path /tmp/node04 \
                         --chain customSpecRaw.json \
                         --scheme Sr25519 \
                         --suri $BABE2 \
                         --key-type babe

../target/release/cherry key \
                         insert \
                         --base-path /tmp/node04 \
                         --chain customSpecRaw.json \
                         --scheme Sr25519 \
                         --suri $IMOL2 \
                         --key-type imol


../target/release/cherry key \
                         insert \
                         --base-path /tmp/node04 \
                         --chain customSpecRaw.json \
                         --scheme Sr25519 \
                         --suri $AUTH2 \
                         --key-type auth
