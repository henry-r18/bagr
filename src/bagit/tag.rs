use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;
use std::slice::Iter;
use std::vec::IntoIter;

use log::{debug, info};
use snafu::ResultExt;

use crate::bagit::bag::BagItVersion;
use crate::bagit::consts::*;
use crate::bagit::error::*;
use crate::bagit::io::{is_space_or_tab, TagLineReader};
use crate::bagit::Error::*;

use serde::Deserialize;

#[derive(Debug)]
pub struct BagDeclaration {
    version: BagItVersion,
    encoding: String,
}

#[derive(Debug, Deserialize)]
pub struct BagInfo {
    tags: TagList,
}

#[derive(Debug, Deserialize)]
pub struct Tag {
    label: String,
    value: String,
}

#[derive(Debug, Deserialize)]
pub struct TagList {
    tags: Vec<Tag>,
}

/// Writes bagit.txt to the bag's base directory
pub fn write_bag_declaration<P: AsRef<Path>>(
    bag_declaration: &BagDeclaration,
    base_dir: P,
) -> Result<()> {
    write_tag_file(
        &bag_declaration.to_tags(),
        base_dir.as_ref().join(BAGIT_TXT),
    )
}

/// Writes bag-info.txt to the bag's base directory
pub fn write_bag_info<P: AsRef<Path>>(bag_info: &BagInfo, base_dir: P) -> Result<()> {
    write_tag_file(bag_info.as_ref(), base_dir.as_ref().join(BAG_INFO_TXT))
}

/// Reads a bag declaration out of the specified `base_dir`
pub fn read_bag_declaration<P: AsRef<Path>>(base_dir: P) -> Result<BagDeclaration> {
    let bagit_file = base_dir.as_ref().join(BAGIT_TXT);
    let tags = read_tag_file(&bagit_file)?;
    tags.try_into()
}

/// Reads bag info out of the specified `base_dir`
pub fn read_bag_info<P: AsRef<Path>>(base_dir: P) -> Result<BagInfo> {
    let bagit_file = base_dir.as_ref().join(BAG_INFO_TXT);
    let tags = read_tag_file(&bagit_file)?;
    Ok(tags.into())
}

impl BagDeclaration {
    pub fn new() -> Self {
        Self {
            version: BAGIT_DEFAULT_VERSION,
            encoding: UTF_8.into(),
        }
    }

    pub fn with_values<S: AsRef<str>>(version: BagItVersion, encoding: S) -> Result<Self> {
        let encoding = encoding.as_ref();

        if BAGIT_1_0 != version {
            return Err(UnsupportedVersion { version });
        }

        if UTF_8 != encoding {
            return Err(UnsupportedEncoding {
                encoding: encoding.into(),
            });
        }

        Ok(Self {
            version,
            encoding: encoding.into(),
        })
    }

    pub fn to_tags(&self) -> TagList {
        let mut tags = TagList::with_capacity(2);
        // Safe to unwrap because it's not possible to create this object with invalid values
        tags.add_tag(LABEL_BAGIT_VERSION, self.version.to_string())
            .unwrap();
        tags.add_tag(LABEL_FILE_ENCODING, &self.encoding).unwrap();
        tags
    }
}

impl Default for BagDeclaration {
    fn default() -> Self {
        Self::new()
    }
}

impl TryFrom<TagList> for BagDeclaration {
    type Error = Error;

    fn try_from(tags: TagList) -> std::result::Result<Self, Self::Error> {
        let version_tag = tags
            .get_tag(LABEL_BAGIT_VERSION)
            .ok_or_else(|| MissingTag {
                tag: LABEL_BAGIT_VERSION.to_string(),
            })?;
        let version = BagItVersion::try_from(&version_tag.value)?;

        let encoding_tag = tags
            .get_tag(LABEL_FILE_ENCODING)
            .ok_or_else(|| MissingTag {
                tag: LABEL_FILE_ENCODING.to_string(),
            })?;
        let encoding = &encoding_tag.value;

        BagDeclaration::with_values(version, encoding)
    }
}

impl BagInfo {
    pub fn new() -> Self {
        Self {
            tags: TagList::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            tags: TagList::with_capacity(capacity),
        }
    }

    pub fn with_tags(tags: TagList) -> Self {
        Self { tags }
    }

