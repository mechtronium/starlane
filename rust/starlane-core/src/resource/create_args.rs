use std::fs::File;
use std::io::{Read, Write};

use std::str::FromStr;

use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::error::Error;
use crate::resource::{ResourceAddress, ResourceType, Kind};
use std::convert::TryInto;
use starlane_resources::ResourcePath;

lazy_static! {
    pub static ref ARTIFACT_BUNDLE: ResourcePath = artifact_bundle_address();
    pub static ref SPACE: &'static str = r#"
name: Space
args:
    - display-name:
        about: Takes a human friendly display name
        required: true
   "#;
    pub static ref SUB_SPACE: &'static str = r#"
name: SubSpace
args:
    - display-name:
        about: Takes a human friendly display name
        required: true
   "#;
}

pub fn artifact_bundle_address() -> ResourcePath {
    let address = format!("space:core:{}", crate::VERSION.to_string());
    ResourcePath::from_str(address.as_str()).expect(
        format!(
            "FATAL: expected artifact_bundle_address '{}' to be parse-able",
            address
        )
        .as_str(),
    )
}

pub fn space_address() -> Result<ResourcePath,Error> {
   let path  = artifact_bundle_address();
   let address = path.append("/create-args/space.yaml" )?;
   Ok(address)
}


pub fn create_args_artifact_bundle() -> Result<Vec<u8>, Error> {
    let zipfile = tempfile::NamedTempFile::new()?;
    let mut zip = ZipWriter::new(zipfile.reopen()?);

    write_file_to_zip(&mut zip, "create-args/space.yaml", &SPACE)?;
    write_file_to_zip(&mut zip, "create-args/sub_space.yaml", &SUB_SPACE)?;

    zip.finish()?;

    let mut file = zipfile.reopen()?;

    let mut data = Vec::with_capacity(file.metadata()?.len() as _);
    file.read_to_end(&mut data)?;

    Ok(data)
}

fn write_file_to_zip(zip: &mut ZipWriter<File>, filename: &str, data: &str) -> Result<(), Error> {
    let file_options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o755);

    zip.start_file(filename, file_options)?;
    zip.write_all(data.as_bytes())?;

    Ok(())
}
