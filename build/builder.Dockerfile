# debian:bookworm @ Sat, 16 Aug 2025 12:57:23 +0000
FROM debian@sha256:731dd1380d6a8d170a695dbeb17fe0eade0e1c29f654cf0a3a07f372191c3f4b

RUN groupadd -g 1000 build && \
    useradd -u 1000 -g 1000 -d /build -s /bin/bash -m build && \
    mkdir /tools && \
    chown -R build:build /build /tools

ENV HOME=/build \
    SHELL=/bin/bash \
    USER=build \
    LOGNAME=build \
    HOSTNAME=builder \
    DEBIAN_FRONTEND=noninteractive

RUN rm -f /etc/apt/sources.list.d/* && \
    ( echo 'deb http://snapshot.debian.org/archive/debian/20250816T024221Z/ bookworm main'; \
      echo 'deb-src http://snapshot.debian.org/archive/debian/20250816T024221Z/ bookworm main'; \
      echo 'deb http://snapshot.debian.org/archive/debian-security/20250816T095058Z/ bookworm-security main'; \
      echo 'deb-src http://snapshot.debian.org/archive/debian-security/20250816T095058Z/ bookworm-security main'; \
    ) > /etc/apt/sources.list

RUN \
    ( echo 'quiet "true";'; \
      echo 'APT::Get::Assume-Yes "true";'; \
      echo 'APT::Install-Recommends "false";'; \
      echo 'Acquire::Check-Valid-Until "false";'; \
      echo 'Acquire::Retries "5";'; \
    ) > /etc/apt/apt.conf.d/99portable-builder

# Builder dependencies

RUN apt update && \
    apt install --no-install-recommends -y \
        ca-certificates curl file \
        build-essential \
        autoconf automake autotools-dev libtool xutils-dev

# `auth-rs` system dependencies

RUN apt install \
    libcairo2-dev \
    libdbus-1-dev \
    libglib2.0-dev \
    libgtk-3-dev \
    libjavascriptcoregtk-4.1-dev \
    libsoup-3.0-dev \
    libssl-dev \
    libwebkit2gtk-4.1-dev \
    pkg-config

# Install rustup
USER build
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none
ENV PATH=/build/.cargo/bin:$PATH