    pub fn add_tag<L: AsRef<str>, S: AsRef<str>>(&mut self, label: L, value: S) -> Result<()> {
        let label = label.as_ref();

        let repeatable = LABEL_REPEATABLE
            .iter()
            .find(|(reserved_label, _)| reserved_label.eq_ignore_ascii_case(label))
            .map(|(_, repeatable)| *repeatable)
            .unwrap_or(true);

        if repeatable {
            self.add_repeatable(label, value)
        } else {
            self.add_non_repeatable(label, value)
        }
    }

    /// Returns the first tag that's found that matches the specified label.
    /// Labels are case insensitive.
    pub fn get_tag<L: AsRef<str>>(&self, label: L) -> Option<&Tag> {
        self.tags.get_tag(label.as_ref())
    }

    /// Returns all of the tags that match the specified label. Labels are case insensitive.
    pub fn get_tags<'a, 'b: 'a>(&'a self, label: &'b str) -> Box<dyn Iterator<Item = &Tag> + 'a> {
        self.tags.get_tags(label.as_ref())
    }

    pub fn add_bagging_date<S: AsRef<str>>(&mut self, value: S) -> Result<()> {
        self.add_non_repeatable(LABEL_BAGGING_DATE, value)
    }

    pub fn bagging_date(&self) -> Option<&Tag> {
        self.get_tag(LABEL_BAGGING_DATE)
    }

    pub fn add_payload_oxum<S: AsRef<str>>(&mut self, value: S) -> Result<()> {
        self.add_non_repeatable(LABEL_PAYLOAD_OXUM, value)
    }

    pub fn payload_oxum(&self) -> Option<&Tag> {
        self.get_tag(LABEL_PAYLOAD_OXUM)
    }

    pub fn add_software_agent<S: AsRef<str>>(&mut self, value: S) -> Result<()> {
        self.add_non_repeatable(LABEL_SOFTWARE_AGENT, value)
    }

    pub fn software_agent(&self) -> Option<&Tag> {
        self.get_tag(LABEL_SOFTWARE_AGENT)
    }

    pub fn add_source_organization<S: AsRef<str>>(&mut self, value: S) -> Result<()> {
        self.add_repeatable(LABEL_SOURCE_ORGANIZATION, value)
    }

