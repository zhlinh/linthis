import os  # noqa: F401
import sys  # noqa: F401

x = 1
y = 2


def foo():
    unused_var = 42  # noqa: F841
    print("hello")
