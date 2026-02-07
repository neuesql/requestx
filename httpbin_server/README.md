# HTTPBin Server Setup

This directory contains the Dockerized environment for the `httpbin` server, which serves as the primary target for the HTTP client benchmark framework. It provides two different configurations to suit various testing needs: a high-performance **Traefik Load Balancer** setup and a **Simple HTTPBin** setup.

## 📋 Overview

The server setup is designed to provide a consistent and controlled environment for performance testing. Using a local server eliminates network variability and allows for deep analysis of client library behavior under different conditions.

- **Traefik Load Balancer Setup**: Simulates a production-like environment with a reverse proxy, SSL/TLS termination, and load balancing across multiple instances.
- **Simple HTTPBin Setup**: A lightweight, single-instance configuration for basic testing and debugging.

---

## ⚡ Quick Start

### Start Traefik Load Balancer (Recommended)
```bash
docker-compose -f httpbin_server/docker-compose.yml up -d
```

### Start Simple HTTPBin
```bash
docker-compose -f httpbin_server/docker-compose.simple.yml up -d
```

### Stop the Server
```bash
docker-compose -f httpbin_server/docker-compose.yml down
# OR
docker-compose -f httpbin_server/docker-compose.simple.yml down
```

---

## 🏗 Traefik Load Balancer Setup (Advanced)

The advanced setup uses **Traefik v3** as a reverse proxy and load balancer. This is the recommended configuration for comprehensive benchmarking.

### Features
- **Load Balancing**: Distributes incoming requests across 3 `httpbin` instances (`httpbin1`, `httpbin2`, `httpbin3`) using a round-robin algorithm.
- **HTTPS Support**: Provides both HTTP and HTTPS endpoints using self-signed certificates.
- **Automatic Discovery**: Traefik automatically discovers and routes traffic to the `httpbin` services.
- **Health Monitoring**: Includes health checks for both Traefik and the backend instances.
- **Resource Limits**: Each `httpbin` instance is limited to 1.0 CPU and 512MB RAM to ensure stable performance.

### Endpoints
- **HTTP**: `http://localhost/` (Port 80)
- **HTTPS**: `https://localhost/` (Port 443)
- **Traefik Dashboard**: `http://localhost:8080` (For monitoring and debugging)

### Testing the Setup
```bash
# Test HTTP endpoint
curl -I http://localhost/get

# Test HTTPS endpoint (using -k to ignore self-signed certificate)
curl -k -I https://localhost/get
```

### Starting the Server
```bash
docker-compose -f httpbin_server/docker-compose.yml up -d
```

---

## 🧪 Simple HTTPBin Setup (Basic)

The simple setup runs a single `httpbin` instance exposed directly on port 80.

### When to use this setup:
- Basic connectivity tests.
- Debugging custom headers or request bodies.
- Low-resource environments where a full load balancer is not needed.

### Endpoints
- **HTTP**: `http://localhost/` (Port 80)

### Testing the Setup
```bash
curl -I http://localhost/get
```

### Starting the Server
```bash
docker-compose -f httpbin_server/docker-compose.simple.yml up -d
```

---

## ⚙️ Configuration Details

### Traefik Static Configuration (`traefik_config.yml`)
Configures core entrypoints (80, 443), log levels, and the Docker/File providers.

### Dynamic Routing Rules (`traefik/dynamic/httpbin-routing.yml`)
Defines how requests are routed to the load-balanced services. It matches requests with `Host: localhost` and maps them to the `httpbin` service consisting of three backend URLs.

### TLS/SSL Setup
- **Config**: `traefik/dynamic/tls.yml` points to the certificates.
- **Certificates**: Located in `certs/server.crt` and `certs/server.key`.
- **Note**: These are self-signed certificates. You may need to disable SSL verification in your benchmark client or trust the certificate.

### Load Balancing Behavior
Traefik uses a round-robin strategy by default. You can verify this by checking the `X-Forwarded-For` or other headers if the backend instances were configured to log their unique IDs, but in this setup, it's handled transparently by Traefik.

---

## 🛠 Troubleshooting

### Port Conflicts
If you see an error like `Bind for 0.0.0.0:80 failed: port is already allocated`, ensure no other web servers (like Nginx or Apache) are running on your host.
- Check ports: `lsof -i :80` or `netstat -tuln | grep :80`

### Docker Socket Permissions
Traefik needs access to `/var/run/docker.sock` to discover services. If you encounter permission issues, ensure your user is in the `docker` group or run with appropriate permissions.

### SSL Verification Errors
When testing HTTPS, you might see `CERTIFICATE_VERIFY_FAILED`.
- **Curl**: Use the `-k` or `--insecure` flag.
- **Benchmark Tool**: Ensure the client adapter handles self-signed certificates (most adapters in this project support an `--insecure` or similar flag if implemented).

---

## 📈 Integration with Benchmark Framework

This server setup is optimized for use with the `http-client-benchmarker` CLI.

### Example: Benchmarking against the Load Balancer
```bash
# Benchmark httpx against the HTTP endpoint
python -m http_benchmark.cli --url http://localhost/get --client httpx --concurrency 50 --duration 60

# Benchmark aiohttp against the HTTPS endpoint
python -m http_benchmark.cli --url https://localhost/get --client aiohttp --concurrency 50 --duration 60 --async
```

### Recommended Setup for Performance Testing
For high-concurrency benchmarks, always use the Traefik setup. It better reflects real-world scenarios where requests are proxied and distributed across multiple worker nodes, allowing you to test how client libraries handle connection pooling and keep-alive over a load balancer.
