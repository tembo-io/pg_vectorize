FROM python:3.12.3-slim-bookworm

RUN apt-get update && \
    apt-get install -y curl make && \
    rm -rf /var/lib/apt/lists/*
    
ENV POETRY_NO_INTERACTION=1 \
    POETRY_VIRTUALENVS_IN_PROJECT=true \
    POETRY_VIRTUALENVS_CREATE=1 \
    POETRY_HOME="/opt/poetry" \
    PATH="/opt/poetry/bin:$PATH"

COPY vector-serve /vector-serve/

WORKDIR /vector-serve

RUN curl -sSL https://install.python-poetry.org | POETRY_VERSION=1.7.1 python3 - && \
	poetry install --no-cache --without dev --no-root && \
    find $POETRY_HOME -type d -name "__pycache__" -exec rm -rf {} + && \
    poetry run python -m app.models

CMD ["poetry", "run", "uvicorn", "app.app:app", "--host", "0.0.0.0", "--port", "3000", "--workers", "1"]

