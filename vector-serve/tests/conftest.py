import pytest
from starlette.testclient import TestClient

from app.app import app


@pytest.fixture()
def test_client():
    with TestClient(app) as test_client:
        yield test_client
