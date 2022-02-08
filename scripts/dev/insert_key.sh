#!/usr/bin/env bash

curl -vH 'Content-Type: application/json' \
    --data \
    '{ "jsonrpc":"2.0", "method":"author_insertKey", "params":["chfs", "0xe5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a", "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"],"id":1 }' \
    localhost:9933
