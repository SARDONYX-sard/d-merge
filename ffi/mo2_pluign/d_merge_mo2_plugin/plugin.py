import typing
from typing import List

import mobase  # type: ignore
from PyQt6.QtCore import Qt
from PyQt6.QtGui import QIcon
from PyQt6.QtWidgets import QApplication, QMessageBox, QWidget

from .data_grid import PatchSelector

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
        return [
            mobase.PluginSetting("enabled", "enable this plugin", True),
            mobase.PluginSetting("d_merge_path", "d_merge.exe path", ""),
        ]

    def localizedName(self) -> str:
        return self.__tr("D_Merge")

    def __tr(self, txt: str) -> str:
        return QApplication.translate("D_MergePlugin", txt)

    def icon(self: mobase.IPluginTool) -> QIcon:
        return QIcon("plugins/d_merge_mo2_plugin/icon.ico")

    def tooltip(self: mobase.IPluginTool) -> str:
        return "Alterative Nemesis patcher"

    def displayName(self: mobase.IPluginTool) -> str:
        return "D Merge"

    def setParentWidget(self, parent: QWidget):
        self.__parentWidget = parent

    def display(self) -> None:
        try:
            # self.run_merge_tool()
            self.show_mod_list(self._organizer.modList().allModsByProfilePriority())

        except Exception as e:
            QMessageBox.critical(
                self.__parentWidget,
                self.__tr("表示エラー"),
                self.__tr(f"Modリストの表示に失敗しました: {e}"),
            )

    def show_mod_list(self, items: ModList):
        self._modListWindow = PatchSelector()
        self._modListWindow.setWindowTitle("D_Merge")
        self._modListWindow.setAttribute(Qt.WidgetAttribute.WA_DeleteOnClose)
        self._modListWindow.show()

    def run_merge_tool(self):
        merge_path_variant = self._organizer.pluginSetting(self.name(), "d_merge_path")
        if not merge_path_variant:
            QMessageBox.warning(
                self.__parentWidget,
                self.__tr("実行エラー"),
                self.__tr("d_merge.exe のパスが設定されていません。"),
            )
            return

        merge_path = str(merge_path_variant)

        args: List[str] = []
        try:
            handle = self._organizer.startApplication(
                merge_path, args, ignoreCustomOverwrite=True
            )
            result, exitCode = self._organizer.waitForApplication(handle)

            if not result or exitCode != 0:
                QMessageBox.warning(
                    self.__parentWidget,
                    self.__tr("実行失敗"),
                    self.__tr(
                        f"d_merge.exe の実行に失敗しました。終了コード: {exitCode}"
                    ),
                )

        except Exception as e:
            QMessageBox.critical(
                self.__parentWidget,
                self.__tr("実行エラー"),
                self.__tr(f"d_merge.exe の起動に失敗しました: {e}"),
            )
