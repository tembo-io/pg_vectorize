#!/bin/bash
# start-server.sh

ollama serve &
serve_pid=$!
sleep 5

ollama pull $1

kill $serve_pid || true
