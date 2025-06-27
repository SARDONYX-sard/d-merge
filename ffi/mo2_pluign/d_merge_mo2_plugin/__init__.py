import mobase  # type: ignore

from .plugin import DMergePlugin


def createPlugin() -> mobase.IPlugin:
    return DMergePlugin()
