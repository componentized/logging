# `to-stdout`

Implements wasi:logging/logging@0.1.0-draft writing log statements to stdout via wasi:cli/stdout@0.2.3. Timestamps for each log statement are generated from wasi:clocks/wall-clock@0.2.3.

Log output takes the form:

```
2025-02-10 18:04:35.925Z LEVEL [component]: message
```
