FROM quay.io/coredb/pgrx-builder:pg15-pgrx0.11.3 as builder

USER root

RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && apt-get clean && \
    rm -rf /var/lib/apt/lists/* /var/cache/apt/archives/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN /root/.cargo/bin/rustup default stable

WORKDIR /build

COPY . .

RUN cargo pgrx init --pg15 /usr/bin/pg_config
RUN cargo build --bin worker --release

FROM pytorch/pytorch:2.1.0-cuda12.1-cudnn8-runtime

COPY --from=builder /build/target/release/worker /usr/local/bin/worker

WORKDIR /usr/src/app

RUN apt-get update && \
    apt-get install -y curl

RUN pip install \
    fastapi==0.104.0 \
    uvicorn[standard]==0.23.2 \
    sentence-transformers==2.2.2

# Download models, save to image cache
COPY vector-serve/app/models.py .
RUN python models.py

COPY . .

CMD ["uvicorn", "app.app:app", "--host", "0.0.0.0", "--port", "3000", "--workers", "2"]
