.PHONY: docs

docs:
	poetry install --no-directory --no-root
	poetry run mkdocs serve
