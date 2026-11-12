# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 TPT Solutions
#
# Reproduces the proof run without installing anything on the host (todo 5D.03).
# It builds tpt-telos at the pinned commit with the z3 feature, then runs scripts/verify_proofs.py
# on this repository. The steps match .github/workflows/nightly.yml.
#
# Build and run from the repository root:
#   docker build -t tpt-statera-proofs .
#   docker run --rm tpt-statera-proofs
#
# Ubuntu 24.04 is required: the official Z3 release is built against glibc 2.39.

FROM ubuntu:24.04

ARG Z3_VERSION=5.4.1
# todo 1A.07a: the same pin as nightly.yml. Bump both together, with the proofs re-run.
ARG TELOS_REF=bfa9637ac8682cd3dae94aa9d59387daaf1a3e0c

ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:/opt/z3/bin:$PATH

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates curl git unzip python3 build-essential pkg-config libclang-dev \
 && rm -rf /var/lib/apt/lists/*

# Rust, stable, with the components the repo's rust-toolchain.toml asks for.
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable -c rustfmt -c clippy

# Official Z3 release. The directory name is normalized so the paths below do not depend on the version.
RUN set -e; \
    curl -sSL -o /tmp/z3.zip \
      "https://github.com/Z3Prover/z3/releases/download/z3-${Z3_VERSION}/z3-${Z3_VERSION}-x64-glibc-2.39.zip"; \
    unzip -q /tmp/z3.zip -d /opt; \
    mv /opt/z3-${Z3_VERSION}-x64-glibc-2.39 /opt/z3; \
    rm /tmp/z3.zip

ENV Z3_SYS_Z3_HEADER=/opt/z3/include/z3.h \
    Z3_LIBRARY_PATH_OVERRIDE=/opt/z3/bin \
    LD_LIBRARY_PATH=/opt/z3/bin

# tpt-telos at the pinned commit. The z3 API port is already in this commit, so no patch is applied.
RUN git clone https://github.com/tpt-solutions/tpt-telos.git /opt/tpt-telos \
 && git -C /opt/tpt-telos checkout "${TELOS_REF}" \
 && cd /opt/tpt-telos \
 && cargo build --release -p tpt-telos --features z3

COPY . /opt/tpt-statera
WORKDIR /opt/tpt-statera

# Every proof must exit 0, and every negative control must refute its claim (see verify_proofs.py).
CMD ["python3", "scripts/verify_proofs.py", "--telos", "/opt/tpt-telos/target/release/telos", "--solver", "z3"]
