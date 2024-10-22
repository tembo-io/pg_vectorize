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
    response = test_client.get("/v1/info", params={"model_name": "sentence-transformers/all-MiniLM-L6-v2"})
    assert response.status_code == 200


def test_metrics_endpoint(test_client):
    response = test_client.get("/metrics")
    assert response.status_code == 200
    assert "all-MiniLM-L6-v2" in response.text

def test_long_text_endpoint(test_client):
    long_text = "This is a very long document. " * 1000  # Create a long document

    payload = {
        "input": [long_text],
        "model": "all-MiniLM-L6-v2",
        "normalize": False
    }

    response = test_client.post("/v1/embeddings", json=payload)
    assert response.status_code == 200
    response_data = response.json()

    # Verify that chunking occurred correctly
    assert len(response_data["data"]) > 1  # More than one chunk returned

    # Validate that each chunk is of appropriate length
    for chunk in response_data["data"]:
        assert len(chunk['embedding']) > 0  # Check that each chunk has an embedding


def test_small_input(test_client):
    small_text = "Short text."
    payload = {
        "input": [small_text],
        "model": "all-MiniLM-L6-v2",
        "normalize": False
    }

    response = test_client.post("/v1/embeddings", json=payload)
    assert response.status_code == 200
    response_data = response.json()

    assert len(response_data["data"]) == 1  # Should return one chunk for small input
    assert response_data["data"][0]['embedding'] is not None  # Check that the embedding exists


def test_empty_input(test_client):
    payload = {
        "input": [""],
        "model": "all-MiniLM-L6-v2",
        "normalize": False
    }

    response = test_client.post("/v1/embeddings", json=payload)
    assert response.status_code == 200
    response_data = response.json()
    
    # Expect no chunks for empty input
    assert len(response_data["data"]) == 0  # No chunks should be created


def test_boundary_chunking(test_client):
    boundary_text = "A" * 500  # Exactly at the chunk size
    payload = {
        "input": [boundary_text],
        "model": "all-MiniLM-L6-v2",
        "normalize": False
    }

    response = test_client.post("/v1/embeddings", json=payload)
    assert response.status_code == 200
    response_data = response.json()

    assert len(response_data["data"]) == 1  # Should return one chunk
    assert response_data["data"][0]['embedding'] is not None  # Check that the embedding exists