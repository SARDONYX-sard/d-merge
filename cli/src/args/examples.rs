// Examples constants
pub(crate) const PATCH_EXAMPLES: &str = "\
Examples:
  # Nemesis only
  d_merge_cli patch --nemesis-ini ids.ini

  # FNIS only (skyrim-data-dir-glob is required)
  d_merge_cli patch --fnis-ini fnis.ini --skyrim-data-dir-glob \"D:/GAME/ModOrganizer Skyrim SE/mods/*\"

  # Both
  d_merge_cli patch --nemesis-ini nemesis_ids.ini --fnis-ini fnis.ini --skyrim-data-dir-glob \"D:/STEAM/steamapps/common/Skyrim Special Edition/Data\"

  # With debug output and log file
  ./d_merge_cli patch --nemesis-ini ./nemesis_ids.ini --fnis-ini ./fnis.ini --skyrim-data-dir-glob \"D:/GAME/ModOrganizer Skyrim SE/mods/*\" \
    --debug --log-level debug --log-file ./logs/d_merge.log

ini sample

  # nemesis_ids.ini

  ```
  ; Nemesis mod IDs — order determines patch priority (top = highest)
  D:\\STEAM\\steamapps\\common\\Skyrim Special Edition\\Data\\Nemesis_Engine\\mod\\slide
  D:\\STEAM\\steamapps\\common\\Skyrim Special Edition\\Data\\Nemesis_Engine\\mod\\dmco
  D:\\STEAM\\steamapps\\common\\Skyrim Special Edition\\Data\\Nemesis_Engine\\mod\\para
  D:\\STEAM\\steamapps\\common\\Skyrim Special Edition\\Data\\Nemesis_Engine\\mod\\scar
  ```

  ### fnis_ids.ini

  ```
  ; FNIS mod IDs (namespaces)
  FNISBase
  FNISCreatureVersion
  FNISZoo
  ```
";

pub(crate) const SKYRIM_DIR_EXAMPLES: &str = "\
Examples:
  d_merge_cli info skyrim-dir --runtime SkyrimSE
  d_merge_cli info skyrim-dir --runtime SkyrimLE
";

pub(crate) const MODS_EXAMPLES: &str = "\
Examples:
  # Print to stdout( Meaning of \"glob\": Treat everything directly under the \"mods\" folder as the Skyrim Data directory and search through its entire contents.)
  d_merge_cli info mods --glob \"D:MO2/mods/*\"

  # MO2 VFS
  d_merge_cli info mods --glob \"D:/STEAM/steamapps/common/Skyrim Special Edition/Data\" --vfs

  # Write to file
  d_merge_cli info mods --glob \"D:/MO2/mods/*\" --output mods.json

  # VFS + file
  d_merge_cli info mods --glob \"D:/STEAM/steamapps/common/Skyrim Special Edition/Data\" --vfs --output mods.json
";
