def label(name):
    """Return the label for a nonempty recipient."""
    name = name.strip()
    if not name:
        raise ValueError("recipient is required")
    return f"Recipient: {name}"