    pub fn source_organization(&self) -> Box<dyn Iterator<Item = &Tag> + '_> {
        self.get_tags(LABEL_SOURCE_ORGANIZATION)
    }

    pub fn add_organization_address<S: AsRef<str>>(&mut self, value: S) -> Result<()> {
        self.add_repeatable(LABEL_ORGANIZATION_ADDRESS, value)
    }

    pub fn organization_address(&self) -> Box<dyn Iterator<Item = &Tag> + '_> {
        self.get_tags(LABEL_ORGANIZATION_ADDRESS)
    }

    pub fn add_contact_name<S: AsRef<str>>(&mut self, value: S) -> Result<()> {
        self.add_repeatable(LABEL_CONTACT_NAME, value)
    }

    pub fn contact_name(&self) -> Box<dyn Iterator<Item = &Tag> + '_> {
        self.get_tags(LABEL_CONTACT_NAME)
    }

    pub fn add_contact_phone<S: AsRef<str>>(&mut self, value: S) -> Result<()> {
        self.add_repeatable(LABEL_CONTACT_PHONE, value)
    }

    pub fn contact_phone(&self) -> Box<dyn Iterator<Item = &Tag> + '_> {
        self.get_tags(LABEL_CONTACT_PHONE)
    }

    pub fn add_contact_email<S: AsRef<str>>(&mut self, value: S) -> Result<()> {
        self.add_repeatable(LABEL_CONTACT_EMAIL, value)
    }

    pub fn contact_email(&self) -> Box<dyn Iterator<Item = &Tag> + '_> {
        self.get_tags(LABEL_CONTACT_EMAIL)
    }

    pub fn add_external_description<S: AsRef<str>>(&mut self, value: S) -> Result<()> {
        self.add_repeatable(LABEL_EXTERNAL_DESCRIPTION, value)
    }

    pub fn external_description(&self) -> Box<dyn Iterator<Item = &Tag> + '_> {
        self.get_tags(LABEL_EXTERNAL_DESCRIPTION)
    }

    pub fn add_external_identifier<S: AsRef<str>>(&mut self, value: S) -> Result<()> {
        self.add_repeatable(LABEL_EXTERNAL_IDENTIFIER, value)
    }

    pub fn external_identifier(&self) -> Box<dyn Iterator<Item = &Tag> + '_> {
        self.get_tags(LABEL_EXTERNAL_IDENTIFIER)
    }

    pub fn add_bag_size<S: AsRef<str>>(&mut self, value: S) -> Result<()> {
        self.add_non_repeatable(LABEL_BAG_SIZE, value)
    }

    pub fn bag_size(&self) -> Option<&Tag> {
        self.get_tag(LABEL_BAG_SIZE)
    }

    pub fn add_bag_group_identifier<S: AsRef<str>>(&mut self, value: S) -> Result<()> {
        self.add_non_repeatable(LABEL_BAG_GROUP_IDENTIFIER, value)
    }

    pub fn bag_group_identifier(&self) -> Option<&Tag> {
        self.get_tag(LABEL_BAG_GROUP_IDENTIFIER)
    }

    pub fn add_bag_count<S: AsRef<str>>(&mut self, value: S) -> Result<()> {
        self.add_non_repeatable(LABEL_BAG_COUNT, value)
    }

    pub fn bag_count(&self) -> Option<&Tag> {
        self.get_tag(LABEL_BAG_COUNT)
    }

    pub fn add_internal_sender_identifier<S: AsRef<str>>(&mut self, value: S) -> Result<()> {
        self.add_repeatable(LABEL_INTERNAL_SENDER_IDENTIFIER, value)
    }

    pub fn internal_sender_identifier(&self) -> Box<dyn Iterator<Item = &Tag> + '_> {
        self.get_tags(LABEL_INTERNAL_SENDER_IDENTIFIER)
    }

    pub fn add_internal_sender_description<S: AsRef<str>>(&mut self, value: S) -> Result<()> {
        self.add_repeatable(LABEL_INTERNAL_SENDER_DESCRIPTION, value)
    }

    pub fn internal_sender_description(&self) -> Box<dyn Iterator<Item = &Tag> + '_> {
        self.get_tags(LABEL_INTERNAL_SENDER_DESCRIPTION)
    }

    pub fn add_bagit_profile_identifier<S: AsRef<str>>(&mut self, value: S) -> Result<()> {
        self.add_repeatable(LABEL_BAGIT_PROFILE_IDENTIFIER, value)
    }

    pub fn bagit_profile_identifier(&self) -> Box<dyn Iterator<Item = &Tag> + '_> {
        self.get_tags(LABEL_BAGIT_PROFILE_IDENTIFIER)
    }

    /// Adds a new tag by first removing all existing tags with the same label.
    fn add_non_repeatable<L: AsRef<str>, S: AsRef<str>>(
        &mut self,
        label: L,
        value: S,
    ) -> Result<()> {
        let label = label.as_ref();
        self.tags.remove_tags(label);
        self.tags.add_tag(label, value)
    }

    /// Adds a new tag but does not remove any existing tags with the same label
    fn add_repeatable<L: AsRef<str>, S: AsRef<str>>(&mut self, label: L, value: S) -> Result<()> {
        let label = label.as_ref();
        self.tags.add_tag(label, value)
    }
}

impl Default for BagInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl From<TagList> for BagInfo {
    fn from(tags: TagList) -> Self {
        BagInfo::with_tags(tags)
    }
}

impl From<BagInfo> for TagList {
    fn from(info: BagInfo) -> Self {
        info.tags
    }
}

impl AsRef<TagList> for BagInfo {
    fn as_ref(&self) -> &TagList {
        &self.tags
    }
}

impl Tag {
    /// Creates a tag and validates that their parts are valid
    pub fn new<L: AsRef<str>, V: AsRef<str>>(label: L, value: V) -> Result<Self> {
        let label = label.as_ref();
        let value = value.as_ref();

        Tag::validate_label(label)?;
        Tag::validate_value(label, value)?;

        Ok(Self {
            label: label.into(),
            value: value.into(),
        })
    }

    fn validate_label(label: &str) -> Result<()> {
        if label.starts_with(is_space_or_tab) || label.ends_with(is_space_or_tab) {
            return Err(InvalidTag {
                label: label.into(),
                details: "Label must not start or end with whitespace".into(),
            });
        } else if label.contains(|c: char| c == CR || c == LF) {
            return Err(InvalidTag {
                label: label.into(),
                details: "Label must not contain CR or LF characters".into(),
            });
        }

        Ok(())
    }

