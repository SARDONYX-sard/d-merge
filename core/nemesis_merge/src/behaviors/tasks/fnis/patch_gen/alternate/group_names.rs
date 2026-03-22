//! Group name enum

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
/// FNIS alternate-animation group identifier.
///
/// Each variant represents one animation group recognised by FNIS and maps to
/// the integer constant used in `FNIS_aa.<group>()` Papyrus calls.
///
/// | Variant        | JSON / FNIS name | `group_id` |
/// |----------------|------------------|------------|
/// | `MtIdle`       | `_mtidle`        | 0          |
/// | `OneHmIdle`    | `_1hmidle`       | 1          |
/// | `TwoHmIdle`    | `_2hmidle`       | 2          |
/// | `TwoHwIdle`    | `_2hwidle`       | 3          |
/// | `BowIdle`      | `_bowidle`       | 4          |
/// | `CBowIdle`     | `_cbowidle`      | 5          |
/// | `H2HIdle`      | `_h2hidle`       | 6          |
/// | `MagIdle`      | `_magidle`       | 7          |
/// | `SneakIdle`    | `_sneakidle`     | 8          |
/// | `StaffIdle`    | `_staffidle`     | 9          |
/// | `MagMt`        | `_magmt`         | 16         |
/// | `MagCastMt`    | `_magcastmt`     | 17         |
/// | `OneHmAtk`     | `_1hmatk`        | 19         |
/// | `H2HAtk`       | `_h2hatk`        | 33         |
/// | `H2HAtkPow`    | `_h2hatkpow`     | 34         |
/// | `MagAtk`       | `_magatk`        | 36         |
/// | `OneHmEqp`     | `_1hmeqp`        | 37         |
/// | `TwoHwEqp`     | `_2hweqp`        | 38         |
/// | `TwoHmEqp`     | `_2hmeqp`        | 39         |
/// | `AxeEqp`       | `_axeeqp`        | 40         |
/// | `BowEqp`       | `_boweqp`        | 41         |
/// | `CBowEqp`      | `_cboweqp`       | 42         |
/// | `DagEqp`       | `_dageqp`        | 43         |
/// | `H2HEqp`       | `_h2heqp`        | 44         |
/// | `MaceEqp`      | `_maceqp`        | 45         |
/// | `MagEqp`       | `_mageqp`        | 46         |
/// | `StfEqp`       | `_stfeqp`        | 47         |
/// | `Shout`        | `_shout`         | 48         |
/// | `MagCon`       | `_magcon`        | 49         |
/// | `DW`           | `_dw`            | 50         |
/// | `Jump`         | `_jump`          | 51         |
/// | `Sprint`       | `_sprint`        | 52         |
/// | `Shield`       | `_shield`        | 53         |
pub enum AAGroupName {
    #[serde(rename = "_mtidle")]
    MtIdle,
    #[serde(rename = "_1hmidle")]
    OneHmIdle,
    #[serde(rename = "_2hmidle")]
    TwoHmIdle,
    #[serde(rename = "_2hwidle")]
    TwoHwIdle,
    #[serde(rename = "_bowidle")]
    BowIdle,
    #[serde(rename = "_cbowidle")]
    CBowIdle,
    #[serde(rename = "_h2hidle")]
    H2HIdle,
    #[serde(rename = "_magidle")]
    MagIdle,
    #[serde(rename = "_sneakidle")]
    SneakIdle,
    #[serde(rename = "_staffidle")]
    StaffIdle,
    #[serde(rename = "_magmt")]
    MagMt,
    #[serde(rename = "_magcastmt")]
    MagCastMt,
    #[serde(rename = "_1hmatk")]
    OneHmAtk,
    #[serde(rename = "_h2hatk")]
    H2HAtk,
    #[serde(rename = "_h2hatkpow")]
    H2HAtkPow,
    #[serde(rename = "_magatk")]
    MagAtk,
    #[serde(rename = "_1hmeqp")]
    OneHmEqp,
    #[serde(rename = "_2hweqp")]
    TwoHwEqp,
    #[serde(rename = "_2hmeqp")]
    TwoHmEqp,
    #[serde(rename = "_axeeqp")]
    AxeEqp,
    #[serde(rename = "_boweqp")]
    BowEqp,
    #[serde(rename = "_cboweqp")]
    CBowEqp,
    #[serde(rename = "_dageqp")]
    DagEqp,
    #[serde(rename = "_h2heqp")]
    H2HEqp,
    #[serde(rename = "_maceqp")]
    MaceEqp,
    #[serde(rename = "_mageqp")]
    MagEqp,
    #[serde(rename = "_stfeqp")]
    StfEqp,
    #[serde(rename = "_shout")]
    Shout,
    #[serde(rename = "_magcon")]
    MagCon,
    #[serde(rename = "_dw")]
    DW,
    #[serde(rename = "_jump")]
    Jump,
    #[serde(rename = "_sprint")]
    Sprint,
    #[serde(rename = "_shield")]
    Shield,
}

