#!/bin/bash

random_port=$(( ( RANDOM % 1000 )  + 1024 ))
export OLLAMA_HOST=0.0.0.0:$random_port

ollama serve &
serve_pid=$!
sleep 5

ollama pull $1

kill $serve_pid