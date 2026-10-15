# `to-stderr`

Implements wasi:logging/logging@0.1.0-draft writing log statements to stderr via wasi:cli/stderr@0.3.0. A composition of [`to-stdout`](../to-stdout/) with its wasi:cli/stdout@0.3.0 import satisfied by wasi:cli/stderr@0.3.0.

Log output takes the form:

```
2025-02-10 18:04:35.925Z LEVEL [component]: message
```
