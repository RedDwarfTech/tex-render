#!/bin/sh

nohup sh -c "while true; do sleep 3600; done" &
nohup ./cv-render >> render.log 2>&1 &

tail -f render.log