import logging
import logging_loki

LOKI_URL = "https://logs-prod-017.grafana.net/loki/api/v1/push"
LOKI_USER = "488816"
LOKI_PASSWORD = "YOUR_GRAFANA_API_TOKEN_HERE"

import urllib3
import warnings

# Disable SSL warnings
warnings.filterwarnings('ignore')
urllib3.disable_warnings()

handler = logging_loki.LokiHandler(
    url=LOKI_URL, 
    tags={"application": "my-app"},
    auth=(LOKI_USER, LOKI_PASSWORD),
    version="1",
)

logger = logging.getLogger("my-logger")
logger.addHandler(handler)
logger.error(
    "Something happened", 
    extra={"tags": {"service": "my-service"}},
)