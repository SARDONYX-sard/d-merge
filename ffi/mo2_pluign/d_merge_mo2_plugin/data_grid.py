import sys
from typing import List, Optional, Tuple

from PyQt6.QtCore import (
    QAbstractTableModel,
    QMimeData,
    QModelIndex,
    Qt,
)
from PyQt6.QtWidgets import (
    QAbstractItemView,
    QApplication,
    QHeaderView,
    QMessageBox,
    QPushButton,
    QTableView,
    QVBoxLayout,
    QWidget,
)

# データの型: (id:int, modname:str, url:str, priority:int, checked:bool)
PatchDataType = Tuple[int, str, str, int, bool]


class PatchTableModel(QAbstractTableModel):
    def __init__(self, data: Optional[List[PatchDataType]] = None) -> None:
        super().__init__()
        self.patch_data: List[PatchDataType] = data or []

    def rowCount(self, parent: QModelIndex = QModelIndex()) -> int:
        return len(self.patch_data)

    def columnCount(self, parent: QModelIndex = QModelIndex()) -> int:
        return 5

    def data(
        self, index: QModelIndex, role: int = Qt.ItemDataRole.DisplayRole
    ) -> Optional[str]:
        if not index.isValid():
            return None

        row, col = index.row(), index.column()
        if role == Qt.ItemDataRole.DisplayRole:
            item = self.patch_data[row]
            if col == 0:
                return ""
            elif col == 1:
                return str(item[0])
            elif col == 2:
                return item[1]
            elif col == 3:
                return item[2]
            elif col == 4:
                return str(item[3])

        if role == Qt.ItemDataRole.CheckStateRole and col == 0:
            return (
                Qt.CheckState.Checked
                if self.patch_data[row][4]
                else Qt.CheckState.Unchecked
            )

        return None

    def setData(
        self, index: QModelIndex, value: object, role: int = Qt.ItemDataRole.EditRole
    ) -> bool:
        if not index.isValid():
            return False

        row, col = index.row(), index.column()
        if role == Qt.ItemDataRole.CheckStateRole and col == 0:
            checked = value == Qt.CheckState.Checked
            item = self.patch_data[row]
            self.patch_data[row] = (item[0], item[1], item[2], item[3], checked)
            self.dataChanged.emit(index, index, [Qt.ItemDataRole.CheckStateRole])
            return True

        return False

    def flags(self, index: QModelIndex) -> Qt.ItemFlag:
        if not index.isValid():
            return Qt.ItemFlag.NoItemFlags

        flags = (
            Qt.ItemFlag.ItemIsEnabled
            | Qt.ItemFlag.ItemIsSelectable
            | Qt.ItemFlag.ItemIsDragEnabled
            | Qt.ItemFlag.ItemIsDropEnabled
        )
        if index.column() == 0:
            flags |= Qt.ItemFlag.ItemIsUserCheckable

        return flags

    def headerData(
        self,
        section: int,
        orientation: Qt.Orientation,
        role: int = Qt.ItemDataRole.DisplayRole,
    ) -> Optional[str]:
        headers = ["選択", "ID", "Mod名", "URL", "優先度"]
        if (
            role == Qt.ItemDataRole.DisplayRole
            and orientation == Qt.Orientation.Horizontal
        ):
            if 0 <= section < len(headers):
                return headers[section]
        return None

    def supportedDropActions(self) -> Qt.DropAction:
        return Qt.DropAction.MoveAction

    def mimeTypes(self) -> List[str]:
        return ["application/x-qabstractitemmodeldatalist"]

    def mimeData(self, indexes: List[QModelIndex]) -> QMimeData:
        mimedata = QMimeData()
        rows = {index.row() for index in indexes}
        row = list(rows)[0]
        mimedata.setData(
            "application/x-qabstractitemmodeldatalist", bytes(str(row), "utf-8")
        )
        return mimedata

    def dropMimeData(
        self,
        data: QMimeData,
        action: Qt.DropAction,
        row: int,
        column: int,
        parent: QModelIndex,
    ) -> bool:
        if action == Qt.DropAction.IgnoreAction:
            return True
        if not data.hasFormat("application/x-qabstractitemmodeldatalist"):
            return False

        try:
            from_row = int(
                bytes(data.data("application/x-qabstractitemmodeldatalist")).decode(
                    "utf-8"
                )
            )
        except Exception:
            return False

        if row == -1:
            if parent.isValid():
                row = parent.row()
            else:
                row = self.rowCount()

        if from_row < 0 or from_row >= self.rowCount():
            return False

        if row > from_row:
            row -= 1

        self.beginMoveRows(QModelIndex(), from_row, from_row, QModelIndex(), row)
        item = self.patch_data.pop(from_row)
        self.patch_data.insert(row, item)
        self.endMoveRows()

        self.update_priorities()
        return True

    def update_priorities(self) -> None:
        new_data: List[PatchDataType] = []
        for i, item in enumerate(self.patch_data):
            new_data.append((item[0], item[1], item[2], i + 1, item[4]))
        self.patch_data = new_data
        top_left = self.index(0, 0)
        bottom_right = self.index(self.rowCount() - 1, self.columnCount() - 1)
        self.dataChanged.emit(top_left, bottom_right)


