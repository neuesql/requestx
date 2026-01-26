# Authentication Guide

RequestX supports various authentication methods for securing your HTTP requests.

## Basic Authentication

HTTP Basic Authentication sends credentials as a base64-encoded header:

```python
import requestx

# Using Auth.basic()
auth = requestx.Auth.basic("username", "password")

response = requestx.get(
    "https://httpbin.org/basic-auth/username/password",
    auth=auth
)
print(response.status_code)  # 200
```

### With Client

```python
import requestx

with requestx.Client(auth=requestx.Auth.basic("user", "pass")) as client:
    # All requests will include Basic auth
    response = client.get("https://api.example.com/protected")
```

## Bearer Token Authentication

Bearer tokens are commonly used for API authentication (OAuth 2.0, JWT):

```python
import requestx

# Using Auth.bearer()
auth = requestx.Auth.bearer("your-api-token-here")

response = requestx.get(
    "https://httpbin.org/bearer",
    auth=auth
)
print(response.status_code)  # 200
```

### With Client

```python
import requestx

# Set bearer token for all requests
with requestx.Client(auth=requestx.Auth.bearer("api-token")) as client:
    users = client.get("https://api.example.com/users").json()
    profile = client.get("https://api.example.com/profile").json()
```

## Custom Header Authentication

For APIs that use custom authentication headers:

```python
import requestx

# API Key in header
headers = {"X-API-Key": "your-api-key"}

response = requestx.get(
    "https://api.example.com/data",
    headers=headers
)
```

### With Client

```python
import requestx

with requestx.Client(
    headers={"X-API-Key": "your-api-key"}
) as client:
    response = client.get("https://api.example.com/data")
```

## Query Parameter Authentication

Some APIs accept tokens as query parameters:

```python
import requestx

response = requestx.get(
    "https://api.example.com/data",
    params={"api_key": "your-api-key"}
)
```

## OAuth 2.0 Flows

### Client Credentials Flow

```python
import requestx

def get_oauth_token(client_id: str, client_secret: str, token_url: str) -> str:
    response = requestx.post(
        token_url,
        data={
            "grant_type": "client_credentials",
            "client_id": client_id,
            "client_secret": client_secret,
        }
    )
    response.raise_for_status()
    return response.json()["access_token"]

# Get token and use it
token = get_oauth_token(
    "your-client-id",
    "your-client-secret",
    "https://auth.example.com/oauth/token"
)

with requestx.Client(auth=requestx.Auth.bearer(token)) as client:
    data = client.get("https://api.example.com/protected").json()
```

### Token Refresh

```python
import requestx
from datetime import datetime, timedelta

class TokenManager:
    def __init__(self, client_id: str, client_secret: str, token_url: str):
        self.client_id = client_id
        self.client_secret = client_secret
        self.token_url = token_url
        self.access_token = None
        self.expires_at = None

    def get_token(self) -> str:
        if self.access_token and self.expires_at and datetime.now() < self.expires_at:
            return self.access_token

        response = requestx.post(
            self.token_url,
            data={
                "grant_type": "client_credentials",
                "client_id": self.client_id,
                "client_secret": self.client_secret,
            }
        )
        response.raise_for_status()
        data = response.json()

        self.access_token = data["access_token"]
        expires_in = data.get("expires_in", 3600)
        self.expires_at = datetime.now() + timedelta(seconds=expires_in - 60)

        return self.access_token

# Usage
token_manager = TokenManager(
    "client-id",
    "client-secret",
    "https://auth.example.com/oauth/token"
)

with requestx.Client() as client:
    # Token is refreshed automatically when needed
    response = client.get(
        "https://api.example.com/data",
        headers={"Authorization": f"Bearer {token_manager.get_token()}"}
    )
```

## Async Authentication

Using authentication with `AsyncClient`:

```python
import asyncio
import requestx

async def main():
    async with requestx.AsyncClient(
        auth=requestx.Auth.bearer("your-token")
    ) as client:
        response = await client.get("https://api.example.com/data")
        print(response.json())

asyncio.run(main())
```

### Async Token Refresh

```python
import asyncio
import requestx
from datetime import datetime, timedelta

class AsyncTokenManager:
    def __init__(self, client_id: str, client_secret: str, token_url: str):
        self.client_id = client_id
        self.client_secret = client_secret
        self.token_url = token_url
        self.access_token = None
        self.expires_at = None
        self._lock = asyncio.Lock()

    async def get_token(self, client: requestx.AsyncClient) -> str:
        async with self._lock:
            if self.access_token and self.expires_at and datetime.now() < self.expires_at:
                return self.access_token

            response = await client.post(
                self.token_url,
                data={
                    "grant_type": "client_credentials",
                    "client_id": self.client_id,
                    "client_secret": self.client_secret,
                }
            )
            response.raise_for_status()
            data = response.json()

            self.access_token = data["access_token"]
            expires_in = data.get("expires_in", 3600)
            self.expires_at = datetime.now() + timedelta(seconds=expires_in - 60)

            return self.access_token

async def main():
    token_manager = AsyncTokenManager(
        "client-id",
        "client-secret",
        "https://auth.example.com/oauth/token"
    )

    async with requestx.AsyncClient() as client:
        token = await token_manager.get_token(client)
        response = await client.get(
            "https://api.example.com/data",
            headers={"Authorization": f"Bearer {token}"}
        )
        print(response.json())

asyncio.run(main())
```

## Proxy Authentication

Authenticate with proxy servers:

```python
import requestx

proxy = requestx.Proxy(
    url="http://proxy.example.com:8080",
    username="proxy-user",
    password="proxy-pass"
)

with requestx.Client(proxy=proxy) as client:
    response = client.get("https://api.example.com/data")
```

## Security Best Practices

1. **Never hardcode credentials** - Use environment variables or secret managers

```python
import os
import requestx

api_key = os.environ.get("API_KEY")
auth = requestx.Auth.bearer(api_key)
```

2. **Use HTTPS** - Always use HTTPS for authenticated requests

```python
# Good
response = requestx.get("https://api.example.com/data", auth=auth)

# Bad - credentials sent in plain text
response = requestx.get("http://api.example.com/data", auth=auth)
```

3. **Rotate tokens regularly** - Implement token refresh for long-running applications

4. **Limit token scope** - Request only the permissions you need

5. **Handle authentication errors gracefully**

```python
import requestx
from requestx import HTTPStatusError

try:
    response = requestx.get(
        "https://api.example.com/data",
        auth=requestx.Auth.bearer("token")
    )
    response.raise_for_status()
except HTTPStatusError as e:
    if e.response.status_code == 401:
        print("Authentication failed - check your credentials")
    elif e.response.status_code == 403:
        print("Access denied - insufficient permissions")
    else:
        raise
```
