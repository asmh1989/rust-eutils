#!/bin/bash
rm -rf web/dist
echo "0. start build front..."
cd ../vue-eutils && git pull origin master && npm run build
cd -
echo "1. start build --release..."
cargo build --release
echo "2. rebuild docker image..."
sudo docker build -t asmh1989/rust-eutils:2.0 .
echo "3. restart docker-compose..."
sudo docker-compose up -d
echo "done..."
