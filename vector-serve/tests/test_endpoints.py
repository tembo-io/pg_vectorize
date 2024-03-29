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
    response = test_client.get("/v1/info", params={"model_name": "all-MiniLM-L12-v2"})
    assert response.status_code == 200
