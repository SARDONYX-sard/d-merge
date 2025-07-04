import typing
from typing import List

import mobase  # type: ignore
from PyQt6.QtCore import Qt
from PyQt6.QtGui import QIcon
from PyQt6.QtWidgets import QApplication, QListWidget, QMessageBox, QVBoxLayout, QWidget

# from .data_grid import PatchSelector

ModList = typing.Iterable[typing.Optional[str]]


class DMergePlugin(mobase.IPluginTool):
    _organizer: mobase.IOrganizer
    _modListWindow = None

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

    def requirements(self) -> list[mobase.IPluginRequirement]:
        return [
            mobase.PluginRequirementFactory.gameDependency(
                ["Skyrim", "Skyrim Special Edition", "Skyrim VR"]
            )
        ]

    def isActive(self) -> bool:
        variant = self._organizer.pluginSetting(self.name(), "enabled")
        if variant is bool:
            return variant
        else:
            return False

    def settings(self) -> List[mobase.PluginSetting]:
        return [mobase.PluginSetting("enabled", "enable this plugin", True)]

    def localizedName(self) -> str:
        return self.__tr("D_Merge")

    def __tr(self, txt: str) -> str:
        return QApplication.translate("D_MergePlugin", txt)

    def icon(self: mobase.IPluginTool) -> QIcon:
        # return QIcon("plugins/d_merge_mo2_plugin/icon.ico")
        return QIcon()

    def tooltip(self: mobase.IPluginTool) -> str:
        return "Alterative Nemesis patcher"

    def displayName(self: mobase.IPluginTool) -> str:
        return "D Merge"

    def setParentWidget(self, parent: QWidget):
        self.__parentWidget = parent

    def display(self) -> None:
        try:
            self.show_mod_list(self._organizer.modList().allMods())

        except Exception as e:
            QMessageBox.critical(
                self.__parentWidget,
                self.__tr("表示エラー"),
                self.__tr(f"Modリストの表示に失敗しました: {e}"),
            )

    def show_mod_list(self, items: ModList):
        self._modListWindow = QWidget()
        self._modListWindow.setWindowTitle("Mod List")
        self._modListWindow.setAttribute(Qt.WidgetAttribute.WA_DeleteOnClose)

        layout = QVBoxLayout()
        list_widget = QListWidget()
        list_widget.addItems(items)
        layout.addWidget(list_widget)

        self._modListWindow.setLayout(layout)
        self._modListWindow.resize(400, 300)
        self._modListWindow.show()
