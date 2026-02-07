"""Logging utilities for the HTTP benchmark framework."""

from loguru import logger
import sys


def setup_logging():
    """Set up logging configuration for the framework."""
    # Remove default logger to avoid duplicate logs
    logger.remove()

    # Add console handler with detailed format
    logger.add(
        sys.stdout,
        format="<green>{time:YYYY-MM-DD HH:mm:ss}</green> | <level>{level: <8}</level> | <cyan>{name}</cyan>:<cyan>{function}</cyan>:<cyan>{line}</cyan> - <level>{message}</level>",
        level="INFO",
    )

    # Add file handler for detailed logs
    logger.add(
        "benchmark_framework.log",
        format="{time:YYYY-MM-DD HH:mm:ss} | {level: <8} | {name}:{function}:{line} - {message}",
        level="DEBUG",
        rotation="10 MB",
        retention="10 days",
    )

    return logger


# Initialize the logger
app_logger = setup_logging()
