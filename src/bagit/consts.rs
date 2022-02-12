use crate::bagit::bag::BagItVersion;

pub const BAGIT_1_0: BagItVersion = BagItVersion::new(1, 0);
pub const BAGIT_DEFAULT_VERSION: BagItVersion = BAGIT_1_0;

pub const UTF_8: &str = "UTF-8";

// Filenames
pub const BAGIT_TXT: &str = "bagit.txt";
pub const BAG_INFO_TXT: &str = "bag-info.txt";
pub const FETCH_TXT: &str = "fetch.txt";
pub const DATA: &str = "data";
pub const PAYLOAD_MANIFEST_PREFIX: &str = "manifest";
pub const TAG_MANIFEST_PREFIX: &str = "tagmanifest";

// bagit.txt tag labels
pub const LABEL_BAGIT_VERSION: &str = "BagIt-Version";
pub const LABEL_FILE_ENCODING: &str = "Tag-File-Character-Encoding";

// bag-info.txt reserved labels
pub const LABEL_BAGGING_DATE: &str = "Bagging-Date";
pub const LABEL_PAYLOAD_OXUM: &str = "Payload-Oxum";