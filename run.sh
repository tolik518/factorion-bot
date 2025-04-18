#!/bin/bash
while true
do
    factorion-bot
    echo "ERROR | run.sh | $(date -u +"%Y-%m-%dT%H:%M:%SZ") | factorion-bot has crashed"
    sleep 60
done
