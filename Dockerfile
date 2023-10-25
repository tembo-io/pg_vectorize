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

RUN git clone -b v2.1.0 --recurse-submodule https://github.com/pytorch/pytorch.git pytorch-static --depth 1
RUN cd pytorch-static && USE_CUDA=OFF BUILD_SHARED_LIBS=OFF python setup.py build

ENV LIBTORCH=/opt/conda/lib/python3.10/site-packages/torch
ENV LD_LIBRARY_PATH=${LIBTORCH}/lib:$LD_LIBRARY_PATH

ENV RUSTBERT_CACHE=./

RUN cargo run --bin download-model