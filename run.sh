#!/bin/bash
while true
do
    factorion-bot
    curl -d "factorion-bot has crashed" ntfy:80/factorion
    sleep 30
done