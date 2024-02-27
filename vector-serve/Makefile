SOURCE_OBJECTS=app
POETRY_VERSION:=1.7.1

format:
	poetry run black ${SOURCE_OBJECTS}
	poetry run ruff check --silent --fix --exit-zero ${SOURCE_OBJECTS}

download.models:
	poetry run python -m app.models

run: download.models
	poetry run uvicorn app.app:app --host 0.0.0.0 --port 3000

run.docker:
	docker build -t vector-serve .
	docker run -p 3000:3000 vector-serve

test:
	poetry run pytest

setup:
	curl -sSL https://install.python-poetry.org | POETRY_VERSION=${POETRY_VERSION} python3 -
	poetry install

check:
	poetry run ruff check ${SOURCE_OBJECTS}
	poetry run black --check ${SOURCE_OBJECTS}
