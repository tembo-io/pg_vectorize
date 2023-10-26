FROM python:3.11.6

WORKDIR /usr/src/app

RUN apt-get update && \
    apt-get install -y curl

RUN curl -sSL https://install.python-poetry.org | POETRY_HOME=/ POETRY_VERSION=1.6.1 python3 -
RUN poetry config virtualenvs.create false

COPY . .

RUN poetry install

# Download models
RUN poetry run python app/init_models.py

CMD ["uvicorn", "app.app:app", "--host", "0.0.0.0", "--port", "8000"]
