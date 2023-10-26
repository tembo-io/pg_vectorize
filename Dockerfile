FROM pytorch/pytorch:2.1.0-cuda12.1-cudnn8-runtime
# FROM python:3.11.1

WORKDIR /usr/src/app

RUN apt-get update && \
    apt-get install -y curl

RUN pip install \
    fastapi==0.104.0 \
    uvicorn[standard]==0.23.2 \
    sentence-transformers==2.2.2
# RUN curl -sSL https://install.python-poetry.org | POETRY_HOME=/ POETRY_VERSION=1.6.1 python3 -
# RUN poetry config virtualenvs.create false

# COPY pyproject.toml poetry.lock ./

# RUN poetry install --no-root

# Download models
COPY app/init_models.py .
# RUN poetry run python init_models.py
RUN python init_models.py

COPY . .

CMD ["uvicorn", "app.app:app", "--host", "0.0.0.0", "--port", "80"]