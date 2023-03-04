use crate::huds::archive_location::ArchiveLocation;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hud {
    /// The URL to download the HUD's archive.
    /// It's also the unique identifier of a HUD.
    pub archive_location: ArchiveLocation,
    /// The name of the HUD seen by the user.
    pub display_name: String,
}
