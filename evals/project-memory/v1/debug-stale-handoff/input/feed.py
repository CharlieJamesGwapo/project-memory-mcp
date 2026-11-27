"""Ascending activity IDs; after_id is the last ID already seen by the caller."""


def next_page(activity_ids, after_id=None, limit=2):
    if limit < 1:
        raise ValueError("limit must be positive")
    remaining = sorted(activity_ids)
    if after_id is not None:
        remaining = [item for item in remaining if item >= after_id]
    return remaining[:limit]
