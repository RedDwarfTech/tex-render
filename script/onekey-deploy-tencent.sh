#!/usr/bin/env bash

set -u

set -e

set -x

# cargo build --release --target=x86_64-unknown-linux-gnu

rsync -avzh ./cv-render tencent01:~/apps/cv-render
