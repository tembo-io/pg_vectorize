SOURCE_OBJECTS=app


format:
	poetry run black ${SOURCE_OBJECTS}
	poetry run ruff check --silent --fix --exit-zero ${SOURCE_OBJECTS}