impl AAGroupName {
    /// Returns the integer constant matching `FNIS_aa.<group>()` in Papyrus.
    #[inline]
    pub const fn group_id(self) -> u64 {
        match self {
            Self::MtIdle => 0,
            Self::OneHmIdle => 1,
            Self::TwoHmIdle => 2,
            Self::TwoHwIdle => 3,
            Self::BowIdle => 4,
            Self::CBowIdle => 5,
            Self::H2HIdle => 6,
            Self::MagIdle => 7,
            Self::SneakIdle => 8,
            Self::StaffIdle => 9,
            Self::MagMt => 16,
            Self::MagCastMt => 17,
            Self::OneHmAtk => 19,
            Self::H2HAtk => 33,
            Self::H2HAtkPow => 34,
            Self::MagAtk => 36,
            Self::OneHmEqp => 37,
            Self::TwoHwEqp => 38,
            Self::TwoHmEqp => 39,
            Self::AxeEqp => 40,
            Self::BowEqp => 41,
            Self::CBowEqp => 42,
            Self::DagEqp => 43,
            Self::H2HEqp => 44,
            Self::MaceEqp => 45,
            Self::MagEqp => 46,
            Self::StfEqp => 47,
            Self::Shout => 48,
            Self::MagCon => 49,
            Self::DW => 50,
            Self::Jump => 51,
            Self::Sprint => 52,
            Self::Shield => 53,
        }
    }

    /// Returns the FNIS wire string for this group, e.g. `"_1hmeqp"`.
    #[inline]
    pub const fn as_fnis_str(self) -> &'static str {
        match self {
            Self::MtIdle => "_mtidle",
            Self::OneHmIdle => "_1hmidle",
            Self::TwoHmIdle => "_2hmidle",
            Self::TwoHwIdle => "_2hwidle",
            Self::BowIdle => "_bowidle",
            Self::CBowIdle => "_cbowidle",
            Self::H2HIdle => "_h2hidle",
            Self::MagIdle => "_magidle",
            Self::SneakIdle => "_sneakidle",
            Self::StaffIdle => "_staffidle",
            Self::MagMt => "_magmt",
            Self::MagCastMt => "_magcastmt",
            Self::OneHmAtk => "_1hmatk",
            Self::H2HAtk => "_h2hatk",
            Self::H2HAtkPow => "_h2hatkpow",
            Self::MagAtk => "_magatk",
            Self::OneHmEqp => "_1hmeqp",
            Self::TwoHwEqp => "_2hweqp",
            Self::TwoHmEqp => "_2hmeqp",
            Self::AxeEqp => "_axeeqp",
            Self::BowEqp => "_boweqp",
            Self::CBowEqp => "_cboweqp",
            Self::DagEqp => "_dageqp",
            Self::H2HEqp => "_h2heqp",
            Self::MaceEqp => "_maceqp",
            Self::MagEqp => "_mageqp",
            Self::StfEqp => "_stfeqp",
            Self::Shout => "_shout",
            Self::MagCon => "_magcon",
            Self::DW => "_dw",
            Self::Jump => "_jump",
            Self::Sprint => "_sprint",
            Self::Shield => "_shield",
        }
    }
}

