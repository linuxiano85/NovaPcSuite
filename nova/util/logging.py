"""Utility functions for logging setup."""

import logging
import sys
from pathlib import Path
from typing import Optional

from rich.console import Console
from rich.logging import RichHandler


def setup_logging(
    level: str = "INFO", 
    log_file: Optional[Path] = None,
    console: Optional[Console] = None
) -> logging.Logger:
    """Set up structured logging with Rich formatting."""
    
    if console is None:
        console = Console()
    
    # Create logger
    logger = logging.getLogger("novapcsuite")
    logger.setLevel(getattr(logging, level.upper()))
    
    # Remove existing handlers
    for handler in logger.handlers[:]:
        logger.removeHandler(handler)
    
    # Console handler with Rich formatting
    console_handler = RichHandler(
        console=console,
        show_time=True,
        show_path=True,
        rich_tracebacks=True,
        tracebacks_show_locals=True,
    )
    console_handler.setLevel(getattr(logging, level.upper()))
    
    # Format for console
    console_format = logging.Formatter(
        fmt="%(message)s",
        datefmt="[%X]"
    )
    console_handler.setFormatter(console_format)
    logger.addHandler(console_handler)
    
    # File handler if specified
    if log_file:
        log_file.parent.mkdir(parents=True, exist_ok=True)
        file_handler = logging.FileHandler(log_file)
        file_handler.setLevel(logging.DEBUG)
        
        # Detailed format for file
        file_format = logging.Formatter(
            fmt="%(asctime)s - %(name)s - %(levelname)s - %(filename)s:%(lineno)d - %(message)s",
            datefmt="%Y-%m-%d %H:%M:%S"
        )
        file_handler.setFormatter(file_format)
        logger.addHandler(file_handler)
    
    return logger


def get_logger(name: str = "novapcsuite") -> logging.Logger:
    """Get a logger instance."""
    return logging.getLogger(name)