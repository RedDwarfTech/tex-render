ARG BASE_IMAGE=dolphinjiang/rust-musl-builder:latest
FROM ${BASE_IMAGE} AS builder
ADD --chown=rust:rust . ./
RUN CARGO_HTTP_MULTIPLEXING=false cargo build --release

# https://github.com/kjarosh/latex-docker
FROM kjarosh/latex:2023.1
LABEL org.reddwarf.image.authors="jiangtingqiang@gmail.com"

ENV LANG=en_US.UTF-8 \
    LC_ALL=en_US.UTF-8 \
    TZ=Asia/Shanghai

WORKDIR /app
ENV ROCKET_ADDRESS=0.0.0.0
COPY --from=builder /home/rust/src/settings-production.toml /app/settings.toml
COPY --from=builder /home/rust/src/script /app/
COPY --from=builder /home/rust/src/target/x86_64-unknown-linux-musl/release/cv-render /app/
RUN mkdir -p /usr/share/fonts/ && mkdir -p /app/config/ && mkdir -p /root/.ssh
COPY --from=builder /home/rust/src/texmf/tex/latex/ /opt/texlive/texmf-local/tex/latex/
COPY --from=builder /home/rust/src/log4rs.yaml /app/
RUN tlmgr update --self && tlmgr install ctex moderncv fontawesome5 fontawesome nth\ 
    academicons multirow arydshln titlesec enumitem makecell relsize\
    tcolorbox environ tikzfill csquotes xifthen ifmtarg tex-gyre && \
    apk update && \
    apk add rsync openssh sshpass fontconfig && \
    chmod +x cv-render && texhash && fc-cache -f
CMD ["./cv-render"]