impl std::fmt::Display for AAGroupName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_fnis_str())
    }
}

/// Error returned when a string does not match any known FNIS group name.
#[derive(Debug, snafu::Snafu)]
#[snafu(display("unknown FNIS group name: `{message}`"))]
pub struct UnknownGroupName {
    pub message: String,
}

impl std::str::FromStr for AAGroupName {
    type Err = UnknownGroupName;

    /// Parses the FNIS group name string into an [`AAGroupName`], case-insensitively.
    #[allow(clippy::cognitive_complexity)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            s if s.eq_ignore_ascii_case("_mtidle") => Ok(Self::MtIdle),
            s if s.eq_ignore_ascii_case("_1hmidle") => Ok(Self::OneHmIdle),
            s if s.eq_ignore_ascii_case("_2hmidle") => Ok(Self::TwoHmIdle),
            s if s.eq_ignore_ascii_case("_2hwidle") => Ok(Self::TwoHwIdle),
            s if s.eq_ignore_ascii_case("_bowidle") => Ok(Self::BowIdle),
            s if s.eq_ignore_ascii_case("_cbowidle") => Ok(Self::CBowIdle),
            s if s.eq_ignore_ascii_case("_h2hidle") => Ok(Self::H2HIdle),
            s if s.eq_ignore_ascii_case("_magidle") => Ok(Self::MagIdle),
            s if s.eq_ignore_ascii_case("_sneakidle") => Ok(Self::SneakIdle),
            s if s.eq_ignore_ascii_case("_staffidle") => Ok(Self::StaffIdle),
            s if s.eq_ignore_ascii_case("_magmt") => Ok(Self::MagMt),
            s if s.eq_ignore_ascii_case("_magcastmt") => Ok(Self::MagCastMt),
            s if s.eq_ignore_ascii_case("_1hmatk") => Ok(Self::OneHmAtk),
            s if s.eq_ignore_ascii_case("_h2hatk") => Ok(Self::H2HAtk),
            s if s.eq_ignore_ascii_case("_h2hatkpow") => Ok(Self::H2HAtkPow),
            s if s.eq_ignore_ascii_case("_magatk") => Ok(Self::MagAtk),
            s if s.eq_ignore_ascii_case("_1hmeqp") => Ok(Self::OneHmEqp),
            s if s.eq_ignore_ascii_case("_2hweqp") => Ok(Self::TwoHwEqp),
            s if s.eq_ignore_ascii_case("_2hmeqp") => Ok(Self::TwoHmEqp),
            s if s.eq_ignore_ascii_case("_axeeqp") => Ok(Self::AxeEqp),
            s if s.eq_ignore_ascii_case("_boweqp") => Ok(Self::BowEqp),
            s if s.eq_ignore_ascii_case("_cboweqp") => Ok(Self::CBowEqp),
            s if s.eq_ignore_ascii_case("_dageqp") => Ok(Self::DagEqp),
            s if s.eq_ignore_ascii_case("_h2heqp") => Ok(Self::H2HEqp),
            s if s.eq_ignore_ascii_case("_maceqp") => Ok(Self::MaceEqp),
            s if s.eq_ignore_ascii_case("_mageqp") => Ok(Self::MagEqp),
            s if s.eq_ignore_ascii_case("_stfeqp") => Ok(Self::StfEqp),
            s if s.eq_ignore_ascii_case("_shout") => Ok(Self::Shout),
            s if s.eq_ignore_ascii_case("_magcon") => Ok(Self::MagCon),
            s if s.eq_ignore_ascii_case("_dw") => Ok(Self::DW),
            s if s.eq_ignore_ascii_case("_jump") => Ok(Self::Jump),
            s if s.eq_ignore_ascii_case("_sprint") => Ok(Self::Sprint),
            s if s.eq_ignore_ascii_case("_shield") => Ok(Self::Shield),
            _ => Err(UnknownGroupName {
                message: s.to_owned(),
            }),
        }
    }
}
