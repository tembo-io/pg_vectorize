from fastapi.testclient import TestClient
from fastapi import FastAPI


def test_ready_endpoint(test_client):
    response = test_client.get("/ready")
    assert response.status_code == 200
    assert response.json() == {"ready": True}


def test_alive_endpoint(test_client):
    response = test_client.get("/alive")
    assert response.status_code == 200
    assert response.json() == {"alive": True}


def test_model_info(test_client):
    response = test_client.get(
        "/v1/info", params={"model_name": "sentence-transformers/all-MiniLM-L6-v2"}
    )
    assert response.status_code == 200


def test_metrics_endpoint(test_client):
    response = test_client.get("/metrics")
    assert response.status_code == 200
    assert "all-MiniLM-L6-v2" in response.text


# Simulate a large document
long_text = "This is a very long document. " * 1000


def test_chunking_basic(test_client):
    payload = {"input": [long_text], "model": "all-MiniLM-L6-v2", "normalize": False}
    response = test_client.post("/v1/embeddings", json=payload)

    assert response.status_code == 200
    response_data = response.json()

    assert len(response_data["data"]) > 0
    assert "embedding" in response_data["data"][0]
    assert len(response_data["data"]) > 1


def test_chunking_small_input(test_client):
    small_text = "Short text."
    payload = {"input": [small_text], "model": "all-MiniLM-L6-v2", "normalize": False}
    response = test_client.post("/v1/embeddings", json=payload)

    assert response.status_code == 200
    response_data = response.json()

    assert len(response_data["data"]) == 1
    assert "embedding" in response_data["data"][0]


def test_chunk_overlap(test_client):
    payload = {"input": [long_text], "model": "all-MiniLM-L6-v2", "normalize": False}
    response = test_client.post("/v1/embeddings", json=payload)

    assert response.status_code == 200
    response_data = response.json()

    chunk_size = 1000
    overlap_size = 200
    num_chunks = len(long_text) // (chunk_size - overlap_size)

    assert len(response_data["data"]) == num_chunks


def test_large_input(test_client):
    large_text = "Lorem ipsum " * 5000
    payload = {"input": [large_text], "model": "all-MiniLM-L6-v2", "normalize": False}
    response = test_client.post("/v1/embeddings", json=payload)

    assert response.status_code == 200
    response_data = response.json()

    assert len(response_data["data"]) > 1
    assert "embedding" in response_data["data"][0]