    fn validate_value(label: &str, value: &str) -> Result<()> {
        // CR/LF will only appear in a value when serialized
        if value.contains(|c: char| c == CR || c == LF) {
            return Err(InvalidTag {
                label: label.into(),
                details: "Value must not contain CR or LF characters".into(),
            });
        }

        Ok(())
    }
}

impl TagList {
    pub fn new() -> Self {
        Self { tags: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            tags: Vec::with_capacity(capacity),
        }
    }

    /// Returns all of the tags with the provided label. It uses a case insensitive match.
    pub fn get_tags<'a, 'b: 'a>(&'a self, label: &'b str) -> Box<dyn Iterator<Item = &Tag> + 'a> {
        Box::new(
            self.tags
                .iter()
                .filter(|tag| tag.label.eq_ignore_ascii_case(label)),
        )
    }

    /// Returns the first tag with the provided label. It uses a case insensitive match.
    pub fn get_tag<S: AsRef<str>>(&self, label: S) -> Option<&Tag> {
        let label = label.as_ref();
        self.tags
            .iter()
            .find(|tag| tag.label.eq_ignore_ascii_case(label))
    }

    pub fn add(&mut self, tag: Tag) {
        self.tags.push(tag);
    }

    pub fn add_tag<L: AsRef<str>, V: AsRef<str>>(&mut self, label: L, value: V) -> Result<()> {
        self.add(Tag::new(label, value)?);
        Ok(())
    }

    /// Removes all of the tags with the provided label. It uses a case insensitive match.
    pub fn remove_tags<S: AsRef<str>>(&mut self, label: S) {
        let label = label.as_ref();
        self.tags.retain(|e| !e.label.eq_ignore_ascii_case(label));
    }
}

impl Default for TagList {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for TagList {
    type Item = Tag;
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.tags.into_iter()
    }
}

impl<'a> IntoIterator for &'a TagList {
    type Item = &'a Tag;
    type IntoIter = Iter<'a, Tag>;

    fn into_iter(self) -> Self::IntoIter {
        self.tags.iter()
    }
}

/// Writes a tag file to the specified destination
fn write_tag_file<P: AsRef<Path>>(tags: &TagList, destination: P) -> Result<()> {
    let destination = destination.as_ref();
    info!("Writing tag file {}", destination.display());

    let mut writer =
        BufWriter::new(File::create(destination).context(IoCreateSnafu { path: destination })?);

    for tag in tags {
        // TODO handle multi-line tags
        writeln!(writer, "{}: {}", tag.label, tag.value)
            .context(IoWriteSnafu { path: destination })?;
    }

    Ok(())
}

fn read_tag_file<P: AsRef<Path>>(path: P) -> Result<TagList> {
    let path = path.as_ref();
    let reader = TagLineReader::new(BufReader::new(
        File::open(path).context(IoReadSnafu { path })?,
    ));

    let mut tags = TagList::new();
    let mut tag_num: u32 = 0;

    // TODO this only works for UTF-8
    // https://crates.io/crates/encoding_rs
    // https://crates.io/crates/encoding_rs_io
    // TODO how should empty lines be handled?
    for line in reader {
        let line = line?;
        tag_num += 1;

        match parse_tag_line(&line) {
            Ok(tag) => tags.add(tag),
            Err(InvalidTag { details, label: _ }) => {
                return Err(InvalidTagLineWithRef {
                    details,
                    path: path.into(),
                    num: tag_num,
                })
            }
            Err(InvalidTagLine { details }) => {
                return Err(InvalidTagLineWithRef {
                    details,
                    path: path.into(),
                    num: tag_num,
                })
            }
            Err(e) => {
                return Err(InvalidTagLineWithRef {
                    details: e.to_string(),
                    path: path.into(),
                    num: tag_num,
                })
            }
        }
    }

    Ok(tags)
}

fn parse_tag_line<S: AsRef<str>>(line: S) -> Result<Tag> {
    let line = line.as_ref();

    if let Some((label, value)) = line.split_once(':') {
        debug!("Tag [`{label}`:`{value}`]");

        if !value.starts_with(is_space_or_tab) {
            Err(InvalidTagLine {
                details: "Value part must start with one whitespace character".to_string(),
            })
        } else {
            let trim_value = &value[1..];
            debug!("Tag [`{label}`:`{trim_value}`]");
            Tag::new(label, trim_value)
        }
    } else {
        Err(InvalidTagLine {
            details: "Missing colon separating the label and value".to_string(),
        })
    }
}
