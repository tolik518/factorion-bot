#!/bin/bash
while true
do
    ./factorion-bot
    curl -d "factorion-bot has crashed" ntfy:8888/factorion
    sleep 10
done