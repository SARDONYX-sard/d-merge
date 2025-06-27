from typing import List

import mobase  # type: ignore
from PyQt6.QtGui import QIcon
from PyQt6.QtWidgets import QApplication

from .data_grid import PatchSelector


class DMergePlugin(mobase.IPluginTool):
    _organizer: mobase.IOrganizer

    def __init__(self):
        super().__init__()

    def init(self, organizer: mobase.IOrganizer):
        self._organizer = organizer
        return True

    def name(self) -> str:
        return "D_Merge MO2 Plugin"

    def author(self) -> str:
        return "SARDONYX"

    def description(self) -> str:
        return self.__tr("Nemesis patcher")

    def version(self) -> mobase.VersionInfo:
        return mobase.VersionInfo(0, 1, 0, mobase.ReleaseType.ALPHA)

    def isActive(self) -> bool:
        variant = self._organizer.pluginSetting(self.name(), "enabled")
        if variant is bool:
            return variant
        else:
            return False

    def settings(self) -> List[mobase.PluginSetting]:
        return [mobase.PluginSetting("enabled", "enable this plugin", True)]

    def localizedName(self) -> str:
        return self.__tr("Plugin Name")

    def __tr(self, txt: str) -> str:
        return QApplication.translate("D_MergePlugin", txt)

    def icon(self: mobase.IPluginTool) -> QIcon:
        return QIcon()

    def tooltip(self: mobase.IPluginTool) -> str:
        return "Alterative Nemesis patcher"

    def displayName(self: mobase.IPluginTool) -> str:
        return "D Merge"

    # This will run automatically when we click the tool button.
    def display(self: mobase.IPluginTool) -> None:
        window = PatchSelector()
        window.show()
