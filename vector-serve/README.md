# vector-serve

## Get started with docker

## Start the server in docker

```bash
make run.docker
```

## or, run directly

```bash
make run
```

## Sentence to embedding transform

The image comes pre-loaded with `all-MiniLM-L6-v2`.

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
  "model": "all-MiniLM-L6-v2"
}
```

Other sentence-transformers will be downloaded on-the-fly on the first request, and cached for future requests.

```bash
curl -X POST http://localhost:3000/v1/embeddings \
  -H 'Content-Type: application/json' \
  -d '{"input": ["solar powered mobile electronics accessories without screens"],
   "model": "sentence-transformers/sentence-t5-base"}'
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
  "model": "sentence-transformers/sentence-t5-base"
}
```
