#!/usr/bin/env bash

set -u

set -e

set -x

ps auxww | grep "cv-render" | grep -v "grep" | grep -v "startup-production" | awk '{print $2}' | xargs -r kill -9
chmod +x /home/ubuntu/apps/cv-render/target/release/cv-render
nohup /home/ubuntu/apps/cv-render/target/release/cv-render >> /home/ubuntu/apps/cv-render/cv-render.log &!
