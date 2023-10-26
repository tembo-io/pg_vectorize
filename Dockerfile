FROM python:3.11.1

WORKDIR /usr/src/app

RUN apt-get update && \
    apt-get install -y curl

RUN curl -sSL https://install.python-poetry.org | POETRY_HOME=/ POETRY_VERSION=1.6.1 python3 -
RUN poetry config virtualenvs.create false

COPY pyproject.toml poetry.lock ./

RUN poetry install --no-root

# Download models
COPY app/init_models.py .
RUN poetry run python init_models.py

COPY . .

CMD ["poetry", "run", "uvicorn", "app.app:app", "--host", "0.0.0.0", "--port", "80"]