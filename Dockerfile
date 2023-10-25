FROM pytorch/pytorch:2.0.0-cuda11.7-cudnn8-runtime

RUN apt-get update && \
    apt-get install -y \
    git \
    curl \
    gcc \
    pkg-config \
    libssl-dev \
    libtorch3-dev

RUN git clone https://huggingface.co/sentence-transformers/all-MiniLM-L12-v2
RUN git clone https://github.com/guillaume-be/rust-bert.git

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:$PATH"

COPY . . 

ENV LIBTORCH=/usr/lib/libtorch.so
ENV LD_LIBRARY_PATH=${LIBTORCH}/lib:$LD_LIBRARY_PATH

ENV RUSTBERT_CACHE=./

# RUN cargo run --bin download-model