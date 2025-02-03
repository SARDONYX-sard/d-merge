# マージ課題

## PathとRangeの分離

- Hashmap作成が難しい
  - 配列内にRangeが複数入る
-

- Hashmap (key) -> 同一のパッチPathを取得できる

### Pathの仕様が問題

- Pathは配列のRangeも含めて取得、最後のRangeのみRange型として扱う
- パッチ作成の仕様自体を再検討
　　- 並列化
- Json Patch からPathを削除（Hashmapのキーにあるため）

1. 優先順位に並べる
2. グループ単位で競合解決（並列化）
3. グループで勝利したパッチで競合解決（逐次処理）
