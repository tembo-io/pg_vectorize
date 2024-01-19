# vector-serve

## Get started with docker

## Start the server

```bash
docker run -p 3000:3000 quay.io/tembo/vector-serve:latest
```

## Run directly

```bash
poetry run uvicorn app.app:app --host "0.0.0.0" --port 3000 --workers 2
```

## Sentence to embedding transform

```bash
curl -X POST http://localhost:3000/v1/embeddings \
  -H 'Content-Type: application/json' \
  -d '{"input": ["solar powered mobile electronics accessories without screens"]}'
```

```console
{
  "data": [
    {
      "embedding": [
        -0.07903402298688889,
        0.028912536799907684,
        -0.018827738240361214,
        -0.013423092663288116,
        -0.06503172218799591,
          ....384 total elements
      ],
      "index": 0
    }
  ],
  "model": "all-MiniLM-L12-v2"
}
```

```bash
curl -X POST http://localhost:3000/v1/embeddings \
  -H 'Content-Type: application/json' \
  -d '{"input": ["solar powered mobile electronics accessories without screens"],
   "model": "jinaai/jina-embeddings-v2-base-en"}'
```

```console
{
  "data": [
    {
      "embedding": [
        -0.07903402298688889,
        0.028912536799907684,
        -0.018827738240361214,
        -0.013423092663288116,
        -0.06503172218799591,
          ....384 total elements
      ],
      "index": 0
    }
  ],
  "model": "all-MiniLM-L12-v2"
}
```