class PatchSelector(QWidget):
    def __init__(self) -> None:
        super().__init__()
        self.setWindowTitle("パッチ選択 UI - 正しい行ドラッグ対応")
        self.resize(800, 400)

        layout = QVBoxLayout(self)

        self.table = QTableView()
        self.model = PatchTableModel(
            [
                (1, "FNIS Patch A", "https://example.com/fnis_a", 10, False),
                (2, "FNIS Patch B", "https://example.com/fnis_b", 20, False),
                (3, "FNIS Patch C", "https://example.com/fnis_c", 5, False),
            ]
        )

        self.table.setModel(self.model)

        # 行ドラッグ・ドロップ設定
        self.table.setDragDropMode(QAbstractItemView.DragDropMode.InternalMove)
        self.table.setDragEnabled(True)
        self.table.setAcceptDrops(True)
        self.table.setDropIndicatorShown(True)
        self.table.setDefaultDropAction(Qt.DropAction.MoveAction)

        self.table.setSelectionBehavior(QAbstractItemView.SelectionBehavior.SelectRows)
        self.table.setEditTriggers(QAbstractItemView.EditTrigger.NoEditTriggers)
        self.table.horizontalHeader().setSectionResizeMode(
            QHeaderView.ResizeMode.Stretch
        )

        # ヘッダークリックで全選択・全解除トグル
        self.table.horizontalHeader().sectionClicked.connect(self.toggle_all_checkboxes)

        layout.addWidget(self.table)

        self.patchButton = QPushButton("選択されたパッチを実行")
        self.patchButton.clicked.connect(self.run_patch)
        layout.addWidget(self.patchButton)

    def toggle_all_checkboxes(self, section: int) -> None:
        if section != 0:
            return

        all_checked = all(
            self.model.patch_data[row][4] for row in range(self.model.rowCount())
        )
        new_checked = not all_checked
        for row in range(self.model.rowCount()):
            index = self.model.index(row, 0)
            self.model.setData(
                index,
                Qt.CheckState.Checked if new_checked else Qt.CheckState.Unchecked,
                Qt.ItemDataRole.CheckStateRole,
            )

    def run_patch(self) -> None:
        selected_mods = [
            self.model.patch_data[row][1]
            for row in range(self.model.rowCount())
            if self.model.patch_data[row][4]
        ]

        if selected_mods:
            QMessageBox.information(
                self,
                "パッチ実行",
                f"{len(selected_mods)} 件のパッチを実行します：\n\n"
                + "\n".join(selected_mods),
            )
        else:
            QMessageBox.information(self, "パッチ実行", "パッチが選択されていません。")


def main() -> None:
    try:
        app = QApplication(sys.argv)
        window = PatchSelector()
        window.show()
        sys.exit(app.exec())
    except Exception as e:
        print(f"❌ Error: {e}")


if __name__ == "__main__":
    main()
