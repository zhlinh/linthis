"""A simple module with no lint errors."""


def hello(name: str) -> str:
    """Return a greeting message."""
    return f"Hello, {name}!"


if __name__ == "__main__":
    print(hello("World"))
