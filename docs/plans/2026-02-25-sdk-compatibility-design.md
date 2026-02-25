# SDK Compatibility via isinstance Patching

**Date:** 2026-02-25
**Status:** Approved
**Author:** Design session with user

## Problem

AI SDKs (OpenAI, Anthropic) perform strict `isinstance(http_client, httpx.Client)` checks when accepting custom HTTP clients. RequestX's `Client` class doesn't inherit from `httpx.Client`, causing type validation failures:

```python
from openai import OpenAI
import requestx

client = OpenAI(http_client=requestx.Client())
# TypeError: Invalid `http_client` argument; Expected an instance of `httpx.Client`
```

This blocks RequestX from being used as a drop-in performance upgrade for AI SDK users.

## Goal

Make `isinstance(requestx.Client(), httpx.Client)` return `True` without changing RequestX's Rust-first architecture or requiring inheritance from httpx.Client.

## Solution: Global isinstance Patching

Patch Python's `type.__instancecheck__` at import time to recognize requestx.Client instances when checked against httpx.Client.

### Architecture

**Components:**
1. **Patch function** - `_patch_httpx_isinstance()` wraps `type.__instancecheck__`
2. **Instance detection** - Identifies requestx clients by class name + module name
3. **Import-time execution** - Runs automatically when `import requestx` happens
4. **Fallback behavior** - Delegates to original isinstance for all other checks

**Location:** `python/requestx/__init__.py`

**Scope:** Global - affects all isinstance checks in the process, but custom logic only triggers for httpx.Client/AsyncClient checks.

## Implementation

### Patch Function

```python
def _patch_httpx_isinstance():
    """Patch isinstance to recognize requestx.Client as httpx.Client."""
    import httpx

    # Store original isinstance behavior
    original_instancecheck = type.__instancecheck__

    def custom_instancecheck(cls, instance):
        # Special case: checking if instance is httpx.Client
        if cls is httpx.Client:
            instance_type = type(instance)
            # Accept actual httpx.Client OR requestx.Client
            if (instance_type.__name__ == 'Client' and
                instance_type.__module__.startswith('requestx')):
                return True

        # Special case: checking if instance is httpx.AsyncClient
        if cls is httpx.AsyncClient:
            instance_type = type(instance)
            if (instance_type.__name__ == 'AsyncClient' and
                instance_type.__module__.startswith('requestx')):
                return True

        # All other cases: use original behavior
        return original_instancecheck(cls, instance)

    # Apply the patch globally
    type.__instancecheck__ = custom_instancecheck
```

### Integration Point

Add to `python/requestx/__init__.py` at the end, after all imports:

```python
# At end of __init__.py, before __all__
_patch_httpx_isinstance()
```

### Detection Strategy

- Match class name: `type(instance).__name__ == 'Client'`
- Match module: `type(instance).__module__.startswith('requestx')`
- Both conditions must be true
- Works for both sync (`Client`) and async (`AsyncClient`)

## Testing Strategy

### Test Coverage

**1. Basic isinstance checks:**
```python
import requestx
import httpx

client = requestx.Client()
assert isinstance(client, httpx.Client)

async_client = requestx.AsyncClient()
assert isinstance(async_client, httpx.AsyncClient)
```

**2. SDK integration tests:**
```python
from openai import OpenAI
from anthropic import Anthropic

# OpenAI sync
client = OpenAI(api_key='fake', http_client=requestx.Client())

# Anthropic sync
client = Anthropic(api_key='fake', http_client=requestx.Client())

# OpenAI async
from openai import AsyncOpenAI
client = AsyncOpenAI(api_key='fake', http_client=requestx.AsyncClient())

# Anthropic async
from anthropic import AsyncAnthropic
client = AsyncAnthropic(api_key='fake', http_client=requestx.AsyncClient())
```

**3. Regression tests:**
```python
# Ensure real httpx.Client instances still pass
real_httpx_client = httpx.Client()
assert isinstance(real_httpx_client, httpx.Client)
```

### Edge Cases

- **httpx not installed**: Acceptable failure (requestx depends on httpx)
- **Import order**: Patch applies globally regardless of import order
- **Multiple requestx versions**: `startswith('requestx')` covers all versions
- **Mock/Test clients**: Any `requestx.*` class named `Client` passes (intentional)

### Test Location

`tests_requestx/test_sdk_compatibility.py` (new file)

## Trade-offs

**Pros:**
- ✅ Transparent - users just `import requestx` and it works
- ✅ No API changes - existing code unaffected
- ✅ Minimal surface area - single function patch
- ✅ Works with all SDKs using isinstance checks

**Cons:**
- ⚠️ Global scope - affects all isinstance checks (narrow detection mitigates this)
- ⚠️ Fragile - depends on class/module naming conventions
- ⚠️ Could break if httpx changes internal structure
- ⚠️ Non-standard approach (monkey patching stdlib)

## Alternatives Considered

**Option A: Full inheritance from httpx.Client**
- Rejected: Would require rewriting requestx.Client to extend httpx, breaking Rust-first architecture
- Would need to implement all httpx internal methods

**Option B: Separate wrapper class (HTTPXClient)**
- Rejected: Requires users to import different class for SDK usage
- Adds API surface and documentation complexity

**Option C: This approach** ✅ Selected
- Minimal changes, maximum transparency
- Acceptable trade-offs for the use case

## Success Criteria

- [ ] `isinstance(requestx.Client(), httpx.Client)` returns True
- [ ] `isinstance(requestx.AsyncClient(), httpx.AsyncClient)` returns True
- [ ] OpenAI SDK accepts `requestx.Client()` as `http_client`
- [ ] Anthropic SDK accepts `requestx.Client()` as `http_client`
- [ ] Real `httpx.Client` instances still pass isinstance checks
- [ ] All existing requestx tests continue passing
- [ ] New SDK compatibility tests pass

## Documentation Updates

Update README.md to include AI SDK usage examples:

```python
# OpenAI
import requestx
from openai import OpenAI

client = OpenAI(http_client=requestx.Client())

# Anthropic
from anthropic import Anthropic
client = Anthropic(http_client=requestx.Client())
```

## Implementation Checklist

1. Write `_patch_httpx_isinstance()` function
2. Add patch call to `__init__.py`
3. Write test file `tests_requestx/test_sdk_compatibility.py`
4. Run full test suite to verify no regressions
5. Update README.md with SDK examples
6. Update CLAUDE.md if needed
