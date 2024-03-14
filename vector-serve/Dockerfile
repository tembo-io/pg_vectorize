FROM rust:1.76.0-slim-buster as builder

RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && apt-get clean && \
    rm -rf /var/lib/apt/lists/* /var/cache/apt/archives/*

WORKDIR /build

COPY core/ core/

RUN cd core && \
    cargo build --bin vectorize-worker --release

FROM pytorch/pytorch:2.1.0-cuda12.1-cudnn8-runtime

COPY --from=builder /build/core/target/release/vectorize-worker /usr/local/bin/vectorize-worker

WORKDIR /usr/src/app

RUN apt-get update && \
    apt-get install -y curl

RUN pip install \
    fastapi==0.104.0 \
    uvicorn[standard]==0.23.2 \
    sentence-transformers==2.2.2

# Download models, save to image cache
COPY vector-serve /vector-serve

RUN python /vector-serve/app/models.py

WORKDIR /vector-serve

CMD ["uvicorn", "app.app:app", "--host", "0.0.0.0", "--port", "3000", "--workers", "2"]
