"""Base model for the HTTP benchmark framework."""

from abc import ABC
from typing import Any, Dict


class BaseModel(ABC):
    """Base model class for all entities in the benchmark framework."""

    def to_dict(self) -> Dict[str, Any]:
        """Convert the model to a dictionary representation."""
        result = {}
        for attr, value in self.__dict__.items():
            if not attr.startswith("_"):  # Skip private attributes
                if hasattr(value, "to_dict"):
                    result[attr] = value.to_dict()
                else:
                    result[attr] = value
        return result

    def __repr__(self) -> str:
        """String representation of the model."""
        attrs = []
        for attr, value in self.__dict__.items():
            if not attr.startswith("_"):
                attrs.append(f"{attr}={repr(value)}")
        return f"{self.__class__.__name__}({', '.join(attrs)})"
