#!/bin/bash

echo "1. start build --release..."
cargo build --release
echo "2. rebuild docker image..."
sudo docker build -t asmh1989/rust-eutils .
echo "3. restart docker-compose..."
sudo docker-compose up -d 
echo "done..."