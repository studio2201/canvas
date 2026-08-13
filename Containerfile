FROM registry.access.redhat.com/ubi9/ubi:latest AS builder
RUN dnf install -y gcc gcc-c++ make openssl-devel
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup target add wasm32-unknown-unknown
RUN curl -L https://github.com/trunk-rs/trunk/releases/download/v0.20.1/trunk-x86_64-unknown-linux-gnu.tar.gz | tar -xzf - -C /usr/local/bin

WORKDIR /app
COPY shared-assets /app/shared-assets
COPY canvas /app/canvas
WORKDIR /app/canvas

RUN cd frontend && trunk build --release
RUN cargo build --release -p backend

FROM registry.access.redhat.com/ubi9/ubi-minimal:latest
WORKDIR /app
COPY --from=builder /app/canvas/target/release/backend /app/server
COPY --from=builder /app/canvas/frontend/dist /app/dist
ENV BIND_ADDR="0.0.0.0:8080"
EXPOSE 8080
CMD ["/app/server"]
