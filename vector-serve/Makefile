SOURCE_OBJECTS=app


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
