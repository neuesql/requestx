# Basic Usage Examples

This page contains common usage patterns for RequestX.

## Simple GET Request

```python
import requestx

response = requestx.get("https://httpbin.org/get")
print(f"Status: {response.status_code}")
print(f"JSON: {response.json()}")
```

## POST with JSON Data

```python
import requestx

response = requestx.post(
    "https://httpbin.org/post",
    json={
        "name": "John Doe",
        "email": "john@example.com",
        "age": 30
    }
)

data = response.json()
print(f"Sent: {data['json']}")
```

## POST with Form Data

```python
import requestx

response = requestx.post(
    "https://httpbin.org/post",
    data={
        "username": "johndoe",
        "password": "secret123"
    }
)

data = response.json()
print(f"Form: {data['form']}")
```

## Custom Headers

```python
import requestx

response = requestx.get(
    "https://httpbin.org/headers",
    headers={
        "User-Agent": "MyApp/1.0",
        "Accept": "application/json",
        "X-Custom-Header": "custom-value"
    }
)

print(response.json()["headers"])
```

## Query Parameters

```python
import requestx

response = requestx.get(
    "https://httpbin.org/get",
    params={
        "search": "python",
        "page": 1,
        "limit": 10
    }
)

print(f"URL: {response.url}")
# https://httpbin.org/get?search=python&page=1&limit=10
```

## Using Client with Base URL

```python
import requestx

with requestx.Client(base_url="https://jsonplaceholder.typicode.com") as client:
    # GET all users
    users = client.get("/users").json()
    print(f"Found {len(users)} users")

    # GET single user
    user = client.get("/users/1").json()
    print(f"User: {user['name']}")

    # GET user's posts
    posts = client.get("/users/1/posts").json()
    print(f"User has {len(posts)} posts")
```

## Authentication

### Basic Auth

```python
import requestx

response = requestx.get(
    "https://httpbin.org/basic-auth/user/pass",
    auth=requestx.Auth.basic("user", "pass")
)

print(f"Authenticated: {response.json()['authenticated']}")
```

### Bearer Token

```python
import requestx

response = requestx.get(
    "https://httpbin.org/bearer",
    auth=requestx.Auth.bearer("my-secret-token")
)

print(f"Token: {response.json()['token']}")
```

## Error Handling

```python
import requestx
from requestx import HTTPStatusError, ConnectError, TimeoutException

def fetch_user(user_id: int) -> dict:
    try:
        response = requestx.get(
            f"https://jsonplaceholder.typicode.com/users/{user_id}",
            timeout=5.0
        )
        response.raise_for_status()
        return response.json()

    except HTTPStatusError as e:
        if e.response.status_code == 404:
            print(f"User {user_id} not found")
            return None
        raise

    except TimeoutException:
        print("Request timed out")
        raise

    except ConnectError:
        print("Could not connect to server")
        raise

# Usage
user = fetch_user(1)
if user:
    print(f"User: {user['name']}")
```

## Timeout Configuration

```python
import requestx

# Simple timeout
response = requestx.get(
    "https://httpbin.org/delay/1",
    timeout=5.0
)

# Detailed timeout
timeout = requestx.Timeout(
    timeout=30.0,  # Total timeout
    connect=5.0,   # Connection timeout
    read=15.0,     # Read timeout
)

response = requestx.get(
    "https://httpbin.org/delay/2",
    timeout=timeout
)
```

## Session Cookies

```python
import requestx

with requestx.Client() as client:
    # Set cookies via request
    client.get("https://httpbin.org/cookies/set/session/abc123")

    # Subsequent requests include the cookie
    response = client.get("https://httpbin.org/cookies")
    print(response.json()["cookies"])  # {'session': 'abc123'}
```

## Redirect Handling

```python
import requestx

# Follow redirects (default)
response = requestx.get("https://httpbin.org/redirect/3")
print(f"Final URL: {response.url}")

# Disable redirects
response = requestx.get(
    "https://httpbin.org/redirect/1",
    follow_redirects=False
)
print(f"Status: {response.status_code}")  # 302
print(f"Location: {response.headers.get('location')}")
```

## Response Inspection

```python
import requestx

response = requestx.get("https://httpbin.org/get")

# Status information
print(f"Status Code: {response.status_code}")
print(f"Reason: {response.reason_phrase}")
print(f"Success: {response.is_success}")
print(f"Is Error: {response.is_error}")

# Headers
print(f"Content-Type: {response.headers.get('content-type')}")

# Content
print(f"Text: {response.text[:100]}...")
print(f"Bytes: {len(response.content)} bytes")

# JSON
data = response.json()
print(f"JSON keys: {list(data.keys())}")

# Timing
print(f"Elapsed: {response.elapsed:.3f} seconds")
```

## Multiple Requests with Client

```python
import requestx

with requestx.Client(
    base_url="https://jsonplaceholder.typicode.com",
    headers={"Accept": "application/json"}
) as client:
    # Fetch multiple resources
    users = client.get("/users").json()
    posts = client.get("/posts").json()
    comments = client.get("/comments").json()

    print(f"Users: {len(users)}")
    print(f"Posts: {len(posts)}")
    print(f"Comments: {len(comments)}")

    # Create a post
    new_post = client.post(
        "/posts",
        json={
            "title": "My Post",
            "body": "This is my post content",
            "userId": 1
        }
    ).json()
    print(f"Created post: {new_post['id']}")

    # Update a post
    updated = client.put(
        "/posts/1",
        json={
            "id": 1,
            "title": "Updated Title",
            "body": "Updated content",
            "userId": 1
        }
    ).json()
    print(f"Updated: {updated['title']}")

    # Delete a post
    response = client.delete("/posts/1")
    print(f"Deleted: {response.status_code}")
```
