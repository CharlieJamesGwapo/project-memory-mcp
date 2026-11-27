def parse_format(value):
    normalized = value.lower()
    if normalized not in {"json", "csv"}:
        raise ValueError(f"unsupported format: {value!r}")
    return normalized
