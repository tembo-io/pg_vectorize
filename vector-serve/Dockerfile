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

RUN apt-get update && \
    apt-get install -y curl make
    
ENV POETRY_NO_INTERACTION=1 \
    POETRY_VIRTUALENVS_IN_PROJECT=true \
    POETRY_VIRTUALENVS_CREATE=1 \
    POETRY_HOME="/opt/poetry" \
    PATH="/opt/poetry/bin:$PATH" \
    TRANSFORMERS_CACHE=/models/.cache

COPY vector-serve /vector-serve/

WORKDIR /vector-serve
    
RUN make setup-build && make download.models

CMD ["poetry", "run", "uvicorn", "app.app:app", "--host", "0.0.0.0", "--port", "3000", "--workers", "2"]
